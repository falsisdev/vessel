package integration_test

import (
	"context"
	"encoding/json"
	"net"
	"net/http"
	"net/http/httptest"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	coreclient "github.com/falsisdev/vessel/core/internal/client"
	"github.com/falsisdev/vessel/core/internal/domain/reading"
	"github.com/falsisdev/vessel/core/internal/plugin"
	coreserver "github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/theme"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func buildMangileBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "mangile")
	cmd := exec.Command("go", "build", "-o", binPath, "../../../plugins/mangile")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build mangile plugin: %v, output: %s", err, string(out))
	}
	return binPath
}

func TestMangileIntegrationWithCoreAndIPC(t *testing.T) {
	// Setup mock Sanity GROQ API
	mockSanity := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")

		// 1. Search Query
		if r.URL.Query().Get("$search") != "" {
			resp := map[string]any{
				"result": []map[string]any{
					{
						"_id":           "manga-solo",
						"_type":         "manga",
						"title":         "Solo Leveling",
						"slug":          "solo-leveling",
						"format":        "webtoon",
						"coverImage":    "https://cdn.sanity.io/solo.jpg",
						"myAnimeListId": 121496,
					},
					{
						"_id":           "novel-mt",
						"_type":         "lightNovel",
						"title":         "Mushoku Tensei",
						"slug":          "mushoku-tensei",
						"format":        "lightNovel",
						"coverImage":    "https://cdn.sanity.io/mt.jpg",
						"myAnimeListId": 70261,
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		// 2. Title Details Query
		idParam := r.URL.Query().Get("$id")
		if idParam == "\"manga-solo\"" {
			resp := map[string]any{
				"result": map[string]any{
					"_id":           "manga-solo",
					"_type":         "manga",
					"title":         "Solo Leveling",
					"slug":          "solo-leveling",
					"format":        "webtoon",
					"coverImage":    "https://cdn.sanity.io/solo.jpg",
					"myAnimeListId": 121496,
					"chapters": []map[string]any{
						{
							"_id":           "ch-solo-1",
							"title":         "Chapter 1",
							"chapterNumber": 1.0,
							"volumeNumber":  1.0,
						},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		// 3. Chapter Content Queries
		if idParam == "\"ch-solo-1\"" {
			resp := map[string]any{
				"result": map[string]any{
					"_id":           "ch-solo-1",
					"_type":         "mangaChapter",
					"title":         "Chapter 1",
					"chapterNumber": 1.0,
					"volumeNumber":  1.0,
					"pages": []map[string]any{
						{"url": "https://cdn.sanity.io/solo-p1.jpg"},
						{"url": "https://cdn.sanity.io/solo-p2.jpg"},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		if idParam == "\"ch-mt-1\"" {
			resp := map[string]any{
				"result": map[string]any{
					"_id":           "ch-mt-1",
					"_type":         "novelChapter",
					"title":         "Prologue: Rebirth",
					"chapterNumber": 1.0,
					"volumeNumber":  1.0,
					"content": []map[string]any{
						{
							"_type": "block",
							"style": "normal",
							"children": []map[string]any{
								{"_type": "span", "text": "I woke up in an unfamiliar room."},
							},
						},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		w.WriteHeader(http.StatusNotFound)
	}))
	defer mockSanity.Close()

	binPath := buildMangileBinary(t)

	mgr := plugin.NewManager()
	supervisor := plugin.NewSupervisor(mgr)
	defer func() { _ = supervisor.Shutdown(2 * time.Second) }()

	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	// Launch Mangile subprocess
	cfg := plugin.ProcessConfig{
		ID:             "com.vessel.reading.mangile",
		ExecutablePath: binPath,
		Args: []string{
			"-sanity-project-id", "test-project",
			"-sanity-base-url", mockSanity.URL,
		},
		Env: []string{"PATH=" + filepath.Dir(binPath)},
	}

	client, err := supervisor.Launch(ctx, cfg)
	if err != nil {
		t.Fatalf("failed to launch mangile via supervisor: %v", err)
	}

	manifest := client.Manifest()
	if manifest.Id != "com.vessel.reading.mangile" {
		t.Fatalf("unexpected manifest id: %s", manifest.Id)
	}
	if !manifest.IsBuiltin {
		t.Fatal("expected Mangile to have IsBuiltin = true")
	}

	readingSvc := service.NewReadingService(mgr, 3*time.Second)
	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)

	// Verify reading service directly
	readingPlugins := mgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_MANGA, pluginv1.Capability_CAPABILITY_READ)
	if len(readingPlugins) != 1 {
		t.Fatalf("expected 1 reading plugin, got %d", len(readingPlugins))
	}

	// 1. Direct ReadingService search
	searchItems, err := readingSvc.Search(ctx, pluginv1.Domain_DOMAIN_MANGA, "Solo")
	if err != nil {
		t.Fatalf("reading search failed: %v", err)
	}
	if len(searchItems) != 2 {
		t.Fatalf("expected 2 items, got %d", len(searchItems))
	}
	if searchItems[0].Type != reading.ReadingTypeWebtoon {
		t.Errorf("expected Solo Leveling to be Webtoon, got %v", searchItems[0].Type)
	}
	if searchItems[1].Type != reading.ReadingTypeWebook {
		t.Errorf("expected Mushoku Tensei to be Webook, got %v", searchItems[1].Type)
	}

	// 2. Setup Core IPC Server over ephemeral TCP port
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on ephemeral port: %v", err)
	}
	tcpAddr := l.Addr().String()
	_ = l.Close()

	themeMgr := theme.NewManager()
	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: tcpAddr,
		Version:    "1.0.0-mangile-test",
	}, cinemaSvc, readingSvc, nil, nil, mgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start core IPC server: %v", err)
	}
	defer srv.Stop()

	// 3. Connect Core Client to Core IPC
	ipcClient, err := coreclient.Dial(ctx, tcpAddr)
	if err != nil {
		t.Fatalf("failed to dial core IPC: %v", err)
	}
	defer func() { _ = ipcClient.Close() }()

	// Verify Client Ping
	pingResp, err := ipcClient.Ping(ctx)
	if err != nil || pingResp.Status != "OK" {
		t.Fatalf("ping failed: %v", err)
	}

	// Verify Client ListPlugins
	pluginsList, err := ipcClient.ListPlugins(ctx)
	if err != nil || len(pluginsList.Plugins) != 1 {
		t.Fatalf("list plugins failed: %v, count: %d", err, len(pluginsList.Plugins))
	}
	if pluginsList.Plugins[0].Id != "com.vessel.reading.mangile" || !pluginsList.Plugins[0].IsBuiltin {
		t.Fatalf("unexpected plugin info: %+v", pluginsList.Plugins[0])
	}

	// Verify Client SearchMedia (Reading domain)
	mediaSearchResp, err := ipcClient.SearchMedia(ctx, pluginv1.Domain_DOMAIN_MANGA, "Solo", 1)
	if err != nil {
		t.Fatalf("search media failed: %v", err)
	}
	if len(mediaSearchResp.Items) != 2 {
		t.Fatalf("expected 2 items, got %d", len(mediaSearchResp.Items))
	}

	// Verify Client GetMediaDetails
	mediaDetails, err := ipcClient.GetMediaDetails(ctx, pluginv1.Domain_DOMAIN_MANGA, "com.vessel.reading.mangile", "manga-solo")
	if err != nil {
		t.Fatalf("get media details failed: %v", err)
	}
	if mediaDetails.Title != "Solo Leveling" {
		t.Errorf("expected Solo Leveling, got %s", mediaDetails.Title)
	}
	if mediaDetails.ExternalIds.SanityId != "manga-solo" || mediaDetails.ExternalIds.MalId != "121496" {
		t.Errorf("unexpected external IDs: %+v", mediaDetails.ExternalIds)
	}

	// Verify Client GetChapterContent (Manga Pages)
	mangaContent, err := ipcClient.GetChapterContent(ctx, "com.vessel.reading.mangile", "manga-solo", "ch-solo-1", 1.0)
	if err != nil {
		t.Fatalf("get manga chapter content failed: %v", err)
	}
	if len(mangaContent.Pages) != 2 {
		t.Fatalf("expected 2 pages, got %d", len(mangaContent.Pages))
	}
	if mangaContent.Pages[0].Url != "https://cdn.sanity.io/solo-p1.jpg" {
		t.Errorf("unexpected page 0 url: %s", mangaContent.Pages[0].Url)
	}

	// Verify Client GetChapterContent (Webook Text Content)
	novelContent, err := ipcClient.GetChapterContent(ctx, "com.vessel.reading.mangile", "novel-mt", "ch-mt-1", 1.0)
	if err != nil {
		t.Fatalf("get novel chapter content failed: %v", err)
	}
	if novelContent.TextContent != "I woke up in an unfamiliar room." {
		t.Errorf("unexpected text content: %q", novelContent.TextContent)
	}
}
