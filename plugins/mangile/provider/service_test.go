package provider_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/falsisdev/vessel/plugins/mangile/provider"
	"github.com/falsisdev/vessel/plugins/mangile/sanity"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func TestMangileService_GetManifest(t *testing.T) {
	client := sanity.NewClient("test-p", "")
	svc := provider.NewMangileService(client)

	resp, err := svc.GetManifest(context.Background(), &pluginv1.GetManifestRequest{})
	if err != nil {
		t.Fatalf("GetManifest error: %v", err)
	}

	m := resp.Manifest
	if m.Id != provider.PluginID || m.Name != "Mangile" || !m.IsBuiltin {
		t.Fatalf("unexpected manifest: %+v", m)
	}

	hasRead := false
	for _, cap := range m.Capabilities {
		if cap == pluginv1.Capability_CAPABILITY_READ {
			hasRead = true
			break
		}
	}
	if !hasRead {
		t.Error("expected manifest to declare CAPABILITY_READ")
	}
}

func TestMangileService_SearchAndMetadata(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query().Get("query")
		if q == "" {
			t.Error("missing query param")
		}

		// Search or GetTitle
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
						"_id":           "novel-tbate",
						"_type":         "lightNovel",
						"title":         "The Beginning After the End",
						"slug":          "tbate",
						"format":        "lightNovel",
						"coverImage":    "https://cdn.sanity.io/tbate.jpg",
						"myAnimeListId": 0,
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		if r.URL.Query().Get("$id") == "\"manga-solo\"" {
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
							"title":         "Chapter 1: The Weakest Hunter",
							"chapterNumber": 1.0,
							"volumeNumber":  1.0,
						},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		w.WriteHeader(http.StatusNotFound)
	}))
	defer ts.Close()

	client := sanity.NewClient("test-p", "", sanity.WithBaseURL(ts.URL))
	svc := provider.NewMangileService(client)
	ctx := context.Background()

	// 1. Search
	searchResp, err := svc.Search(ctx, &pluginv1.SearchRequest{
		Query: "Solo",
		Page:  1,
	})
	if err != nil {
		t.Fatalf("Search failed: %v", err)
	}
	if len(searchResp.Items) != 2 {
		t.Fatalf("expected 2 items, got %d", len(searchResp.Items))
	}
	if searchResp.Items[0].Type != pluginv1.MediaType_MEDIA_TYPE_WEBTOON {
		t.Errorf("expected WEBTOON type for Solo Leveling, got %v", searchResp.Items[0].Type)
	}
	if searchResp.Items[1].Type != pluginv1.MediaType_MEDIA_TYPE_WEBOOK {
		t.Errorf("expected WEBOOK type for TBATE, got %v", searchResp.Items[1].Type)
	}

	// 2. Metadata
	metaResp, err := svc.GetMetadata(ctx, &pluginv1.GetMetadataRequest{
		MediaId: "manga-solo",
	})
	if err != nil {
		t.Fatalf("GetMetadata failed: %v", err)
	}
	if metaResp.Details.Title != "Solo Leveling" {
		t.Errorf("expected Solo Leveling, got %s", metaResp.Details.Title)
	}
	if len(metaResp.Details.Seasons) != 1 || len(metaResp.Details.Seasons[0].Episodes) != 1 {
		t.Errorf("unexpected episodes in metadata: %+v", metaResp.Details.Seasons)
	}
}

func TestMangileService_GetChapterContent(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		idParam := r.URL.Query().Get("$id")
		if idParam == "\"ch-manga-1\"" {
			resp := map[string]any{
				"result": map[string]any{
					"_id":           "ch-manga-1",
					"_type":         "mangaChapter",
					"title":         "Chapter 1",
					"chapterNumber": 1.0,
					"pages": []map[string]any{
						{"url": "https://cdn.sanity.io/img1.jpg"},
						{"url": "https://cdn.sanity.io/img2.jpg"},
					},
				},
			}
			_ = json.NewEncoder(w).Encode(resp)
			return
		}

		if idParam == "\"ch-novel-1\"" {
			resp := map[string]any{
				"result": map[string]any{
					"_id":           "ch-novel-1",
					"_type":         "novelChapter",
					"title":         "Chapter 1: The Awakening",
					"chapterNumber": 1.0,
					"content": []map[string]any{
						{
							"_type": "block",
							"style": "normal",
							"children": []map[string]any{
								{"_type": "span", "text": "Arthur Leywin opened his eyes."},
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
	defer ts.Close()

	client := sanity.NewClient("test-p", "", sanity.WithBaseURL(ts.URL))
	svc := provider.NewMangileService(client)
	ctx := context.Background()

	// 1. Manga Pages
	mangaContent, err := svc.GetChapterContent(ctx, &pluginv1.GetChapterContentRequest{
		ChapterId: "ch-manga-1",
	})
	if err != nil {
		t.Fatalf("GetChapterContent manga failed: %v", err)
	}
	if len(mangaContent.Pages) != 2 {
		t.Fatalf("expected 2 pages, got %d", len(mangaContent.Pages))
	}
	if mangaContent.Pages[0].Url != "https://cdn.sanity.io/img1.jpg" {
		t.Errorf("unexpected page URL: %s", mangaContent.Pages[0].Url)
	}

	// 2. Novel Text Content
	novelContent, err := svc.GetChapterContent(ctx, &pluginv1.GetChapterContentRequest{
		ChapterId: "ch-novel-1",
	})
	if err != nil {
		t.Fatalf("GetChapterContent novel failed: %v", err)
	}
	if novelContent.TextContent != "Arthur Leywin opened his eyes." {
		t.Errorf("unexpected novel content: %q", novelContent.TextContent)
	}
}
