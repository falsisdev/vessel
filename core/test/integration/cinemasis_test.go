package integration_test

import (
	"context"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/cinema"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func buildCinemasisBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cinemasis")
	cmd := exec.Command("go", "build", "-o", binPath, "../../../plugins/cinemasis")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build cinemasis plugin: %v, output: %s", err, string(out))
	}
	return binPath
}

func TestCinemasisIntegrationWithCore(t *testing.T) {
	// Setup mock TMDB HTTP API
	mockTMDB := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		switch r.URL.Path {
		case "/search/multi":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"page": 1,
				"results": [
					{
						"id": 155,
						"media_type": "movie",
						"title": "The Dark Knight",
						"release_date": "2008-07-16",
						"poster_path": "/darkknight.jpg",
						"genre_ids": [18, 28, 80],
						"original_language": "en"
					}
				],
				"total_pages": 1,
				"total_results": 1
			}`))
		case "/movie/155":
			w.WriteHeader(http.StatusOK)
			_, _ = w.Write([]byte(`{
				"id": 155,
				"title": "The Dark Knight",
				"release_date": "2008-07-16",
				"poster_path": "/darkknight.jpg",
				"overview": "Batman battles the Joker...",
				"genres": [{"id": 18, "name": "Drama"}, {"id": 28, "name": "Action"}],
				"external_ids": {"imdb_id": "tt0468569"}
			}`))
		default:
			w.WriteHeader(http.StatusNotFound)
		}
	}))
	defer mockTMDB.Close()

	binPath := buildCinemasisBinary(t)

	mgr := plugin.NewManager()
	supervisor := plugin.NewSupervisor(mgr)
	defer func() { _ = supervisor.Shutdown(2 * time.Second) }()

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	// Launch Cinemasis as a managed subprocess pointing to mock TMDB
	cfg := plugin.ProcessConfig{
		ID:             "com.vessel.cinema.cinemasis",
		ExecutablePath: binPath,
		Args:           []string{"-tmdb-api-key", "test-key", "-tmdb-base-url", mockTMDB.URL},
		Env:            []string{"PATH=" + filepath.Dir(binPath)},
	}

	client, err := supervisor.Launch(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to launch cinemasis via supervisor: %v", err)
	}

	manifest := client.Manifest()
	if manifest.Id != "com.vessel.cinema.cinemasis" {
		t.Fatalf("unexpected manifest id: %s", manifest.Id)
	}
	if !manifest.IsBuiltin {
		t.Fatal("expected Cinemasis to have IsBuiltin = true")
	}

	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)

	// Verify plugin is registered in manager
	plugins := mgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_CINEMA, pluginv1.Capability_CAPABILITY_SEARCH)
	if len(plugins) != 1 {
		t.Fatalf("expected 1 cinema search plugin, got %d", len(plugins))
	}

	// Verify CinemaService Search
	results, err := cinemaSvc.Search(ctx, "Dark Knight")
	if err != nil {
		t.Fatalf("search error: %v", err)
	}
	if len(results) != 1 {
		t.Fatalf("expected 1 search result, got %d", len(results))
	}
	if results[0].Title != "The Dark Knight" {
		t.Errorf("expected The Dark Knight, got %s", results[0].Title)
	}
	if results[0].Type != cinema.MediaTypeMovie {
		t.Errorf("expected MediaTypeMovie, got %v", results[0].Type)
	}

	// Verify CinemaService GetMetadata and ExternalIDs
	details, err := cinemaSvc.GetMetadata(ctx, "com.vessel.cinema.cinemasis", results[0].ID)
	if err != nil {
		t.Fatalf("metadata error: %v", err)
	}
	if details.ExternalIDs.IMDbID != "tt0468569" {
		t.Errorf("expected IMDb ID tt0468569, got %s", details.ExternalIDs.IMDbID)
	}
	if details.ExternalIDs.TMDBID != "155" {
		t.Errorf("expected TMDB ID 155, got %s", details.ExternalIDs.TMDBID)
	}
}
