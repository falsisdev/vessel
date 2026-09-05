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
	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/domain/locale"
	"github.com/falsisdev/vessel/core/internal/domain/reading"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/theme"
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
	libraryService *service.LibraryService
	streamService  *service.StreamService
	pluginManager  *plugin.Manager
	themeManager   *theme.Manager
	startTime      time.Time
	isUnixSocket   bool
	socketPath     string
}

func NewServer(
	cfg ServerConfig,
	cinemaSvc *service.CinemaService,
	readingSvc *service.ReadingService,
	librarySvc *service.LibraryService,
	streamSvc *service.StreamService,
	pluginMgr *plugin.Manager,
	themeMgr *theme.Manager,
) *Server {
	if cfg.Version == "" {
		cfg.Version = "1.0.0"
	}
	if themeMgr == nil {
		themeMgr = theme.NewManager()
	}
	return &Server{
		cfg:            cfg,
		cinemaService:  cinemaSvc,
		readingService: readingSvc,
		libraryService: librarySvc,
		streamService:  streamSvc,
		pluginManager:  pluginMgr,
		themeManager:   themeMgr,
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

func (s *Server) ListThemes(ctx context.Context, req *corev1.ListThemesRequest) (*corev1.ListThemesResponse, error) {
	if s.themeManager == nil {
		return nil, errors.New("theme manager not initialized")
	}

	themes := s.themeManager.List()
	active, _ := s.themeManager.GetActive()

	var activeThemeID, activeVariantID string
	if active != nil {
		activeThemeID = active.Theme.Manifest.ID
		activeVariantID = active.Variant.ID
	}

	var protoThemes []*corev1.ThemeSummary
	for _, th := range themes {
		var variants []*corev1.ThemeVariantInfo
		for _, v := range th.Manifest.Variants {
			variants = append(variants, &corev1.ThemeVariantInfo{
				Id:     v.ID,
				Name:   v.Name,
				IsDark: v.IsDark,
			})
		}
		activeVariant := th.Manifest.DefaultVariant
		if active != nil && active.Theme.Manifest.ID == th.Manifest.ID {
			activeVariant = active.Variant.ID
		}

		protoThemes = append(protoThemes, &corev1.ThemeSummary{
			Id:            th.Manifest.ID,
			Name:          th.Manifest.Name,
			Version:       th.Manifest.Version,
			Description:   th.Manifest.Description,
			Author:        th.Manifest.Author,
			IsBuiltin:     th.IsBuiltin,
			ActiveVariant: activeVariant,
			Variants:      variants,
		})
	}

	return &corev1.ListThemesResponse{
		Themes:          protoThemes,
		ActiveThemeId:   activeThemeID,
		ActiveVariantId: activeVariantID,
	}, nil
}

func (s *Server) GetActiveTheme(ctx context.Context, req *corev1.GetActiveThemeRequest) (*corev1.GetActiveThemeResponse, error) {
	if s.themeManager == nil {
		return nil, errors.New("theme manager not initialized")
	}

	active, err := s.themeManager.GetActive()
	if err != nil {
		return nil, fmt.Errorf("failed to get active theme: %w", err)
	}

	var variants []*corev1.ThemeVariantInfo
	for _, v := range active.Theme.Manifest.Variants {
		variants = append(variants, &corev1.ThemeVariantInfo{
			Id:     v.ID,
			Name:   v.Name,
			IsDark: v.IsDark,
		})
	}

	return &corev1.GetActiveThemeResponse{
		Theme: &corev1.ThemeSummary{
			Id:            active.Theme.Manifest.ID,
			Name:          active.Theme.Manifest.Name,
			Version:       active.Theme.Manifest.Version,
			Description:   active.Theme.Manifest.Description,
			Author:        active.Theme.Manifest.Author,
			IsBuiltin:     active.Theme.IsBuiltin,
			ActiveVariant: active.Variant.ID,
			Variants:      variants,
		},
		VariantId: active.Variant.ID,
		IsDark:    active.Variant.IsDark,
		Tokens:    active.Tokens,
		Css:       active.CompiledCSS,
	}, nil
}

func (s *Server) SetActiveTheme(ctx context.Context, req *corev1.SetActiveThemeRequest) (*corev1.SetActiveThemeResponse, error) {
	if s.themeManager == nil {
		return nil, errors.New("theme manager not initialized")
	}

	active, err := s.themeManager.SetActive(req.ThemeId, req.VariantId)
	if err != nil {
		return nil, fmt.Errorf("failed to set active theme: %w", err)
	}

	var variants []*corev1.ThemeVariantInfo
	for _, v := range active.Theme.Manifest.Variants {
		variants = append(variants, &corev1.ThemeVariantInfo{
			Id:     v.ID,
			Name:   v.Name,
			IsDark: v.IsDark,
		})
	}

	return &corev1.SetActiveThemeResponse{
		Success: true,
		ActiveTheme: &corev1.GetActiveThemeResponse{
			Theme: &corev1.ThemeSummary{
				Id:            active.Theme.Manifest.ID,
				Name:          active.Theme.Manifest.Name,
				Version:       active.Theme.Manifest.Version,
				Description:   active.Theme.Manifest.Description,
				Author:        active.Theme.Manifest.Author,
				IsBuiltin:     active.Theme.IsBuiltin,
				ActiveVariant: active.Variant.ID,
				Variants:      variants,
			},
			VariantId: active.Variant.ID,
			IsDark:    active.Variant.IsDark,
			Tokens:    active.Tokens,
			Css:       active.CompiledCSS,
		},
	}, nil
}

func (s *Server) GetSupportedLocales(ctx context.Context, req *corev1.GetSupportedLocalesRequest) (*corev1.GetSupportedLocalesResponse, error) {
	var locales []*corev1.LocaleInfo
	for _, l := range locale.SupportedLocales {
		locales = append(locales, &corev1.LocaleInfo{
			Code:       l.Code,
			Name:       l.Name,
			NativeName: l.NativeName,
			IsRtl:      l.IsRTL,
		})
	}
	return &corev1.GetSupportedLocalesResponse{
		Locales:       locales,
		DefaultLocale: locale.DefaultLocale,
	}, nil
}

func (s *Server) SaveLibraryItem(ctx context.Context, req *corev1.SaveLibraryItemRequest) (*corev1.SaveLibraryItemResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}
	if req.Item == nil {
		return nil, errors.New("item cannot be nil")
	}

	item := mapLibraryItemFromProto(req.Item)
	if err := s.libraryService.SaveItem(ctx, item); err != nil {
		return nil, fmt.Errorf("failed to save library item: %w", err)
	}

	saved, err := s.libraryService.GetItem(ctx, item.ProviderID, item.MediaID)
	if err != nil {
		return nil, err
	}

	return &corev1.SaveLibraryItemResponse{Item: mapLibraryItemToProto(saved)}, nil
}

func (s *Server) GetLibraryItem(ctx context.Context, req *corev1.GetLibraryItemRequest) (*corev1.GetLibraryItemResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	item, err := s.libraryService.GetItem(ctx, req.ProviderId, req.MediaId)
	if err != nil {
		return nil, err
	}

	return &corev1.GetLibraryItemResponse{Item: mapLibraryItemToProto(item)}, nil
}

func (s *Server) ListLibraryItems(ctx context.Context, req *corev1.ListLibraryItemsRequest) (*corev1.ListLibraryItemsResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	filter := library.Filter{
		Domain: req.Domain,
		Status: mapLibraryStatusFromProto(req.Status),
		Limit:  int(req.Limit),
		Offset: int(req.Offset),
	}

	items, total, err := s.libraryService.ListItems(ctx, filter)
	if err != nil {
		return nil, err
	}

	var protoItems []*corev1.LibraryItem
	for _, it := range items {
		protoItems = append(protoItems, mapLibraryItemToProto(it))
	}

	return &corev1.ListLibraryItemsResponse{
		Items:      protoItems,
		TotalCount: int32(total),
	}, nil
}

func (s *Server) DeleteLibraryItem(ctx context.Context, req *corev1.DeleteLibraryItemRequest) (*corev1.DeleteLibraryItemResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	if err := s.libraryService.DeleteItem(ctx, req.ProviderId, req.MediaId); err != nil {
		return nil, err
	}

	return &corev1.DeleteLibraryItemResponse{Success: true}, nil
}

func (s *Server) SavePlaybackProgress(ctx context.Context, req *corev1.SavePlaybackProgressRequest) (*corev1.SavePlaybackProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}
	if req.Progress == nil {
		return nil, errors.New("progress cannot be nil")
	}

	p := mapPlaybackProgressFromProto(req.Progress)
	if err := s.libraryService.RecordPlayback(ctx, p); err != nil {
		return nil, err
	}

	saved, err := s.libraryService.GetPlayback(ctx, p.ProviderID, p.MediaID, p.SeasonNumber, p.EpisodeNumber)
	if err != nil {
		return nil, err
	}

	return &corev1.SavePlaybackProgressResponse{Progress: mapPlaybackProgressToProto(saved)}, nil
}

func (s *Server) GetPlaybackProgress(ctx context.Context, req *corev1.GetPlaybackProgressRequest) (*corev1.GetPlaybackProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	p, err := s.libraryService.GetPlayback(ctx, req.ProviderId, req.MediaId, req.SeasonNumber, req.EpisodeNumber)
	if err != nil {
		return nil, err
	}

	return &corev1.GetPlaybackProgressResponse{Progress: mapPlaybackProgressToProto(p)}, nil
}

func (s *Server) ListRecentPlaybackProgress(ctx context.Context, req *corev1.ListRecentPlaybackProgressRequest) (*corev1.ListRecentPlaybackProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	list, err := s.libraryService.ListRecentPlayback(ctx, int(req.Limit))
	if err != nil {
		return nil, err
	}

	var protoList []*corev1.PlaybackProgress
	for _, p := range list {
		protoList = append(protoList, mapPlaybackProgressToProto(p))
	}

	return &corev1.ListRecentPlaybackProgressResponse{Items: protoList}, nil
}

func (s *Server) SaveReadingProgress(ctx context.Context, req *corev1.SaveReadingProgressRequest) (*corev1.SaveReadingProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}
	if req.Progress == nil {
		return nil, errors.New("progress cannot be nil")
	}

	p := mapReadingProgressFromProto(req.Progress)
	if err := s.libraryService.RecordReading(ctx, p); err != nil {
		return nil, err
	}

	saved, err := s.libraryService.GetReading(ctx, p.ProviderID, p.MediaID, p.ChapterID)
	if err != nil {
		return nil, err
	}

	return &corev1.SaveReadingProgressResponse{Progress: mapReadingProgressToProto(saved)}, nil
}

func (s *Server) GetReadingProgress(ctx context.Context, req *corev1.GetReadingProgressRequest) (*corev1.GetReadingProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	p, err := s.libraryService.GetReading(ctx, req.ProviderId, req.MediaId, req.ChapterId)
	if err != nil {
		return nil, err
	}

	return &corev1.GetReadingProgressResponse{Progress: mapReadingProgressToProto(p)}, nil
}

func (s *Server) ListRecentReadingProgress(ctx context.Context, req *corev1.ListRecentReadingProgressRequest) (*corev1.ListRecentReadingProgressResponse, error) {
	if s.libraryService == nil {
		return nil, errors.New("library service not initialized")
	}

	list, err := s.libraryService.ListRecentReading(ctx, int(req.Limit))
	if err != nil {
		return nil, err
	}

	var protoList []*corev1.ReadingProgress
	for _, p := range list {
		protoList = append(protoList, mapReadingProgressToProto(p))
	}

	return &corev1.ListRecentReadingProgressResponse{Items: protoList}, nil
}

func (s *Server) ResolveStream(ctx context.Context, req *corev1.ResolveStreamRequest) (*corev1.ResolveStreamResponse, error) {
	if s.streamService == nil {
		return nil, errors.New("stream service not available")
	}

	res, err := s.streamService.ResolveStream(ctx, req.StreamUrl, req.Title, int(req.SeasonNumber), int(req.EpisodeNumber), req.PreferredProvider)
	if err != nil {
		return nil, err
	}
	return &corev1.ResolveStreamResponse{Stream: res}, nil
}

func (s *Server) GetDebridStatus(ctx context.Context, req *corev1.GetDebridStatusRequest) (*corev1.GetDebridStatusResponse, error) {
	if s.streamService == nil {
		return &corev1.GetDebridStatusResponse{}, nil
	}

	statuses, err := s.streamService.GetDebridStatus(ctx, req.Provider)
	if err != nil {
		return nil, err
	}
	return &corev1.GetDebridStatusResponse{Accounts: statuses}, nil
}

func (s *Server) ConfigureDebrid(ctx context.Context, req *corev1.ConfigureDebridRequest) (*corev1.ConfigureDebridResponse, error) {
	if s.streamService == nil {
		return nil, errors.New("stream service not available")
	}

	status, err := s.streamService.ConfigureDebrid(ctx, req.Provider, req.ApiKey, req.Enabled)
	if err != nil {
		return &corev1.ConfigureDebridResponse{
			Success: false,
			Message: err.Error(),
		}, nil
	}

	return &corev1.ConfigureDebridResponse{
		Success: true,
		Message: "debrid provider configured successfully",
		Status:  status,
	}, nil
}

func mapLibraryItemToProto(it *library.Item) *corev1.LibraryItem {
	if it == nil {
		return nil
	}
	return &corev1.LibraryItem{
		Id:               it.ID,
		ProviderId:       it.ProviderID,
		MediaId:          it.MediaID,
		Domain:           it.Domain,
		Title:            it.Title,
		Type:             it.Type,
		PosterUrl:        it.PosterURL,
		Status:           mapLibraryStatusToProto(it.Status),
		UserRating:       it.UserRating,
		LastInteractedAt: it.LastInteractedAt.Unix(),
		CreatedAt:        it.CreatedAt.Unix(),
		UpdatedAt:        it.UpdatedAt.Unix(),
	}
}

func mapLibraryItemFromProto(it *corev1.LibraryItem) *library.Item {
	if it == nil {
		return nil
	}
	return &library.Item{
		ID:               it.Id,
		ProviderID:       it.ProviderId,
		MediaID:          it.MediaId,
		Domain:           it.Domain,
		Title:            it.Title,
		Type:             it.Type,
		PosterURL:        it.PosterUrl,
		Status:           mapLibraryStatusFromProto(it.Status),
		UserRating:       it.UserRating,
		LastInteractedAt: time.Unix(it.LastInteractedAt, 0),
		CreatedAt:        time.Unix(it.CreatedAt, 0),
		UpdatedAt:        time.Unix(it.UpdatedAt, 0),
	}
}

func mapLibraryStatusToProto(s library.Status) corev1.LibraryStatus {
	switch s {
	case library.StatusPlanToWatch:
		return corev1.LibraryStatus_LIBRARY_STATUS_PLAN_TO_WATCH
	case library.StatusWatching:
		return corev1.LibraryStatus_LIBRARY_STATUS_WATCHING
	case library.StatusCompleted:
		return corev1.LibraryStatus_LIBRARY_STATUS_COMPLETED
	case library.StatusOnHold:
		return corev1.LibraryStatus_LIBRARY_STATUS_ON_HOLD
	case library.StatusDropped:
		return corev1.LibraryStatus_LIBRARY_STATUS_DROPPED
	case library.StatusFavorite:
		return corev1.LibraryStatus_LIBRARY_STATUS_FAVORITE
	default:
		return corev1.LibraryStatus_LIBRARY_STATUS_UNSPECIFIED
	}
}

func mapLibraryStatusFromProto(s corev1.LibraryStatus) library.Status {
	switch s {
	case corev1.LibraryStatus_LIBRARY_STATUS_PLAN_TO_WATCH:
		return library.StatusPlanToWatch
	case corev1.LibraryStatus_LIBRARY_STATUS_WATCHING:
		return library.StatusWatching
	case corev1.LibraryStatus_LIBRARY_STATUS_COMPLETED:
		return library.StatusCompleted
	case corev1.LibraryStatus_LIBRARY_STATUS_ON_HOLD:
		return library.StatusOnHold
	case corev1.LibraryStatus_LIBRARY_STATUS_DROPPED:
		return library.StatusDropped
	case corev1.LibraryStatus_LIBRARY_STATUS_FAVORITE:
		return library.StatusFavorite
	default:
		return library.StatusUnspecified
	}
}

func mapPlaybackProgressToProto(p *library.PlaybackProgress) *corev1.PlaybackProgress {
	if p == nil {
		return nil
	}
	return &corev1.PlaybackProgress{
		ProviderId:             p.ProviderID,
		MediaId:                p.MediaID,
		Domain:                 p.Domain,
		SeasonNumber:           p.SeasonNumber,
		EpisodeNumber:          p.EpisodeNumber,
		CurrentPositionSeconds: p.CurrentPositionSeconds,
		TotalDurationSeconds:   p.TotalDurationSeconds,
		ProgressPercent:        p.ProgressPercent,
		IsCompleted:            p.IsCompleted,
		UpdatedAt:              p.UpdatedAt.Unix(),
	}
}

func mapPlaybackProgressFromProto(p *corev1.PlaybackProgress) *library.PlaybackProgress {
	if p == nil {
		return nil
	}
	return &library.PlaybackProgress{
		ProviderID:             p.ProviderId,
		MediaID:                p.MediaId,
		Domain:                 p.Domain,
		SeasonNumber:           p.SeasonNumber,
		EpisodeNumber:          p.EpisodeNumber,
		CurrentPositionSeconds: p.CurrentPositionSeconds,
		TotalDurationSeconds:   p.TotalDurationSeconds,
		ProgressPercent:        p.ProgressPercent,
		IsCompleted:            p.IsCompleted,
		UpdatedAt:              time.Unix(p.UpdatedAt, 0),
	}
}

func mapReadingProgressToProto(p *library.ReadingProgress) *corev1.ReadingProgress {
	if p == nil {
		return nil
	}
	return &corev1.ReadingProgress{
		ProviderId:      p.ProviderID,
		MediaId:         p.MediaID,
		Domain:          p.Domain,
		ChapterId:       p.ChapterID,
		ChapterNumber:   p.ChapterNumber,
		CurrentPage:     p.CurrentPage,
		TotalPages:      p.TotalPages,
		TextScrollRatio: p.TextScrollRatio,
		IsCompleted:     p.IsCompleted,
		UpdatedAt:       p.UpdatedAt.Unix(),
	}
}

func mapReadingProgressFromProto(p *corev1.ReadingProgress) *library.ReadingProgress {
	if p == nil {
		return nil
	}
	return &library.ReadingProgress{
		ProviderID:      p.ProviderId,
		MediaID:         p.MediaId,
		Domain:          p.Domain,
		ChapterID:       p.ChapterId,
		ChapterNumber:   p.ChapterNumber,
		CurrentPage:     p.CurrentPage,
		TotalPages:      p.TotalPages,
		TextScrollRatio: p.TextScrollRatio,
		IsCompleted:     p.IsCompleted,
		UpdatedAt:       time.Unix(p.UpdatedAt, 0),
	}
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
