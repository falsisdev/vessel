package provider

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"github.com/falsisdev/vessel/plugins/cinemasis/internal/tmdb"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

const (
	PluginID        = "com.vessel.cinema.cinemasis"
	PluginName      = "Cinemasis"
	PluginVersion   = "1.0.0"
	ProtocolVersion = "1.0.0"
)

type CinemasisService struct {
	pluginv1.UnimplementedPluginServiceServer
	client *tmdb.Client
}

func NewCinemasisService(client *tmdb.Client) *CinemasisService {
	return &CinemasisService{
		client: client,
	}
}

func (s *CinemasisService) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:          PluginID,
			Name:        PluginName,
			Version:     PluginVersion,
			Description: "Official TMDB-based cinema catalog and metadata provider",
			Author:      "Vessel Team",
			Domain:      pluginv1.Domain_DOMAIN_CINEMA,
			Capabilities: []pluginv1.Capability{
				pluginv1.Capability_CAPABILITY_SEARCH,
				pluginv1.Capability_CAPABILITY_METADATA,
			},
			ProtocolVersion: ProtocolVersion,
			IsBuiltin:       true,
		},
	}, nil
}

func (s *CinemasisService) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	resp, err := s.client.Search(ctx, req.Query, req.Page)
	if err != nil {
		return nil, fmt.Errorf("Cinemasis search error: %w", err)
	}

	var items []*pluginv1.MediaItem
	for _, res := range resp.Results {
		if res.MediaType != "movie" && res.MediaType != "tv" {
			continue
		}

		item := &pluginv1.MediaItem{
			Id:        fmt.Sprintf("%s:%d", res.MediaType, res.ID),
			Overview:  res.Overview,
			PosterUrl: s.client.BuildPosterURL(res.PosterPath),
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(res.ID),
			},
		}

		if res.MediaType == "movie" {
			item.Title = res.Title
			item.Year = parseYear(res.ReleaseDate)
			item.Type = classifyMediaType(res.MediaType, res.OriginalLanguage, res.GenreIDs)
		} else {
			item.Title = res.Name
			item.Year = parseYear(res.FirstAirDate)
			item.Type = classifyMediaType(res.MediaType, res.OriginalLanguage, res.GenreIDs)
		}

		items = append(items, item)
	}

	return &pluginv1.SearchResponse{
		Items:   items,
		HasMore: resp.Page < resp.TotalPages,
	}, nil
}

func (s *CinemasisService) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	mediaType, idStr, found := strings.Cut(req.MediaId, ":")
	if !found {
		return nil, fmt.Errorf("Invalid Cinemasis media_id format (expected type:id): %s", req.MediaId)
	}

	id, err := strconv.Atoi(idStr)
	if err != nil {
		return nil, fmt.Errorf("Invalid media id integer: %w", err)
	}

	switch mediaType {
	case "movie":
		movie, err := s.client.GetMovie(ctx, id)
		if err != nil {
			return nil, fmt.Errorf("Failed to fetch movie details from TMDB: %w", err)
		}

		var genreNames []string
		var genreIDs []int
		for _, g := range movie.Genres {
			genreNames = append(genreNames, g.Name)
			genreIDs = append(genreIDs, g.ID)
		}

		details := &pluginv1.MediaDetails{
			Id:        req.MediaId,
			Title:     movie.Title,
			Type:      classifyMediaType("movie", movie.OriginalLanguage, genreIDs),
			Year:      parseYear(movie.ReleaseDate),
			PosterUrl: s.client.BuildPosterURL(movie.PosterPath),
			Overview:  movie.Overview,
			Genres:    genreNames,
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(movie.ID),
				ImdbId: movie.ExternalIDs.IMDbID,
			},
		}
		return &pluginv1.GetMetadataResponse{Details: details}, nil

	case "tv":
		tv, err := s.client.GetTV(ctx, id)
		if err != nil {
			return nil, fmt.Errorf("failed to fetch tv details from TMDB: %w", err)
		}

		var genreNames []string
		var genreIDs []int
		for _, g := range tv.Genres {
			genreNames = append(genreNames, g.Name)
			genreIDs = append(genreIDs, g.ID)
		}

		details := &pluginv1.MediaDetails{
			Id:        req.MediaId,
			Title:     tv.Name,
			Type:      classifyMediaType("tv", tv.OriginalLanguage, genreIDs),
			Year:      parseYear(tv.FirstAirDate),
			PosterUrl: s.client.BuildPosterURL(tv.PosterPath),
			Overview:  tv.Overview,
			Genres:    genreNames,
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(tv.ID),
				ImdbId: tv.ExternalIDs.IMDbID,
			},
		}

		for _, sOverview := range tv.Seasons {
			if sOverview.SeasonNumber <= 0 {
				continue // skip specials season 0 by default
			}
			seasonDetails, err := s.client.GetTVSeason(ctx, id, sOverview.SeasonNumber)
			if err != nil {
				continue
			}

			season := &pluginv1.Season{
				SeasonNumber: int32(seasonDetails.SeasonNumber),
				Title:        seasonDetails.Name,
			}
			for _, ep := range seasonDetails.Episodes {
				season.Episodes = append(season.Episodes, &pluginv1.Episode{
					EpisodeNumber:   int32(ep.EpisodeNumber),
					Title:           ep.Name,
					Overview:        ep.Overview,
					DurationSeconds: int64(ep.Runtime * 60),
				})
			}
			details.Seasons = append(details.Seasons, season)
		}

		return &pluginv1.GetMetadataResponse{Details: details}, nil

	default:
		return nil, fmt.Errorf("unsupported cinemasis media type: %s", mediaType)
	}
}

func (s *CinemasisService) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	// Catalog providers do not supply video streams
	return &pluginv1.GetStreamsResponse{}, nil
}

func classifyMediaType(tmdbMediaType, originalLanguage string, genreIDs []int) pluginv1.MediaType {
	isAnimation := false
	for _, id := range genreIDs {
		if id == 16 { // TMDB Animation genre ID
			isAnimation = true
			break
		}
	}

	if isAnimation && strings.EqualFold(originalLanguage, "ja") {
		return pluginv1.MediaType_MEDIA_TYPE_ANIME
	}

	if tmdbMediaType == "movie" {
		return pluginv1.MediaType_MEDIA_TYPE_MOVIE
	}
	return pluginv1.MediaType_MEDIA_TYPE_SERIES
}

func parseYear(dateStr string) int32 {
	if len(dateStr) < 4 {
		return 0
	}
	year, err := strconv.Atoi(dateStr[:4])
	if err != nil {
		return 0
	}
	return int32(year)
}
