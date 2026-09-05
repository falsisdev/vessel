package provider_test

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/falsisdev/vessel/plugins/cinemasis/provider"
	"github.com/falsisdev/vessel/plugins/cinemasis/tmdb"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func setupMockServer(t *testing.T) (*httptest.Server, *provider.CinemasisService) {
	t.Helper()

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch r.URL.Path {
		case "/search/multi":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"page": 1,
				"results": [
					{
						"id": 272,
						"media_type": "movie",
						"title": "Batman Begins",
						"release_date": "2005-06-10",
						"poster_path": "/batman.jpg",
						"genre_ids": [28],
						"original_language": "en"
					},
					{
						"id": 85937,
						"media_type": "tv",
						"name": "Demon Slayer",
						"first_air_date": "2019-04-06",
						"poster_path": "/demonslayer.jpg",
						"genre_ids": [16, 28],
						"original_language": "ja"
					}
				],
				"total_pages": 1,
				"total_results": 2
			}`))
		case "/movie/272":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 272,
				"title": "Batman Begins",
				"release_date": "2005-06-10",
				"poster_path": "/batman.jpg",
				"overview": "Batman begins...",
				"genres": [{"id": 28, "name": "Action"}],
				"external_ids": {"imdb_id": "tt0372784"}
			}`))
		case "/tv/85937":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 85937,
				"name": "Demon Slayer",
				"first_air_date": "2019-04-06",
				"poster_path": "/demonslayer.jpg",
				"overview": "Tanjiro fights demons...",
				"original_language": "ja",
				"genres": [{"id": 16, "name": "Animation"}, {"id": 28, "name": "Action"}],
				"seasons": [
					{"id": 1, "season_number": 1, "name": "Season 1", "episode_count": 26}
				],
				"external_ids": {"imdb_id": "tt9335498"}
			}`))
		case "/tv/85937/season/1":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 1,
				"season_number": 1,
				"name": "Season 1",
				"episodes": [
					{"id": 1, "episode_number": 1, "name": "Cruelty", "overview": "Family attacked...", "runtime": 24}
				]
			}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))

	client := tmdb.NewClient("test-key", tmdb.WithBaseURL(server.URL))
	svc := provider.NewCinemasisService(client)
	return server, svc
}

func TestCinemasisManifest(t *testing.T) {
	_, svc := setupMockServer(t)

	resp, err := svc.GetManifest(context.Background(), &pluginv1.GetManifestRequest{})
	if err != nil {
		t.Fatalf("manifest error: %v", err)
	}

	manifest := resp.Manifest
	if manifest.Id != "com.vessel.cinema.cinemasis" {
		t.Errorf("unexpected id: %s", manifest.Id)
	}
	if !manifest.IsBuiltin {
		t.Error("expected IsBuiltin to be true")
	}
	if manifest.Domain != pluginv1.Domain_DOMAIN_CINEMA {
		t.Errorf("expected cinema domain, got %v", manifest.Domain)
	}
}

func TestCinemasisSearch(t *testing.T) {
	server, svc := setupMockServer(t)
	defer server.Close()

	resp, err := svc.Search(context.Background(), &pluginv1.SearchRequest{Query: "test", Page: 1})
	if err != nil {
		t.Fatalf("search error: %v", err)
	}

	if len(resp.Items) != 2 {
		t.Fatalf("expected 2 items, got %d", len(resp.Items))
	}

	movie := resp.Items[0]
	if movie.Id != "movie:272" || movie.Type != pluginv1.MediaType_MEDIA_TYPE_MOVIE {
		t.Errorf("unexpected movie mapping: %+v", movie)
	}

	anime := resp.Items[1]
	if anime.Id != "tv:85937" || anime.Type != pluginv1.MediaType_MEDIA_TYPE_ANIME {
		t.Errorf("expected anime classification, got: %+v", anime)
	}
}

func TestCinemasisGetMetadata(t *testing.T) {
	server, svc := setupMockServer(t)
	defer server.Close()

	// Movie details
	mResp, err := svc.GetMetadata(context.Background(), &pluginv1.GetMetadataRequest{MediaId: "movie:272"})
	if err != nil {
		t.Fatalf("movie metadata error: %v", err)
	}
	if mResp.Details.ExternalIds.ImdbId != "tt0372784" {
		t.Errorf("expected IMDb ID tt0372784, got %s", mResp.Details.ExternalIds.ImdbId)
	}

	// TV/Anime details with seasons & episodes
	tvResp, err := svc.GetMetadata(context.Background(), &pluginv1.GetMetadataRequest{MediaId: "tv:85937"})
	if err != nil {
		t.Fatalf("tv metadata error: %v", err)
	}
	if tvResp.Details.Type != pluginv1.MediaType_MEDIA_TYPE_ANIME {
		t.Errorf("expected anime type, got %v", tvResp.Details.Type)
	}
	if len(tvResp.Details.Seasons) != 1 || len(tvResp.Details.Seasons[0].Episodes) != 1 {
		t.Fatalf("unexpected seasons/episodes: %+v", tvResp.Details.Seasons)
	}
	if tvResp.Details.ExternalIds.ImdbId != "tt9335498" {
		t.Errorf("expected IMDb ID tt9335498, got %s", tvResp.Details.ExternalIds.ImdbId)
	}

	// Streams (should be empty for catalog provider)
	streamResp, err := svc.GetStreams(context.Background(), &pluginv1.GetStreamsRequest{MediaId: "movie:272"})
	if err != nil {
		t.Fatalf("get streams error: %v", err)
	}
	if len(streamResp.Streams) != 0 {
		t.Errorf("expected 0 streams from catalog provider, got %d", len(streamResp.Streams))
	}
}
