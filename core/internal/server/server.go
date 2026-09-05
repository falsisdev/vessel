package server

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"net"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/cinema"
	"github.com/falsisdev/vessel/core/internal/domain/reading"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	corev1 "github.com/falsisdev/vessel/proto/gen/go/core/v1"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

type ServerConfig struct {
	ListenAddr string
	Version    string
}

type Server struct {
	corev1.UnimplementedCoreServiceServer

	cfg            ServerConfig
	listener       net.Listener
	grpcServer     *grpc.Server
	cinemaService  *service.CinemaService
	readingService *service.ReadingService
	pluginManager  *plugin.Manager
	startTime      time.Time
	isUnixSocket   bool
	socketPath     string
}

func NewServer(cfg ServerConfig, cinemaSvc *service.CinemaService, readingSvc *service.ReadingService, pluginMgr *plugin.Manager) *Server {
	if cfg.Version == "" {
		cfg.Version = "1.0.0"
	}
	return &Server{
		cfg:            cfg,
		cinemaService:  cinemaSvc,
		readingService: readingSvc,
		pluginManager:  pluginMgr,
		startTime:      time.Now(),
	}
}

func (s *Server) Start() error {
	network, address, err := parseListenAddr(s.cfg.ListenAddr)
	if err != nil {
		return err
	}

	if network == "unix" {
		s.isUnixSocket = true
		s.socketPath = address
		_ = os.Remove(address)
	}

	listener, err := net.Listen(network, address)
	if err != nil {
		return fmt.Errorf("failed to listen on %s (%s): %w", address, network, err)
	}
	s.listener = listener

	s.grpcServer = grpc.NewServer()
	corev1.RegisterCoreServiceServer(s.grpcServer, s)

	slog.Info("Core IPC Server listening", "network", network, "address", address)

	go func() {
		if err := s.grpcServer.Serve(listener); err != nil && !errors.Is(err, grpc.ErrServerStopped) {
			slog.Error("Core IPC Server error", "error", err)
		}
	}()

	return nil
}

func (s *Server) Addr() net.Addr {
	if s.listener != nil {
		return s.listener.Addr()
	}
	return nil
}

func (s *Server) Stop() {
	if s.grpcServer != nil {
		s.grpcServer.GracefulStop()
	}
	if s.isUnixSocket && s.socketPath != "" {
		_ = os.Remove(s.socketPath)
	}
}

func (s *Server) Ping(ctx context.Context, req *corev1.PingRequest) (*corev1.PingResponse, error) {
	return &corev1.PingResponse{
		Status:        "OK",
		Version:       s.cfg.Version,
		UptimeSeconds: int64(time.Since(s.startTime).Seconds()),
	}, nil
}

func (s *Server) SearchMedia(ctx context.Context, req *corev1.SearchMediaRequest) (*corev1.SearchMediaResponse, error) {
	if req.Domain == pluginv1.Domain_DOMAIN_CINEMA || req.Domain == pluginv1.Domain_DOMAIN_UNSPECIFIED {
		if s.cinemaService != nil {
			items, err := s.cinemaService.Search(ctx, req.Query)
			if err != nil {
				return nil, fmt.Errorf("cinema search failed: %w", err)
			}

			var coreItems []*corev1.CoreMediaItem
			for _, item := range items {
				coreItems = append(coreItems, &corev1.CoreMediaItem{
					Id:         item.ID,
					ProviderId: item.ProviderID,
					Title:      item.Title,
					Type:       mapCinemaTypeToProto(item.Type),
					Year:       item.Year,
					PosterUrl:  item.PosterURL,
					Overview:   item.Overview,
					ExternalIds: &pluginv1.ExternalIDs{
						ImdbId:    item.ExternalIDs.IMDbID,
						TmdbId:    item.ExternalIDs.TMDBID,
						SimklId:   item.ExternalIDs.SIMKLID,
						MalId:     item.ExternalIDs.MALID,
						AnilistId: item.ExternalIDs.AniListID,
						KitsuId:   item.ExternalIDs.KitsuID,
						Extra:     item.ExternalIDs.Extra,
						SanityId:  item.ExternalIDs.SanityID,
					},
				})
			}
			return &corev1.SearchMediaResponse{Items: coreItems}, nil
		}
	}

	if req.Domain == pluginv1.Domain_DOMAIN_MANGA || req.Domain == pluginv1.Domain_DOMAIN_WEBTOON || req.Domain == pluginv1.Domain_DOMAIN_WEBOOK {
		if s.readingService != nil {
			items, err := s.readingService.Search(ctx, req.Domain, req.Query)
			if err != nil {
				return nil, fmt.Errorf("reading search failed: %w", err)
			}

			var coreItems []*corev1.CoreMediaItem
			for _, item := range items {
				coreItems = append(coreItems, &corev1.CoreMediaItem{
					Id:         item.ID,
					ProviderId: item.ProviderID,
					Title:      item.Title,
					Type:       mapReadingTypeToProto(item.Type),
					Year:       item.Year,
					PosterUrl:  item.PosterURL,
					Overview:   item.Overview,
					ExternalIds: &pluginv1.ExternalIDs{
						ImdbId:    item.ExternalIDs.IMDbID,
						TmdbId:    item.ExternalIDs.TMDBID,
						SimklId:   item.ExternalIDs.SIMKLID,
						MalId:     item.ExternalIDs.MALID,
						AnilistId: item.ExternalIDs.AniListID,
						KitsuId:   item.ExternalIDs.KitsuID,
						Extra:     item.ExternalIDs.Extra,
						SanityId:  item.ExternalIDs.SanityID,
					},
				})
			}
			return &corev1.SearchMediaResponse{Items: coreItems}, nil
		}
	}

	return &corev1.SearchMediaResponse{}, nil
}

func (s *Server) GetMediaDetails(ctx context.Context, req *corev1.GetMediaDetailsRequest) (*corev1.GetMediaDetailsResponse, error) {
	if req.Domain == pluginv1.Domain_DOMAIN_MANGA || req.Domain == pluginv1.Domain_DOMAIN_WEBTOON || req.Domain == pluginv1.Domain_DOMAIN_WEBOOK {
		if s.readingService != nil {
			details, err := s.readingService.GetMetadata(ctx, req.ProviderId, req.MediaId)
			if err != nil {
				return nil, fmt.Errorf("get reading details failed: %w", err)
			}

			var protoSeasons []*pluginv1.Season
			var episodes []*pluginv1.Episode
			for _, ch := range details.Chapters {
				episodes = append(episodes, &pluginv1.Episode{
					EpisodeNumber: int32(ch.ChapterNumber),
					Title:         ch.Title,
				})
			}
			protoSeasons = append(protoSeasons, &pluginv1.Season{
				SeasonNumber: 1,
				Title:        "Chapters",
				Episodes:     episodes,
			})

			return &corev1.GetMediaDetailsResponse{
				Id:         details.ID,
				ProviderId: details.ProviderID,
				Title:      details.Title,
				Type:       mapReadingTypeToProto(details.Type),
				Year:       details.Year,
				PosterUrl:  details.PosterURL,
				Overview:   details.Overview,
				Genres:     details.Genres,
				Seasons:    protoSeasons,
				ExternalIds: &pluginv1.ExternalIDs{
					ImdbId:    details.ExternalIDs.IMDbID,
					TmdbId:    details.ExternalIDs.TMDBID,
					SimklId:   details.ExternalIDs.SIMKLID,
					MalId:     details.ExternalIDs.MALID,
					AnilistId: details.ExternalIDs.AniListID,
					KitsuId:   details.ExternalIDs.KitsuID,
					Extra:     details.ExternalIDs.Extra,
					SanityId:  details.ExternalIDs.SanityID,
				},
			}, nil
		}
	}

	if s.cinemaService != nil {
		details, err := s.cinemaService.GetMetadata(ctx, req.ProviderId, req.MediaId)
		if err != nil {
			return nil, fmt.Errorf("get media details failed: %w", err)
		}

		var protoSeasons []*pluginv1.Season
		for _, season := range details.Seasons {
			s := &pluginv1.Season{
				SeasonNumber: season.SeasonNumber,
				Title:        season.Title,
			}
			for _, ep := range season.Episodes {
				s.Episodes = append(s.Episodes, &pluginv1.Episode{
					EpisodeNumber:   ep.EpisodeNumber,
					Title:           ep.Title,
					Overview:        ep.Overview,
					DurationSeconds: ep.DurationSeconds,
				})
			}
			protoSeasons = append(protoSeasons, s)
		}

		return &corev1.GetMediaDetailsResponse{
			Id:         details.ID,
			ProviderId: details.ProviderID,
			Title:      details.Title,
			Type:       mapCinemaTypeToProto(details.Type),
			Year:       details.Year,
			PosterUrl:  details.PosterURL,
			Overview:   details.Overview,
			Genres:     details.Genres,
			Seasons:    protoSeasons,
			ExternalIds: &pluginv1.ExternalIDs{
				ImdbId:    details.ExternalIDs.IMDbID,
				TmdbId:    details.ExternalIDs.TMDBID,
				SimklId:   details.ExternalIDs.SIMKLID,
				MalId:     details.ExternalIDs.MALID,
				AnilistId: details.ExternalIDs.AniListID,
				KitsuId:   details.ExternalIDs.KitsuID,
				Extra:     details.ExternalIDs.Extra,
				SanityId:  details.ExternalIDs.SanityID,
			},
		}, nil
	}

	return nil, fmt.Errorf("service not available for domain %v", req.Domain)
}

func (s *Server) GetStreams(ctx context.Context, req *corev1.GetStreamsRequest) (*corev1.GetStreamsResponse, error) {
	if s.cinemaService == nil {
		return nil, errors.New("cinema service not initialized")
	}
	streams, subs, err := s.cinemaService.GetStreams(ctx, req.ProviderId, req.MediaId, req.SeasonNumber, req.EpisodeNumber)
	if err != nil {
		return nil, fmt.Errorf("get streams failed: %w", err)
	}

	var protoStreams []*pluginv1.StreamSource
	for _, st := range streams {
		protoStreams = append(protoStreams, &pluginv1.StreamSource{
			Id:      st.ID,
			Title:   st.Title,
			Url:     st.URL,
			Format:  mapStreamFormatToProto(st.Format),
			Quality: st.Quality,
			Headers: st.Headers,
		})
	}

	var protoSubs []*pluginv1.Subtitle
	for _, sub := range subs {
		protoSubs = append(protoSubs, &pluginv1.Subtitle{
			Language:  sub.Language,
			Label:     sub.Label,
			Url:       sub.URL,
			Format:    mapSubtitleFormatToProto(sub.Format),
			IsDefault: sub.IsDefault,
		})
	}

	return &corev1.GetStreamsResponse{
		Streams:   protoStreams,
		Subtitles: protoSubs,
	}, nil
}

func (s *Server) GetChapterContent(ctx context.Context, req *corev1.GetChapterContentRequest) (*corev1.GetChapterContentResponse, error) {
	if s.readingService == nil {
		return nil, errors.New("reading service not initialized")
	}

	content, err := s.readingService.GetChapterContent(ctx, req.ProviderId, req.MediaId, req.ChapterId, req.ChapterNumber)
	if err != nil {
		return nil, fmt.Errorf("get chapter content failed: %w", err)
	}

	var protoPages []*pluginv1.PageItem
	for _, p := range content.Pages {
		protoPages = append(protoPages, &pluginv1.PageItem{
			PageNumber: p.PageNumber,
			Url:        p.URL,
			Headers:    p.Headers,
		})
	}

	return &corev1.GetChapterContentResponse{
		ChapterId:     content.ChapterID,
		Title:         content.Title,
		ChapterNumber: float32(content.ChapterNumber),
		Pages:         protoPages,
		TextContent:   content.TextContent,
	}, nil
}

func (s *Server) ListPlugins(ctx context.Context, req *corev1.ListPluginsRequest) (*corev1.ListPluginsResponse, error) {
	clients := s.pluginManager.ListAll()
	var plugins []*corev1.PluginInfo
	for _, c := range clients {
		m := c.Manifest()
		if m == nil {
			continue
		}
		plugins = append(plugins, &corev1.PluginInfo{
			Id:           m.Id,
			Name:         m.Name,
			Version:      m.Version,
			Description:  m.Description,
			Author:       m.Author,
			Domain:       m.Domain,
			Capabilities: m.Capabilities,
			IsBuiltin:    m.IsBuiltin,
			Status:       "RUNNING",
		})
	}

	return &corev1.ListPluginsResponse{Plugins: plugins}, nil
}

func parseListenAddr(addr string) (network string, address string, err error) {
	if strings.HasPrefix(addr, "unix://") {
		return "unix", strings.TrimPrefix(addr, "unix://"), nil
	}
	if strings.HasPrefix(addr, "tcp://") {
		u, err := url.Parse(addr)
		if err != nil {
			return "", "", err
		}
		return "tcp", u.Host, nil
	}
	if strings.Contains(addr, "/") {
		return "unix", addr, nil
	}
	return "tcp", addr, nil
}

func mapCinemaTypeToProto(t cinema.MediaType) pluginv1.MediaType {
	switch t {
	case cinema.MediaTypeMovie:
		return pluginv1.MediaType_MEDIA_TYPE_MOVIE
	case cinema.MediaTypeSeries:
		return pluginv1.MediaType_MEDIA_TYPE_SERIES
	case cinema.MediaTypeAnime:
		return pluginv1.MediaType_MEDIA_TYPE_ANIME
	default:
		return pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED
	}
}

func mapReadingTypeToProto(t reading.ReadingType) pluginv1.MediaType {
	switch t {
	case reading.ReadingTypeManga:
		return pluginv1.MediaType_MEDIA_TYPE_MANGA
	case reading.ReadingTypeWebtoon:
		return pluginv1.MediaType_MEDIA_TYPE_WEBTOON
	case reading.ReadingTypeWebook:
		return pluginv1.MediaType_MEDIA_TYPE_WEBOOK
	case reading.ReadingTypeBook:
		return pluginv1.MediaType_MEDIA_TYPE_BOOK
	default:
		return pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED
	}
}

func mapStreamFormatToProto(f cinema.StreamFormat) pluginv1.StreamFormat {
	switch f {
	case cinema.StreamFormatHLS:
		return pluginv1.StreamFormat_STREAM_FORMAT_HLS
	case cinema.StreamFormatDASH:
		return pluginv1.StreamFormat_STREAM_FORMAT_DASH
	case cinema.StreamFormatMP4:
		return pluginv1.StreamFormat_STREAM_FORMAT_MP4
	case cinema.StreamFormatMKV:
		return pluginv1.StreamFormat_STREAM_FORMAT_MKV
	default:
		return pluginv1.StreamFormat_STREAM_FORMAT_UNSPECIFIED
	}
}

func mapSubtitleFormatToProto(f cinema.SubtitleFormat) pluginv1.SubtitleFormat {
	switch f {
	case cinema.SubtitleFormatVTT:
		return pluginv1.SubtitleFormat_SUBTITLE_FORMAT_VTT
	case cinema.SubtitleFormatSRT:
		return pluginv1.SubtitleFormat_SUBTITLE_FORMAT_SRT
	default:
		return pluginv1.SubtitleFormat_SUBTITLE_FORMAT_UNSPECIFIED
	}
}
