package sanity

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"sort"
	"strings"
	"time"
)

const (
	DefaultAPIVersion = "v2021-10-21"
	DefaultDataset    = "production"
)

var (
	ErrResourceNotFound = errors.New("sanity resource not found")
	ErrEmptyProjectID   = errors.New("sanity project id is required")
)

type Client struct {
	projectID  string
	dataset    string
	token      string
	apiVersion string
	baseURL    string
	httpClient *http.Client
}

type Option func(*Client)

func WithBaseURL(u string) Option {
	return func(c *Client) {
		c.baseURL = strings.TrimRight(u, "/")
	}
}

func WithHTTPClient(client *http.Client) Option {
	return func(c *Client) {
		c.httpClient = client
	}
}

func WithAPIVersion(ver string) Option {
	return func(c *Client) {
		c.apiVersion = ver
	}
}

func WithDataset(ds string) Option {
	return func(c *Client) {
		c.dataset = ds
	}
}

func NewClient(projectID, token string, opts ...Option) *Client {
	c := &Client{
		projectID:  projectID,
		dataset:    DefaultDataset,
		token:      token,
		apiVersion: DefaultAPIVersion,
		httpClient: &http.Client{Timeout: 15 * time.Second},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

func (c *Client) Query(ctx context.Context, groq string, params map[string]any, dst any) error {
	var endpoint string
	if c.baseURL != "" {
		endpoint = c.baseURL
	} else {
		if c.projectID == "" {
			return ErrEmptyProjectID
		}
		endpoint = fmt.Sprintf("https://%s.api.sanity.io/%s/data/query/%s", c.projectID, c.apiVersion, c.dataset)
	}

	values := url.Values{}
	values.Set("query", groq)
	if c.token != "" {
		values.Set("perspective", "drafts")
	} else {
		values.Set("perspective", "published")
	}
	for key, value := range params {
		encoded, err := json.Marshal(value)
		if err != nil {
			return fmt.Errorf("failed to encode param %s: %w", key, err)
		}
		values.Set("$"+key, string(encoded))
	}

	fullURL := endpoint + "?" + values.Encode()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fullURL, nil)
	if err != nil {
		return fmt.Errorf("failed to create sanity request: %w", err)
	}
	req.Header.Set("Accept", "application/json")
	if c.token != "" {
		req.Header.Set("Authorization", "Bearer "+c.token)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("sanity request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("sanity api error, status code: %d", resp.StatusCode)
	}

	var wrapper struct {
		Result json.RawMessage `json:"result"`
	}
	if err := json.NewDecoder(resp.Body).Decode(&wrapper); err != nil {
		return fmt.Errorf("failed to decode sanity response: %w", err)
	}

	if len(wrapper.Result) == 0 || string(wrapper.Result) == "null" {
		return ErrResourceNotFound
	}

	if err := json.Unmarshal(wrapper.Result, dst); err != nil {
		return fmt.Errorf("failed to unmarshal sanity result: %w", err)
	}

	return nil
}

func (c *Client) SearchTitles(ctx context.Context, search string, limit int) ([]SanityTitle, error) {
	if limit <= 0 {
		limit = 50
	}

	query := `*[(_type == "manga" || _type == "lightNovel") && ($search == "" || title match $search)] | order(_createdAt desc) [0...$limit] {
		_id,
		_type,
		title,
		"slug": slug.current,
		myAnimeListId,
		description,
		uploadStatus,
		tags,
		format,
		"coverImage": coverImage.asset->url,
		"bannerImage": bannerImage.asset->url
	}`

	var results []SanityTitle
	err := c.Query(ctx, query, map[string]any{
		"search": search,
		"limit":  limit,
	}, &results)
	if err != nil {
		if errors.Is(err, ErrResourceNotFound) {
			return []SanityTitle{}, nil
		}
		return nil, err
	}
	return results, nil
}

func (c *Client) GetTitle(ctx context.Context, id string) (*SanityTitle, error) {
	query := `*[(_type == "manga" || _type == "lightNovel") && (_id == $id || slug.current == $id)] [0] {
		_id,
		_type,
		title,
		"slug": slug.current,
		myAnimeListId,
		description,
		uploadStatus,
		tags,
		format,
		"coverImage": coverImage.asset->url,
		"bannerImage": bannerImage.asset->url,
		"embeddedChapters": chapters[] {
			"_id": coalesce(_id, _key),
			"_key": _key,
			title,
			chapterNumber,
			volumeNumber
		},
		"externalChapters": *[(_type in ["mangaChapter", "novelChapter", "lightNovelChapter"]) && (manga._ref == ^._id || lightNovel._ref == ^._id || (defined(manga->myAnimeListId) && defined(^.myAnimeListId) && manga->myAnimeListId == ^.myAnimeListId) || (defined(lightNovel->myAnimeListId) && defined(^.myAnimeListId) && lightNovel->myAnimeListId == ^.myAnimeListId))] | order(chapterNumber asc) {
			_id,
			title,
			chapterNumber,
			volumeNumber
		},
		chapters[] {
			"_id": coalesce(_id, _key),
			"_key": _key,
			title,
			chapterNumber,
			volumeNumber
		}
	}`

	var title SanityTitle
	err := c.Query(ctx, query, map[string]any{"id": id}, &title)
	if err != nil {
		return nil, err
	}
	if title.ID == "" {
		return nil, ErrResourceNotFound
	}

	// Merge and deduplicate chapters by chapterNumber
	chapterMap := make(map[float64]SanityChapterSummary)
	for _, ch := range title.ExternalChapters {
		chapterMap[ch.ChapterNumber] = ch
	}
	for _, ch := range title.EmbeddedChapters {
		chapterMap[ch.ChapterNumber] = ch
	}
	for _, ch := range title.Chapters {
		chapterMap[ch.ChapterNumber] = ch
	}

	var allChapters []SanityChapterSummary
	for _, ch := range chapterMap {
		if ch.ID == "" && ch.Key != "" {
			ch.ID = ch.Key
		}
		allChapters = append(allChapters, ch)
	}

	sort.Slice(allChapters, func(i, j int) bool {
		if allChapters[i].VolumeNumber != allChapters[j].VolumeNumber {
			return allChapters[i].VolumeNumber < allChapters[j].VolumeNumber
		}
		return allChapters[i].ChapterNumber < allChapters[j].ChapterNumber
	})

	title.Chapters = allChapters
	return &title, nil
}

func (c *Client) GetChapter(ctx context.Context, id string) (*SanityChapterDetails, error) {
	query := `*[(_type in ["mangaChapter", "novelChapter", "lightNovelChapter"]) && (_id == $id || _key == $id)] [0] {
		_id,
		_type,
		title,
		chapterNumber,
		volumeNumber,
		pages[] {
			"url": asset->url,
			"ref": asset._ref
		},
		content
	}`

	var chapter SanityChapterDetails
	err := c.Query(ctx, query, map[string]any{"id": id}, &chapter)
	if err != nil {
		return nil, err
	}
	if chapter.ID == "" && chapter.Key == "" {
		return nil, ErrResourceNotFound
	}
	if chapter.ID == "" {
		chapter.ID = chapter.Key
	}
	for i := range chapter.Pages {
		if chapter.Pages[i].URL == "" && chapter.Pages[i].Ref != "" {
			chapter.Pages[i].URL = chapter.Pages[i].ResolvedURL(c.projectID, c.dataset)
		}
	}
	return &chapter, nil
}

func (c *Client) GetChapterFromTitle(ctx context.Context, mediaID, chapterID string, chapterNumber float64) (*SanityChapterDetails, error) {
	var query string
	params := map[string]any{"mediaId": mediaID}

	if chapterID != "" {
		query = `*[_type in ["manga", "lightNovel"] && (_id == $mediaId || slug.current == $mediaId)][0].chapters[_key == $chapterId || _id == $chapterId][0] {
			"_id": coalesce(_id, _key),
			"_key": _key,
			_type,
			title,
			chapterNumber,
			volumeNumber,
			pages[] {
				"url": asset->url,
				"ref": asset._ref
			},
			content
		}`
		params["chapterId"] = chapterID
	} else {
		query = `*[_type in ["manga", "lightNovel"] && (_id == $mediaId || slug.current == $mediaId)][0].chapters[chapterNumber == $chapterNum][0] {
			"_id": coalesce(_id, _key),
			"_key": _key,
			_type,
			title,
			chapterNumber,
			volumeNumber,
			pages[] {
				"url": asset->url,
				"ref": asset._ref
			},
			content
		}`
		params["chapterNum"] = chapterNumber
	}

	var chapter SanityChapterDetails
	err := c.Query(ctx, query, params, &chapter)
	if err != nil {
		return nil, err
	}
	if chapter.ID == "" && chapter.Key == "" {
		return nil, ErrResourceNotFound
	}
	if chapter.ID == "" {
		chapter.ID = chapter.Key
	}
	for i := range chapter.Pages {
		if chapter.Pages[i].URL == "" && chapter.Pages[i].Ref != "" {
			chapter.Pages[i].URL = chapter.Pages[i].ResolvedURL(c.projectID, c.dataset)
		}
	}
	return &chapter, nil
}
