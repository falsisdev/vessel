package sanity_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/falsisdev/vessel/plugins/mangile/sanity"
)

func TestSanityClient_SearchTitles(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Authorization") != "Bearer test-token" {
			t.Errorf("expected Bearer test-token, got %s", r.Header.Get("Authorization"))
		}
		if r.URL.Query().Get("perspective") != "drafts" {
			t.Errorf("expected perspective=drafts, got %s", r.URL.Query().Get("perspective"))
		}

		resp := map[string]any{
			"result": []map[string]any{
				{
					"_id":           "manga-1",
					"_type":         "manga",
					"title":         "Chainsaw Man",
					"slug":          "chainsaw-man",
					"myAnimeListId": 114791,
					"format":        "manga",
					"coverImage":    "https://cdn.sanity.io/cover1.jpg",
				},
				{
					"_id":           "novel-1",
					"_type":         "lightNovel",
					"title":         "Mushoku Tensei",
					"slug":          "mushoku-tensei",
					"myAnimeListId": 70261,
					"format":        "lightNovel",
					"coverImage":    "https://cdn.sanity.io/cover2.jpg",
				},
			},
		}
		_ = json.NewEncoder(w).Encode(resp)
	}))
	defer ts.Close()

	client := sanity.NewClient("test-project", "test-token", sanity.WithBaseURL(ts.URL))
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	titles, err := client.SearchTitles(ctx, "Chainsaw", 10)
	if err != nil {
		t.Fatalf("SearchTitles failed: %v", err)
	}

	if len(titles) != 2 {
		t.Fatalf("expected 2 titles, got %d", len(titles))
	}
	if titles[0].Title != "Chainsaw Man" || titles[0].Type != "manga" {
		t.Errorf("unexpected first title: %+v", titles[0])
	}
	if titles[1].Title != "Mushoku Tensei" || titles[1].Type != "lightNovel" {
		t.Errorf("unexpected second title: %+v", titles[1])
	}
}

func TestSanityClient_GetTitle(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := map[string]any{
			"result": map[string]any{
				"_id":           "manga-1",
				"_type":         "manga",
				"title":         "Chainsaw Man",
				"slug":          "chainsaw-man",
				"myAnimeListId": 114791,
				"format":        "manga",
				"coverImage":    "https://cdn.sanity.io/cover1.jpg",
				"chapters": []map[string]any{
					{
						"_id":           "ch-1",
						"title":         "Dog and Chainsaw",
						"chapterNumber": 1.0,
						"volumeNumber":  1.0,
					},
				},
			},
		}
		_ = json.NewEncoder(w).Encode(resp)
	}))
	defer ts.Close()

	client := sanity.NewClient("test-project", "", sanity.WithBaseURL(ts.URL))
	ctx := context.Background()

	title, err := client.GetTitle(ctx, "manga-1")
	if err != nil {
		t.Fatalf("GetTitle failed: %v", err)
	}

	if title.Title != "Chainsaw Man" {
		t.Errorf("expected Chainsaw Man, got %s", title.Title)
	}
	if len(title.Chapters) != 1 || title.Chapters[0].ChapterNumber != 1.0 {
		t.Errorf("unexpected chapters: %+v", title.Chapters)
	}
}

func TestSanityClient_GetChapter_Manga(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := map[string]any{
			"result": map[string]any{
				"_id":           "ch-1",
				"_type":         "mangaChapter",
				"title":         "Dog and Chainsaw",
				"chapterNumber": 1.0,
				"volumeNumber":  1.0,
				"pages": []map[string]any{
					{"url": "https://cdn.sanity.io/page-01.jpg"},
					{"url": "https://cdn.sanity.io/page-02.jpg"},
				},
			},
		}
		_ = json.NewEncoder(w).Encode(resp)
	}))
	defer ts.Close()

	client := sanity.NewClient("test-project", "", sanity.WithBaseURL(ts.URL))
	ctx := context.Background()

	ch, err := client.GetChapter(ctx, "ch-1")
	if err != nil {
		t.Fatalf("GetChapter failed: %v", err)
	}

	if len(ch.Pages) != 2 {
		t.Fatalf("expected 2 pages, got %d", len(ch.Pages))
	}
	if ch.Pages[0].URL != "https://cdn.sanity.io/page-01.jpg" {
		t.Errorf("unexpected page URL: %s", ch.Pages[0].URL)
	}
}

func TestSanityClient_GetChapter_Novel(t *testing.T) {
	ts := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		resp := map[string]any{
			"result": map[string]any{
				"_id":           "ln-ch-1",
				"_type":         "novelChapter",
				"title":         "Prologue",
				"chapterNumber": 1.0,
				"volumeNumber":  1.0,
				"content": []map[string]any{
					{
						"_type": "block",
						"style": "normal",
						"children": []map[string]any{
							{"_type": "span", "text": "It was a dark and stormy night."},
						},
					},
					{
						"_type": "block",
						"style": "normal",
						"children": []map[string]any{
							{"_type": "span", "text": "Suddenly, a reincarnation happened."},
						},
					},
				},
			},
		}
		_ = json.NewEncoder(w).Encode(resp)
	}))
	defer ts.Close()

	client := sanity.NewClient("test-project", "", sanity.WithBaseURL(ts.URL))
	ctx := context.Background()

	ch, err := client.GetChapter(ctx, "ln-ch-1")
	if err != nil {
		t.Fatalf("GetChapter failed: %v", err)
	}

	text := ch.ExtractTextContent()
	expected := "It was a dark and stormy night.\n\nSuddenly, a reincarnation happened."
	if text != expected {
		t.Errorf("expected %q, got %q", expected, text)
	}
}
