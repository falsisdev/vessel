package provider

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"strconv"
	"strings"

	"github.com/falsisdev/vessel/plugins/mangile/sanity"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc/codes"
	"google.golang.org/grpc/status"
)

const (
	PluginID        = "com.vessel.reading.mangile"
	PluginName      = "Mangile"
	PluginVersion   = "1.0.0"
	ProtocolVersion = "1.0.0"
)

type MangileService struct {
	pluginv1.UnimplementedPluginServiceServer
	client *sanity.Client
}

func NewMangileService(client *sanity.Client) *MangileService {
	return &MangileService{
		client: client,
	}
}

func (s *MangileService) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:          PluginID,
			Name:        PluginName,
			Version:     PluginVersion,
			Description: "Official embedded reading provider for Manga, Webtoon, and Webook powered by Mangile",
			Author:      "Vessel Team",
			Domain:      pluginv1.Domain_DOMAIN_MANGA,
			Capabilities: []pluginv1.Capability{
				pluginv1.Capability_CAPABILITY_SEARCH,
				pluginv1.Capability_CAPABILITY_METADATA,
				pluginv1.Capability_CAPABILITY_READ,
			},
			ProtocolVersion: ProtocolVersion,
			IsBuiltin:       true,
		},
	}, nil
}

func (s *MangileService) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	limit := 30
	if req.Page > 1 {
		limit = int(req.Page) * 30
	}

	q := strings.TrimSpace(req.Query)
	if strings.EqualFold(q, "popular") || strings.EqualFold(q, "trending") || strings.EqualFold(q, "latest") || strings.EqualFold(q, "all") {
		q = ""
	}

	titles, err := s.client.SearchTitles(ctx, q, limit)
	if err != nil {
		return nil, fmt.Errorf("mangile search error: %w", err)
	}

	var items []*pluginv1.MediaItem
	for _, t := range titles {
		mediaType := classifyReadingType(t.Type, t.Format)
		var malID string
		if t.MyAnimeListID > 0 {
			malID = strconv.Itoa(t.MyAnimeListID)
		}

		item := &pluginv1.MediaItem{
			Id:        t.ID,
			Title:     t.Title,
			Type:      mediaType,
			PosterUrl: t.CoverImage,
			Overview:  t.Description,
			ExternalIds: &pluginv1.ExternalIDs{
				SanityId: t.ID,
				MalId:    malID,
			},
		}
		items = append(items, item)
	}

	return &pluginv1.SearchResponse{
		Items:   items,
		HasMore: len(titles) >= limit,
	}, nil
}

func (s *MangileService) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	title, err := s.client.GetTitle(ctx, req.MediaId)
	if err != nil {
		if errors.Is(err, sanity.ErrResourceNotFound) {
			return nil, status.Errorf(codes.NotFound, "title %s not found", req.MediaId)
		}
		return nil, fmt.Errorf("mangile get metadata error: %w", err)
	}

	var episodes []*pluginv1.Episode
	var chaptersRaw []map[string]any
	for _, ch := range title.Chapters {
		chID := ch.GetID()
		episodes = append(episodes, &pluginv1.Episode{
			EpisodeNumber: int32(ch.ChapterNumber),
			Title:         ch.Title,
		})
		chaptersRaw = append(chaptersRaw, map[string]any{
			"id":             chID,
			"title":          ch.Title,
			"chapter_number": ch.ChapterNumber,
			"volume_number":  ch.VolumeNumber,
		})
	}

	season := &pluginv1.Season{
		SeasonNumber: 1,
		Title:        "Chapters",
		Episodes:     episodes,
	}

	var malID string
	if title.MyAnimeListID > 0 {
		malID = strconv.Itoa(title.MyAnimeListID)
	}

	extra := make(map[string]string)
	if chaptersBytes, err := json.Marshal(chaptersRaw); err == nil {
		extra["chapters_json"] = string(chaptersBytes)
	}

	details := &pluginv1.MediaDetails{
		Id:        title.ID,
		Title:     title.Title,
		Type:      classifyReadingType(title.Type, title.Format),
		PosterUrl: title.CoverImage,
		Overview:  title.Description,
		Genres:    title.Tags,
		Seasons:   []*pluginv1.Season{season},
		ExternalIds: &pluginv1.ExternalIDs{
			SanityId: title.ID,
			MalId:    malID,
			Extra:    extra,
		},
	}

	return &pluginv1.GetMetadataResponse{Details: details}, nil
}

func (s *MangileService) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{}, nil
}

func (s *MangileService) GetChapterContent(ctx context.Context, req *pluginv1.GetChapterContentRequest) (*pluginv1.GetChapterContentResponse, error) {
	chapterID := req.ChapterId
	mediaID := req.MediaId
	chapterNum := float64(req.ChapterNumber)

	var chapter *sanity.SanityChapterDetails
	var err error

	// 1. If chapterID is provided, first try standalone chapter lookup
	if chapterID != "" {
		chapter, err = s.client.GetChapter(ctx, chapterID)
		if err != nil && !errors.Is(err, sanity.ErrResourceNotFound) {
			// Non-critical error, continue to title lookup
		}
	}

	// 2. If not found and mediaID is provided, try title embedded lookup by chapterID
	if (chapter == nil || errors.Is(err, sanity.ErrResourceNotFound)) && mediaID != "" && chapterID != "" {
		chapter, err = s.client.GetChapterFromTitle(ctx, mediaID, chapterID, chapterNum)
	}

	// 3. If still not found and mediaID is provided, try title embedded lookup by chapterNumber
	if (chapter == nil || errors.Is(err, sanity.ErrResourceNotFound)) && mediaID != "" {
		chapter, err = s.client.GetChapterFromTitle(ctx, mediaID, "", chapterNum)
	}

	// 4. If still not found, search title chapter list for the matching ID
	if (chapter == nil || errors.Is(err, sanity.ErrResourceNotFound)) && mediaID != "" {
		title, titleErr := s.client.GetTitle(ctx, mediaID)
		if titleErr == nil && title != nil {
			for _, ch := range title.Chapters {
				if float32(ch.ChapterNumber) == req.ChapterNumber || (chapterID != "" && (ch.ID == chapterID || ch.Key == chapterID)) {
					targetID := ch.GetID()
					chapter, err = s.client.GetChapter(ctx, targetID)
					if err != nil || chapter == nil {
						chapter, err = s.client.GetChapterFromTitle(ctx, mediaID, targetID, ch.ChapterNumber)
					}
					if chapter != nil {
						break
					}
				}
			}
		}
	}

	if err != nil {
		if errors.Is(err, sanity.ErrResourceNotFound) {
			return nil, status.Errorf(codes.NotFound, "chapter %s (media: %s, num: %f) not found", chapterID, mediaID, chapterNum)
		}
		return nil, fmt.Errorf("mangile get chapter error: %w", err)
	}

	if chapter == nil {
		return nil, status.Errorf(codes.NotFound, "chapter not found")
	}

	var pages []*pluginv1.PageItem
	for i, p := range chapter.Pages {
		pages = append(pages, &pluginv1.PageItem{
			PageNumber: int32(i + 1),
			Url:        p.URL,
		})
	}

	return &pluginv1.GetChapterContentResponse{
		ChapterId:     chapter.GetID(),
		Title:         chapter.Title,
		ChapterNumber: float32(chapter.ChapterNumber),
		Pages:         pages,
		TextContent:   chapter.ExtractTextContent(),
	}, nil
}

func classifyReadingType(docType, format string) pluginv1.MediaType {
	f := strings.ToLower(format)
	if f == "webtoon" || f == "manhwa" || f == "manhua" {
		return pluginv1.MediaType_MEDIA_TYPE_WEBTOON
	}
	if docType == "lightNovel" || f == "lightnovel" || f == "novel" || f == "webook" {
		return pluginv1.MediaType_MEDIA_TYPE_WEBOOK
	}
	return pluginv1.MediaType_MEDIA_TYPE_MANGA
}
