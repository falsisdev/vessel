package tmdb

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"
)

const (
	DefaultBaseURL   = "https://api.themoviedb.org/3"
	DefaultPosterURL = "https://image.tmdb.org/t/p/w500"
)

var (
	ErrEmptyAPIKey   = errors.New("tmdb api key is required")
	ErrNotFound      = errors.New("tmdb resource not found")
	ErrRateLimited   = errors.New("tmdb rate limited")
	ErrInvalidStatus = errors.New("unexpected tmdb response status")
)

type MultiSearchResult struct {
	ID               int    `json:"id"`
	MediaType        string `json:"media_type"`
	Title            string `json:"title,omitempty"`
	Name             string `json:"name,omitempty"`
	Overview         string `json:"overview"`
	PosterPath       string `json:"poster_path"`
	ReleaseDate      string `json:"release_date,omitempty"`
	FirstAirDate     string `json:"first_air_date,omitempty"`
	OriginalLanguage string `json:"original_language"`
	GenreIDs         []int  `json:"genre_ids"`
}

type MultiSearchResponse struct {
	Page         int                 `json:"page"`
	Results      []MultiSearchResult `json:"results"`
	TotalPages   int                 `json:"total_pages"`
	TotalResults int                 `json:"total_results"`
}

type Genre struct {
	ID   int    `json:"id"`
	Name string `json:"name"`
}

type ExternalIDs struct {
	IMDbID     string `json:"imdb_id"`
	WikidataID string `json:"wikidata_id"`
	TVDBID     int    `json:"tvdb_id"`
}

type MovieDetails struct {
	ID               int         `json:"id"`
	Title            string      `json:"title"`
	Overview         string      `json:"overview"`
	PosterPath       string      `json:"poster_path"`
	ReleaseDate      string      `json:"release_date"`
	OriginalLanguage string      `json:"original_language"`
	Genres           []Genre     `json:"genres"`
	ExternalIDs      ExternalIDs `json:"external_ids"`
}

type TVSeasonOverview struct {
	ID           int    `json:"id"`
	SeasonNumber int    `json:"season_number"`
	Name         string `json:"name"`
	EpisodeCount int    `json:"episode_count"`
}

type TVDetails struct {
	ID               int                `json:"id"`
	Name             string             `json:"name"`
	Overview         string             `json:"overview"`
	PosterPath       string             `json:"poster_path"`
	FirstAirDate     string             `json:"first_air_date"`
	OriginalLanguage string             `json:"original_language"`
	Genres           []Genre            `json:"genres"`
	Seasons          []TVSeasonOverview `json:"seasons"`
	ExternalIDs      ExternalIDs        `json:"external_ids"`
}

type TVEpisode struct {
	ID            int    `json:"id"`
	EpisodeNumber int    `json:"episode_number"`
	Name          string `json:"name"`
	Overview      string `json:"overview"`
	Runtime       int    `json:"runtime"`
}

type TVSeasonDetails struct {
	ID           int         `json:"id"`
	SeasonNumber int         `json:"season_number"`
	Name         string      `json:"name"`
	Episodes     []TVEpisode `json:"episodes"`
}

type Client struct {
	apiKey     string
	baseURL    string
	httpClient *http.Client
}

type Option func(*Client)

func WithBaseURL(u string) Option {
	return func(c *Client) {
		c.baseURL = strings.TrimRight(u, "/")
	}
}

func WithHTTPClient(httpClient *http.Client) Option {
	return func(c *Client) {
		c.httpClient = httpClient
	}
}

func NewClient(apiKey string, opts ...Option) *Client {
	c := &Client{
		apiKey:     apiKey,
		baseURL:    DefaultBaseURL,
		httpClient: &http.Client{Timeout: 10 * time.Second},
	}
	for _, opt := range opts {
		opt(c)
	}
	return c
}

func (c *Client) Search(ctx context.Context, query string, page int32) (*MultiSearchResponse, error) {
	if page <= 0 {
		page = 1
	}

	endpoint := fmt.Sprintf("%s/search/multi", c.baseURL)
	params := url.Values{}
	params.Set("query", query)
	params.Set("page", strconv.Itoa(int(page)))

	var resp MultiSearchResponse
	if err := c.doGet(ctx, endpoint, params, &resp); err != nil {
		return nil, err
	}
	return &resp, nil
}

func (c *Client) GetMovie(ctx context.Context, id int) (*MovieDetails, error) {
	endpoint := fmt.Sprintf("%s/movie/%d", c.baseURL, id)
	params := url.Values{}
	params.Set("append_to_response", "external_ids")

	var movie MovieDetails
	if err := c.doGet(ctx, endpoint, params, &movie); err != nil {
		return nil, err
	}
	return &movie, nil
}

func (c *Client) GetTV(ctx context.Context, id int) (*TVDetails, error) {
	endpoint := fmt.Sprintf("%s/tv/%d", c.baseURL, id)
	params := url.Values{}
	params.Set("append_to_response", "external_ids")

	var tv TVDetails
	if err := c.doGet(ctx, endpoint, params, &tv); err != nil {
		return nil, err
	}
	return &tv, nil
}

func (c *Client) GetTVSeason(ctx context.Context, tvID, seasonNumber int) (*TVSeasonDetails, error) {
	endpoint := fmt.Sprintf("%s/tv/%d/season/%d", c.baseURL, tvID, seasonNumber)
	var season TVSeasonDetails
	if err := c.doGet(ctx, endpoint, url.Values{}, &season); err != nil {
		return nil, err
	}
	return &season, nil
}

func (c *Client) BuildPosterURL(posterPath string) string {
	if posterPath == "" {
		return ""
	}
	if strings.HasPrefix(posterPath, "http://") || strings.HasPrefix(posterPath, "https://") {
		return posterPath
	}
	return DefaultPosterURL + "/" + strings.TrimLeft(posterPath, "/")
}

func (c *Client) doGet(ctx context.Context, endpoint string, params url.Values, out any) error {
	if c.apiKey == "" {
		return ErrEmptyAPIKey
	}

	params.Set("api_key", c.apiKey)
	fullURL := endpoint + "?" + params.Encode()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, fullURL, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("Accept", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("http request failed: %w", err)
	}
	defer resp.Body.Close()

	switch resp.StatusCode {
	case http.StatusOK:
		if err := json.NewDecoder(resp.Body).Decode(out); err != nil {
			return fmt.Errorf("failed to decode response body: %w", err)
		}
		return nil
	case http.StatusNotFound:
		return ErrNotFound
	case http.StatusTooManyRequests:
		return ErrRateLimited
	default:
		return fmt.Errorf("%w: status %d", ErrInvalidStatus, resp.StatusCode)
	}
}
