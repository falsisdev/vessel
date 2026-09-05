package integration_test

import (
	"context"
	"errors"
	"net"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/cinema"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

type mockServer struct {
	pluginv1.UnimplementedPluginServiceServer
}

func (s *mockServer) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:          "com.vessel.cinema.mock",
			Name:        "Mock Cinema Provider",
			Version:     "1.0.0",
			Description: "Mock cinema provider for integration tests",
			Author:      "Vessel Team",
			Domain:      pluginv1.Domain_DOMAIN_CINEMA,
			Capabilities: []pluginv1.Capability{
				pluginv1.Capability_CAPABILITY_SEARCH,
				pluginv1.Capability_CAPABILITY_METADATA,
				pluginv1.Capability_CAPABILITY_STREAMS,
			},
			ProtocolVersion: "1.0.0",
		},
	}, nil
}

func (s *mockServer) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	return &pluginv1.SearchResponse{
		Items: []*pluginv1.MediaItem{
			{
				Id:        "batman-begins",
				Title:     "Batman Begins",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2005,
				PosterUrl: "https://images.vessel.local/posters/batman-begins.jpg",
				Overview:  "Batman begins his fight against crime.",
			},
			{
				Id:        "the-dark-knight",
				Title:     "The Dark Knight",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2008,
				PosterUrl: "https://images.vessel.local/posters/the-dark-knight.jpg",
				Overview:  "Batman faces the Joker.",
			},
			{
				Id:        "batman-animated",
				Title:     "Batman: The Animated Series",
				Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
				Year:      1992,
				PosterUrl: "https://images.vessel.local/posters/batman-animated.jpg",
				Overview:  "The Dark Knight battles crime in Gotham City.",
			},
		},
		HasMore: false,
	}, nil
}

func (s *mockServer) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	if req.MediaId == "batman-animated" {
		return &pluginv1.GetMetadataResponse{
			Details: &pluginv1.MediaDetails{
				Id:        "batman-animated",
				Title:     "Batman: The Animated Series",
				Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
				Year:      1992,
				PosterUrl: "https://images.vessel.local/posters/batman-animated.jpg",
				Overview:  "Animated adventures of Batman.",
				Genres:    []string{"Animation", "Action"},
				Seasons: []*pluginv1.Season{
					{
						SeasonNumber: 1,
						Title:        "Season 1",
						Episodes: []*pluginv1.Episode{
							{
								EpisodeNumber:   1,
								Title:           "On Leather Wings",
								Overview:        "Bat creature in Gotham.",
								DurationSeconds: 1320,
							},
							{
								EpisodeNumber:   2,
								Title:           "Christmas with the Joker",
								Overview:        "Holiday breakout.",
								DurationSeconds: 1320,
							},
						},
					},
				},
			},
		}, nil
	}

	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:        "the-dark-knight",
			Title:     "The Dark Knight",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2008,
			PosterUrl: "https://images.vessel.local/posters/the-dark-knight.jpg",
			Overview:  "Batman takes on the Joker.",
			Genres:    []string{"Action", "Crime", "Drama"},
		},
	}, nil
}

func (s *mockServer) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{
			{
				Id:      "hls-1080p",
				Title:   "HLS 1080p",
				Url:     "https://mock.stream.vessel.local/hls/master.m3u8",
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_HLS,
				Quality: "1080p",
				Headers: map[string]string{
					"User-Agent": "Vessel/1.0",
				},
			},
			{
				Id:      "mp4-720p",
				Title:   "Direct MP4 720p",
				Url:     "https://mock.stream.vessel.local/mp4/720p.mp4",
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_MP4,
				Quality: "720p",
			},
		},
		Subtitles: []*pluginv1.Subtitle{
			{
				Language:  "en",
				Label:     "English",
				Url:       "https://mock.stream.vessel.local/subs/en.vtt",
				Format:    pluginv1.SubtitleFormat_SUBTITLE_FORMAT_VTT,
				IsDefault: true,
			},
			{
				Language:  "tr",
				Label:     "Türkçe",
				Url:       "https://mock.stream.vessel.local/subs/tr.srt",
				Format:    pluginv1.SubtitleFormat_SUBTITLE_FORMAT_SRT,
				IsDefault: false,
			},
		},
	}, nil
}

func startMockPluginServer(t *testing.T) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on dynamic port: %v", err)
	}

	server := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(server, &mockServer{})

	go func() {
		_ = server.Serve(lis)
	}()

	cleanup := func() {
		server.Stop()
		_ = lis.Close()
	}

	return lis.Addr().String(), cleanup
}

type failingServer struct {
	pluginv1.UnimplementedPluginServiceServer
}

func (s *failingServer) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:              "com.vessel.cinema.failing",
			Name:            "Failing Cinema Provider",
			Version:         "0.0.1",
			Domain:          pluginv1.Domain_DOMAIN_CINEMA,
			Capabilities:    []pluginv1.Capability{pluginv1.Capability_CAPABILITY_SEARCH},
			ProtocolVersion: "1.0.0",
		},
	}, nil
}

func (s *failingServer) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	return nil, errors.New("upstream scraper timeout")
}

func startFailingPluginServer(t *testing.T) (string, func()) {
	t.Helper()
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on dynamic port: %v", err)
	}

	server := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(server, &failingServer{})

	go func() {
		_ = server.Serve(lis)
	}()

	cleanup := func() {
		server.Stop()
		_ = lis.Close()
	}

	return lis.Addr().String(), cleanup
}

func TestCompleteCinemaFlow(t *testing.T) {
	addr, stopPlugin := startMockPluginServer(t)
	defer stopPlugin()

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	client, err := plugin.Dial(ctx, addr)
	if err != nil {
		t.Fatalf("failed to dial plugin: %v", err)
	}
	defer client.Close()

	// 1. Verify Manifest Handshake
	manifest := client.Manifest()
	if manifest == nil {
		t.Fatal("expected manifest to be non-nil")
	}
	if manifest.Id != "com.vessel.cinema.mock" {
		t.Errorf("expected manifest.id 'com.vessel.cinema.mock', got '%s'", manifest.Id)
	}
	if manifest.Domain != pluginv1.Domain_DOMAIN_CINEMA {
		t.Errorf("expected domain CINEMA, got %v", manifest.Domain)
	}

	manager := plugin.NewManager()
	if err := manager.Register(client); err != nil {
		t.Fatalf("failed to register plugin: %v", err)
	}

	cinemaService := service.NewCinemaService(manager, 3*time.Second)

	// 2. Test Search Aggregation
	t.Run("Search Batman", func(t *testing.T) {
		results, err := cinemaService.Search(ctx, "Batman")
		if err != nil {
			t.Fatalf("search failed: %v", err)
		}

		if len(results) != 3 {
			t.Fatalf("expected 3 search results, got %d", len(results))
		}

		first := results[0]
		if first.Title != "Batman Begins" {
			t.Errorf("expected first title 'Batman Begins', got '%s'", first.Title)
		}
		if first.ProviderID != "com.vessel.cinema.mock" {
			t.Errorf("expected provider ID 'com.vessel.cinema.mock', got '%s'", first.ProviderID)
		}
		if first.Type != cinema.MediaTypeMovie {
			t.Errorf("expected MediaTypeMovie, got %v", first.Type)
		}

		third := results[2]
		if third.Type != cinema.MediaTypeSeries {
			t.Errorf("expected MediaTypeSeries for animated show, got %v", third.Type)
		}
	})

	// 3. Test Metadata Retrieval
	t.Run("GetMetadata Movie", func(t *testing.T) {
		details, err := cinemaService.GetMetadata(ctx, "com.vessel.cinema.mock", "the-dark-knight")
		if err != nil {
			t.Fatalf("get metadata failed: %v", err)
		}
		if details.Title != "The Dark Knight" {
			t.Errorf("expected title 'The Dark Knight', got '%s'", details.Title)
		}
		if len(details.Genres) != 3 {
			t.Errorf("expected 3 genres, got %d", len(details.Genres))
		}
	})

	t.Run("GetMetadata Series with Seasons and Episodes", func(t *testing.T) {
		details, err := cinemaService.GetMetadata(ctx, "com.vessel.cinema.mock", "batman-animated")
		if err != nil {
			t.Fatalf("get metadata failed: %v", err)
		}
		if len(details.Seasons) != 1 {
			t.Fatalf("expected 1 season, got %d", len(details.Seasons))
		}
		if len(details.Seasons[0].Episodes) != 2 {
			t.Fatalf("expected 2 episodes, got %d", len(details.Seasons[0].Episodes))
		}
		ep1 := details.Seasons[0].Episodes[0]
		if ep1.Title != "On Leather Wings" {
			t.Errorf("expected ep1 title 'On Leather Wings', got '%s'", ep1.Title)
		}
	})

	// 4. Test Stream Resolution
	t.Run("GetStreams and Subtitles", func(t *testing.T) {
		streams, subtitles, err := cinemaService.GetStreams(ctx, "com.vessel.cinema.mock", "the-dark-knight", 0, 0)
		if err != nil {
			t.Fatalf("get streams failed: %v", err)
		}
		if len(streams) != 2 {
			t.Fatalf("expected 2 streams, got %d", len(streams))
		}
		if streams[0].Format != cinema.StreamFormatHLS {
			t.Errorf("expected HLS format, got %v", streams[0].Format)
		}
		if streams[0].Headers["User-Agent"] != "Vessel/1.0" {
			t.Errorf("expected custom header preserved, got %s", streams[0].Headers["User-Agent"])
		}

		if len(subtitles) != 2 {
			t.Fatalf("expected 2 subtitles, got %d", len(subtitles))
		}
		if !subtitles[0].IsDefault || subtitles[0].Language != "en" {
			t.Errorf("expected default english subtitle, got %+v", subtitles[0])
		}
	})
}

func TestResilienceWithFailingPlugin(t *testing.T) {
	goodAddr, stopGood := startMockPluginServer(t)
	defer stopGood()

	failAddr, stopFail := startFailingPluginServer(t)
	defer stopFail()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	goodClient, err := plugin.Dial(ctx, goodAddr)
	if err != nil {
		t.Fatalf("failed to dial good plugin: %v", err)
	}
	defer goodClient.Close()

	failClient, err := plugin.Dial(ctx, failAddr)
	if err != nil {
		t.Fatalf("failed to dial failing plugin: %v", err)
	}
	defer failClient.Close()

	manager := plugin.NewManager()
	if err := manager.Register(goodClient); err != nil {
		t.Fatalf("failed to register good client: %v", err)
	}
	if err := manager.Register(failClient); err != nil {
		t.Fatalf("failed to register failing client: %v", err)
	}

	cinemaService := service.NewCinemaService(manager, 2*time.Second)

	// Search must succeed and return good plugin results even when one plugin fails
	results, err := cinemaService.Search(ctx, "Batman")
	if err != nil {
		t.Fatalf("expected search to be resilient and succeed, got error: %v", err)
	}

	if len(results) != 3 {
		t.Fatalf("expected 3 results from healthy plugin, got %d", len(results))
	}
}
