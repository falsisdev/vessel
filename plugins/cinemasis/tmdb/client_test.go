package tmdb_test

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/falsisdev/vessel/plugins/cinemasis/tmdb"
)

func TestTMDBClientSearch(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/search/multi" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		if r.URL.Query().Get("query") != "batman" {
			t.Errorf("unexpected query: %s", r.URL.Query().Get("query"))
		}
		if r.URL.Query().Get("api_key") != "test-key" {
			t.Errorf("unexpected api key: %s", r.URL.Query().Get("api_key"))
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{
			"page": 1,
			"results": [
				{
					"id": 272,
					"media_type": "movie",
					"title": "Batman Begins",
					"overview": "Driven by tragedy...",
					"poster_path": "/batman.jpg",
					"release_date": "2005-06-10",
					"genre_ids": [28, 80, 18],
					"original_language": "en"
				},
				{
					"id": 2098,
					"media_type": "tv",
					"name": "Batman: The Animated Series",
					"overview": "The Dark Knight fights crime...",
					"poster_path": "/animated.jpg",
					"first_air_date": "1992-09-05",
					"genre_ids": [16, 28, 80],
					"original_language": "en"
				}
			],
			"total_pages": 1,
			"total_results": 2
		}`))
	}))
	defer server.Close()

	client := tmdb.NewClient("test-key", tmdb.WithBaseURL(server.URL))
	resp, err := client.Search(context.Background(), "batman", 1)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(resp.Results) != 2 {
		t.Fatalf("expected 2 results, got %d", len(resp.Results))
	}
	if resp.Results[0].Title != "Batman Begins" {
		t.Errorf("expected Batman Begins, got %s", resp.Results[0].Title)
	}
	if client.BuildPosterURL(resp.Results[0].PosterPath) != "https://image.tmdb.org/t/p/w500/batman.jpg" {
		t.Errorf("unexpected poster URL: %s", client.BuildPosterURL(resp.Results[0].PosterPath))
	}
}

func TestTMDBClientGetMovieDetails(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/movie/272" {
			t.Errorf("unexpected path: %s", r.URL.Path)
		}
		if r.URL.Query().Get("append_to_response") != "external_ids" {
			t.Errorf("expected append_to_response=external_ids")
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(`{
			"id": 272,
			"title": "Batman Begins",
			"overview": "Driven by tragedy...",
			"poster_path": "/batman.jpg",
			"release_date": "2005-06-10",
			"genres": [{"id": 28, "name": "Action"}, {"id": 80, "name": "Crime"}],
			"external_ids": {
				"imdb_id": "tt0372784",
				"wikidata_id": "Q166723"
			}
		}`))
	}))
	defer server.Close()

	client := tmdb.NewClient("test-key", tmdb.WithBaseURL(server.URL))
	movie, err := client.GetMovie(context.Background(), 272)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if movie.Title != "Batman Begins" {
		t.Errorf("expected Batman Begins, got %s", movie.Title)
	}
	if movie.ExternalIDs.IMDbID != "tt0372784" {
		t.Errorf("expected IMDb ID tt0372784, got %s", movie.ExternalIDs.IMDbID)
	}
	if len(movie.Genres) != 2 {
		t.Errorf("expected 2 genres, got %d", len(movie.Genres))
	}
}

func TestTMDBClientGetTVAndSeason(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch r.URL.Path {
		case "/tv/2098":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 2098,
				"name": "Batman: The Animated Series",
				"overview": "The Dark Knight...",
				"poster_path": "/animated.jpg",
				"first_air_date": "1992-09-05",
				"original_language": "en",
				"genres": [{"id": 16, "name": "Animation"}],
				"seasons": [
					{"id": 100, "season_number": 1, "name": "Season 1", "episode_count": 2}
				],
				"external_ids": {
					"imdb_id": "tt0103359",
					"tvdb_id": 76168
				}
			}`))
		case "/tv/2098/season/1":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 100,
				"season_number": 1,
				"name": "Season 1",
				"episodes": [
					{"id": 1, "episode_number": 1, "name": "On Leather Wings", "overview": "A bat terrorizes...", "runtime": 22},
					{"id": 2, "episode_number": 2, "name": "Christmas with the Joker", "overview": "Joker escapes...", "runtime": 22}
				]
			}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	defer server.Close()

	client := tmdb.NewClient("test-key", tmdb.WithBaseURL(server.URL))
	tv, err := client.GetTV(context.Background(), 2098)
	if err != nil {
		t.Fatalf("unexpected TV error: %v", err)
	}
	if tv.ExternalIDs.IMDbID != "tt0103359" {
		t.Errorf("expected tt0103359, got %s", tv.ExternalIDs.IMDbID)
	}

	season, err := client.GetTVSeason(context.Background(), 2098, 1)
	if err != nil {
		t.Fatalf("unexpected Season error: %v", err)
	}
	if len(season.Episodes) != 2 {
		t.Fatalf("expected 2 episodes, got %d", len(season.Episodes))
	}
	if season.Episodes[0].Name != "On Leather Wings" {
		t.Errorf("unexpected episode name: %s", season.Episodes[0].Name)
	}
}
