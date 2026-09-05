package main

import (
	"context"
	"flag"
	"fmt"
	"log"
	"net"
	"strings"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

type MockCinemaPlugin struct {
	pluginv1.UnimplementedPluginServiceServer
}

func (p *MockCinemaPlugin) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:          "com.vessel.cinema.mock",
			Name:        "Mock Cinema Provider",
			Version:     "1.0.0",
			Description: "Mock cinema provider for architectural validation",
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

func (p *MockCinemaPlugin) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	q := strings.ToLower(strings.TrimSpace(req.Query))

	var items []*pluginv1.MediaItem
	if strings.Contains(q, "batman") {
		items = []*pluginv1.MediaItem{
			{
				Id:        "batman-begins",
				Title:     "Batman Begins",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2005,
				PosterUrl: "https://images.vessel.local/posters/batman-begins.jpg",
				Overview:  "After training with his mentor, Batman begins his fight against crime in Gotham.",
			},
			{
				Id:        "the-dark-knight",
				Title:     "The Dark Knight",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2008,
				PosterUrl: "https://images.vessel.local/posters/the-dark-knight.jpg",
				Overview:  "Batman faces the Joker, who seeks to plunge Gotham into anarchy.",
			},
			{
				Id:        "batman-animated",
				Title:     "Batman: The Animated Series",
				Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
				Year:      1992,
				PosterUrl: "https://images.vessel.local/posters/batman-animated.jpg",
				Overview:  "The Dark Knight battles crime in Gotham City with occasional help from Robin and Batgirl.",
			},
		}
	} else if q != "" {
		items = []*pluginv1.MediaItem{
			{
				Id:        "generic-" + q,
				Title:     "Result for " + req.Query,
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2024,
				PosterUrl: "https://images.vessel.local/posters/placeholder.jpg",
				Overview:  "Mock search result.",
			},
		}
	}

	return &pluginv1.SearchResponse{
		Items:   items,
		HasMore: false,
	}, nil
}

func (p *MockCinemaPlugin) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	switch req.MediaId {
	case "batman-animated":
		return &pluginv1.GetMetadataResponse{
			Details: &pluginv1.MediaDetails{
				Id:        "batman-animated",
				Title:     "Batman: The Animated Series",
				Type:      pluginv1.MediaType_MEDIA_TYPE_SERIES,
				Year:      1992,
				PosterUrl: "https://images.vessel.local/posters/batman-animated.jpg",
				Overview:  "The Dark Knight battles crime in Gotham City.",
				Genres:    []string{"Animation", "Action", "Crime"},
				Seasons: []*pluginv1.Season{
					{
						SeasonNumber: 1,
						Title:        "Season 1",
						Episodes: []*pluginv1.Episode{
							{
								EpisodeNumber:   1,
								Title:           "On Leather Wings",
								Overview:        "A bat-like creature terrorizes Gotham.",
								DurationSeconds: 1320,
							},
							{
								EpisodeNumber:   2,
								Title:           "Christmas with the Joker",
								Overview:        "The Joker escapes Arkham Asylum during Christmas Eve.",
								DurationSeconds: 1320,
							},
						},
					},
				},
			},
		}, nil

	case "batman-begins":
		return &pluginv1.GetMetadataResponse{
			Details: &pluginv1.MediaDetails{
				Id:        "batman-begins",
				Title:     "Batman Begins",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2005,
				PosterUrl: "https://images.vessel.local/posters/batman-begins.jpg",
				Overview:  "Bruce Wayne trains with the League of Shadows before protecting Gotham City.",
				Genres:    []string{"Action", "Crime", "Drama"},
			},
		}, nil

	default:
		return &pluginv1.GetMetadataResponse{
			Details: &pluginv1.MediaDetails{
				Id:        "the-dark-knight",
				Title:     "The Dark Knight",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2008,
				PosterUrl: "https://images.vessel.local/posters/the-dark-knight.jpg",
				Overview:  "When the menace known as the Joker wreaks havoc and chaos on the people of Gotham, Batman must accept one of the greatest psychological and physical tests of his ability to fight injustice.",
				Genres:    []string{"Action", "Crime", "Drama"},
			},
		}, nil
	}
}

func (p *MockCinemaPlugin) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{
			{
				Id:      "hls-1080p",
				Title:   "HLS 1080p Master",
				Url:     "https://mock.stream.vessel.local/hls/master.m3u8",
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_HLS,
				Quality: "1080p",
				Headers: map[string]string{
					"User-Agent": "Vessel/1.0",
					"Referer":    "https://mock.cinema.provider",
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

func main() {
	port := flag.Int("port", 50051, "Port for gRPC server")
	flag.Parse()

	addr := fmt.Sprintf("127.0.0.1:%d", *port)
	listener, err := net.Listen("tcp", addr)
	if err != nil {
		log.Fatalf("failed to listen on %s: %v", addr, err)
	}

	server := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(server, &MockCinemaPlugin{})

	log.Printf("Mock Cinema Plugin running on %s", addr)
	if err := server.Serve(listener); err != nil {
		log.Fatalf("server error: %v", err)
	}
}
