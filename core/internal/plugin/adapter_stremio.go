package plugin

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"strconv"
	"strings"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type StremioAdapter struct {
	httpClient  *http.Client
	manifestURL string
	manifest    *stremioManifest
}

type stremioCatalog struct {
	Type  string `json:"type"`
	ID    string `json:"id"`
	Name  string `json:"name"`
	Extra []struct {
		Name       string `json:"name"`
		IsRequired bool   `json:"isRequired"`
	} `json:"extra"`
}

type stremioManifest struct {
	ID          string           `json:"id"`
	Version     string           `json:"version"`
	Name        string           `json:"name"`
	Description string           `json:"description"`
	Author      string           `json:"author"`
	Resources   json.RawMessage  `json:"resources"`
	Types       []string         `json:"types"`
	Catalogs    []stremioCatalog `json:"catalogs"`
}

type stremioMeta struct {
	ID          string   `json:"id"`
	Type        string   `json:"type"`
	Name        string   `json:"name"`
	Release     string   `json:"release"`
	Poster      string   `json:"poster"`
	Background  string   `json:"background"`
	Description string   `json:"description"`
	Genres      []string `json:"genres"`
	Videos      []struct {
		ID         string `json:"id"`
		Name       string `json:"name"`
		Season     int    `json:"season"`
		Episode    int    `json:"episode"`
		Title      string `json:"title"`
		Overview   string `json:"overview"`
		Duration   int    `json:"duration"`
	} `json:"videos"`
}

type stremioStream struct {
	Name           string            `json:"name"`
	Title          string            `json:"title"`
	URL            string            `json:"url"`
	Quality        string            `json:"quality"`
	Description    string            `json:"description"`
	BehaviorHints  struct {
		IsLive bool `json:"isLive"`
	} `json:"behaviorHints"`
	Headers map[string]string `json:"headers"`
}

type stremioSearchItem struct {
	ID          string   `json:"id"`
	Type        string   `json:"type"`
	Name        string   `json:"name"`
	Poster      string   `json:"poster"`
	Background  string   `json:"background"`
	Description string   `json:"description"`
	Genres      []string `json:"genres"`
	ReleaseInfo string   `json:"releaseInfo"`
}

var (
	reCountrySuffix = regexp.MustCompile(`\.([a-zA-Z]{2})$`)
	reYear          = regexp.MustCompile(`^(\d{4})`)
)

func (a *StremioAdapter) client() *http.Client {
	if a.httpClient == nil {
		a.httpClient = &http.Client{Timeout: 15 * time.Second}
	}
	return a.httpClient
}

func stremioBaseURL(manifestURL string) string {
	u, err := url.Parse(manifestURL)
	if err != nil {
		return manifestURL
	}
	path := strings.TrimRight(u.Path, "/")
	if strings.HasSuffix(path, "/manifest.json") {
		path = path[:len(path)-len("/manifest.json")]
	}
	u.Path = path
	u.RawQuery = ""
	u.Fragment = ""
	return strings.TrimRight(u.String(), "/")
}

func stremioTypeForItem(stremioType string) pluginv1.MediaType {
	switch strings.ToLower(stremioType) {
	case "movie":
		return pluginv1.MediaType_MEDIA_TYPE_MOVIE
	case "series", "tv":
		return pluginv1.MediaType_MEDIA_TYPE_SERIES
	default:
		return pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED
	}
}

func stremioTypeHint(mediaID string) string {
	if strings.HasPrefix(mediaID, "tv:") {
		return "tv"
	}
	if strings.HasPrefix(mediaID, "movie:") {
		return "movie"
	}
	return ""
}

func stremioCountryFromID(mediaID string) string {
	lastSeg := mediaID
	if idx := strings.LastIndex(mediaID, "/"); idx != -1 {
		lastSeg = mediaID[idx+1:]
	}
	m := reCountrySuffix.FindStringSubmatch(lastSeg)
	if len(m) > 1 {
		return strings.ToUpper(m[1])
	}
	return ""
}

func stremioYearFromRelease(release string) int32 {
	if m := reYear.FindStringSubmatch(release); len(m) > 1 {
		if y, err := strconv.Atoi(m[1]); err == nil {
			return int32(y)
		}
	}
	return 0
}

func stremioFormatFromURL(streamURL string) pluginv1.StreamFormat {
	lower := strings.ToLower(streamURL)
	switch {
	case strings.HasSuffix(lower, ".m3u8") || strings.Contains(lower, ".m3u8"):
		return pluginv1.StreamFormat_STREAM_FORMAT_HLS
	case strings.Contains(lower, ".mpd"):
		return pluginv1.StreamFormat_STREAM_FORMAT_DASH
	case strings.Contains(lower, ".mp4"):
		return pluginv1.StreamFormat_STREAM_FORMAT_MP4
	case strings.Contains(lower, ".mkv"):
		return pluginv1.StreamFormat_STREAM_FORMAT_MKV
	default:
		return pluginv1.StreamFormat_STREAM_FORMAT_UNSPECIFIED
	}
}

var stremioKeywords = map[string]string{
	"popular": "", "featured": "", "trending": "", "top": "", "new": "", "latest": "",
}

func (a *StremioAdapter) selectCatalog(query string) (*stremioCatalog, bool, string) {
	if a.manifest == nil || len(a.manifest.Catalogs) == 0 {
		return nil, false, query
	}
	lower := strings.ToLower(strings.TrimSpace(query))

	catFilter := ""
	textQ := lower
	for _, token := range strings.Fields(lower) {
		if strings.HasPrefix(token, "category:") {
			catFilter = strings.TrimPrefix(token, "category:")
			textQ = strings.TrimSpace(strings.ReplaceAll(textQ, token, ""))
		} else if strings.HasPrefix(token, "country:") {
			textQ = strings.TrimSpace(strings.ReplaceAll(textQ, token, ""))
		}
	}

	if catFilter != "" {
		for i := range a.manifest.Catalogs {
			c := &a.manifest.Catalogs[i]
			cid := strings.ToLower(c.ID)
			cname := strings.ToLower(c.Name)
			if strings.Contains(cid, catFilter) || strings.Contains(cname, catFilter) {
				return c, textQ != "", textQ
			}
		}
	}

	isKeyword := textQ == "" || stremioKeywords[textQ] != ""

	var tvType string
	for _, t := range a.manifest.Types {
		switch t {
		case "tv", "series":
			tvType = t
		case "movie":
		}
	}
	if tvType == "" {
		tvType = "movie"
	}

	selectByType := func(t string) (*stremioCatalog, bool) {
		for i := range a.manifest.Catalogs {
			if a.manifest.Catalogs[i].Type == t {
				return &a.manifest.Catalogs[i], true
			}
		}
		if len(a.manifest.Catalogs) > 0 {
			return &a.manifest.Catalogs[0], true
		}
		return nil, false
	}

	switch {
	case textQ == "news" || textQ == "haber":
		for i := range a.manifest.Catalogs {
			cid := strings.ToLower(a.manifest.Catalogs[i].ID)
			cname := strings.ToLower(a.manifest.Catalogs[i].Name)
			if strings.Contains(cid, "haber") || strings.Contains(cid, "news") ||
				strings.Contains(cname, "haber") || strings.Contains(cname, "news") {
				return &a.manifest.Catalogs[i], false, ""
			}
		}
		cat, ok := selectByType(tvType)
		return cat, ok, ""
	case textQ == "sports" || textQ == "spor":
		for i := range a.manifest.Catalogs {
			cid := strings.ToLower(a.manifest.Catalogs[i].ID)
			cname := strings.ToLower(a.manifest.Catalogs[i].Name)
			if strings.Contains(cid, "spor") || strings.Contains(cid, "sports") ||
				strings.Contains(cname, "spor") || strings.Contains(cname, "sports") {
				return &a.manifest.Catalogs[i], false, ""
			}
		}
		cat, ok := selectByType(tvType)
		return cat, ok, ""
	case textQ == "music" || textQ == "muzik" || textQ == "müzik":
		for i := range a.manifest.Catalogs {
			cid := strings.ToLower(a.manifest.Catalogs[i].ID)
			cname := strings.ToLower(a.manifest.Catalogs[i].Name)
			if strings.Contains(cid, "muzik") || strings.Contains(cid, "müzik") || strings.Contains(cid, "music") ||
				strings.Contains(cname, "muzik") || strings.Contains(cname, "müzik") || strings.Contains(cname, "music") {
				return &a.manifest.Catalogs[i], false, ""
			}
		}
		cat, ok := selectByType(tvType)
		return cat, ok, ""
	case textQ == "belgesel" || textQ == "documentary" || textQ == "cocuk" || textQ == "çocuk":
		for i := range a.manifest.Catalogs {
			cid := strings.ToLower(a.manifest.Catalogs[i].ID)
			cname := strings.ToLower(a.manifest.Catalogs[i].Name)
			if strings.Contains(cid, textQ) || strings.Contains(cname, textQ) {
				return &a.manifest.Catalogs[i], false, ""
			}
		}
		cat, ok := selectByType(tvType)
		return cat, ok, ""
	case isKeyword:
		cat, ok := selectByType(tvType)
		return cat, ok, ""
	default:
		cat, ok := selectByType(tvType)
		return cat, ok, textQ
	}
}

func (a *StremioAdapter) GetManifest(ctx context.Context, manifestURL string) (*pluginv1.PluginManifest, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, manifestURL, nil)
	if err != nil {
		return nil, err
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch stremio manifest: %w", err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read stremio manifest: %w", err)
	}

	var m stremioManifest
	if err := json.Unmarshal(body, &m); err != nil {
		return nil, fmt.Errorf("invalid stremio manifest JSON: %w", err)
	}
	if m.ID == "" {
		var wrap struct {
			Manifest stremioManifest `json:"manifest"`
		}
		if err := json.Unmarshal(body, &wrap); err == nil && wrap.Manifest.ID != "" {
			m = wrap.Manifest
		} else {
			return nil, fmt.Errorf("stremio manifest missing required 'id' field")
		}
	}
	if m.Name == "" {
		return nil, fmt.Errorf("stremio manifest missing required 'name' field")
	}

	a.manifestURL = manifestURL
	a.manifest = &m

	hasChannel := false
	isLive := false
	lowerID := strings.ToLower(m.ID)
	lowerName := strings.ToLower(m.Name)
	for _, t := range m.Types {
		if t == "channel" {
			hasChannel = true
		}
	}
	if !hasChannel {
		for _, hint := range []string{"iptv", "canlitv", "canlı tv", "live tv", "canlı"} {
			if strings.Contains(lowerID, hint) || strings.Contains(lowerName, hint) {
				isLive = true
				break
			}
		}
	}
	domain := pluginv1.Domain_DOMAIN_CINEMA
	if hasChannel || isLive {
		domain = pluginv1.Domain_DOMAIN_IPTV
	}

	caps := []pluginv1.Capability{
		pluginv1.Capability_CAPABILITY_METADATA,
		pluginv1.Capability_CAPABILITY_STREAMS,
	}
	if len(m.Catalogs) > 0 {
		caps = append(caps, pluginv1.Capability_CAPABILITY_SEARCH)
	}

	return &pluginv1.PluginManifest{
		Id:              m.ID,
		Name:            m.Name,
		Version:         m.Version,
		Description:     m.Description,
		Author:          m.Author,
		Domain:          domain,
		Capabilities:    caps,
		ProtocolVersion: "1.0.0",
	}, nil
}

func (a *StremioAdapter) Search(ctx context.Context, baseURL string, query string, page int32) (*pluginv1.SearchResponse, error) {
	if a.manifest == nil {
		return &pluginv1.SearchResponse{}, nil
	}

	cat, useSearch, searchText := a.selectCatalog(query)
	if cat == nil {
		return &pluginv1.SearchResponse{Items: []*pluginv1.MediaItem{}}, nil
	}

	lower := strings.ToLower(strings.TrimSpace(query))
	countryFilter := ""
	for _, token := range strings.Fields(lower) {
		if strings.HasPrefix(token, "country:") {
			countryFilter = strings.ToUpper(strings.TrimPrefix(token, "country:"))
		}
	}

	var endpoint string
	if useSearch && searchText != "" {
		endpoint = fmt.Sprintf("%s/catalog/%s/%s/search=%s.json",
			baseURL, cat.Type, cat.ID, url.PathEscape(searchText))
	} else {
		endpoint = fmt.Sprintf("%s/catalog/%s/%s.json", baseURL, cat.Type, cat.ID)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to build catalog request: %w", err)
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch catalog: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return &pluginv1.SearchResponse{Items: []*pluginv1.MediaItem{}}, nil
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read catalog response: %w", err)
	}

	var catalogResp struct {
		Metas []stremioSearchItem `json:"metas"`
	}
	if err := json.Unmarshal(body, &catalogResp); err != nil {
		return &pluginv1.SearchResponse{Items: []*pluginv1.MediaItem{}}, nil
	}

	var items []*pluginv1.MediaItem
	for _, meta := range catalogResp.Metas {
		if countryFilter != "" && countryFilter != "ALL" {
			idCountry := stremioCountryFromID(meta.ID)
			if idCountry != "" && idCountry != countryFilter {
				continue
			}
		}
		extra := map[string]string{}
		if len(meta.Genres) > 0 && meta.Genres[0] != "" {
			extra["category"] = meta.Genres[0]
		}
		if c := stremioCountryFromID(meta.ID); c != "" {
			extra["country"] = c
		}
		if a.manifest != nil {
			for _, t := range a.manifest.Types {
				if t == "channel" {
					extra["quality"] = "Live"
					break
				}
			}
		}
		items = append(items, &pluginv1.MediaItem{
			Id:        meta.ID,
			Title:     meta.Name,
			Type:      stremioTypeForItem(meta.Type),
			Year:      stremioYearFromRelease(meta.ReleaseInfo),
			PosterUrl: meta.Poster,
			Overview:  meta.Description,
			ExternalIds: &pluginv1.ExternalIDs{
				Extra: extra,
			},
		})
	}

	return &pluginv1.SearchResponse{
		Items:   items,
		HasMore: false,
	}, nil
}

func (a *StremioAdapter) GetMetadata(ctx context.Context, baseURL string, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	mediaType := stremioTypeHint(mediaID)
	if mediaType == "" {
		if a.manifest != nil {
			for _, t := range a.manifest.Types {
				if t == "channel" || t == "tv" || t == "series" {
					mediaType = t
					break
				}
			}
		}
		if mediaType == "" {
			mediaType = "movie"
		}
	}

	endpoint := fmt.Sprintf("%s/meta/%s/%s.json", baseURL, mediaType, url.PathEscape(mediaID))
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to build metadata request: %w", err)
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch metadata: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("metadata endpoint returned HTTP %d", resp.StatusCode)
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read metadata: %w", err)
	}

	var metaResp struct {
		Meta stremioMeta `json:"meta"`
	}
	if err := json.Unmarshal(body, &metaResp); err != nil {
		return nil, fmt.Errorf("invalid metadata JSON: %w", err)
	}
	meta := metaResp.Meta
	if meta.Name == "" && meta.ID == "" {
		return nil, fmt.Errorf("metadata not found for %s", mediaID)
	}

	extra := map[string]string{}
	if len(meta.Genres) > 0 && meta.Genres[0] != "" {
		extra["category"] = meta.Genres[0]
	}
	if c := stremioCountryFromID(meta.ID); c != "" {
		extra["country"] = c
	}

	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:         meta.ID,
			Title:      meta.Name,
			Type:       stremioTypeForItem(meta.Type),
			Year:       stremioYearFromRelease(meta.Release),
			PosterUrl:  meta.Poster,
			Overview:   meta.Description,
			Genres:     meta.Genres,
			ExternalIds: &pluginv1.ExternalIDs{
				Extra: extra,
			},
		},
	}, nil
}

func (a *StremioAdapter) GetStreams(ctx context.Context, baseURL string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	mediaType := stremioTypeHint(mediaID)
	if mediaType == "" {
		if a.manifest != nil {
			for _, t := range a.manifest.Types {
				if t == "channel" || t == "tv" || t == "series" {
					mediaType = t
					break
				}
			}
		}
		if mediaType == "" {
			mediaType = "movie"
		}
	}

	var endpoint string
	if season > 0 && episode > 0 && (mediaType == "series" || mediaType == "tv") {
		endpoint = fmt.Sprintf("%s/stream/%s/%s/%d/%d.json",
			baseURL, mediaType, url.PathEscape(mediaID), season, episode)
	} else {
		endpoint = fmt.Sprintf("%s/stream/%s/%s.json", baseURL, mediaType, url.PathEscape(mediaID))
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to build stream request: %w", err)
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch streams: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return &pluginv1.GetStreamsResponse{Streams: []*pluginv1.StreamSource{}}, nil
	}

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read streams: %w", err)
	}

	var streamResp struct {
		Streams []stremioStream `json:"streams"`
	}
	if err := json.Unmarshal(body, &streamResp); err != nil {
		return &pluginv1.GetStreamsResponse{Streams: []*pluginv1.StreamSource{}}, nil
	}

	var sources []*pluginv1.StreamSource
	for _, s := range streamResp.Streams {
		title := s.Title
		if title == "" {
			title = s.Name
		}
		if title == "" {
			title = "Live Stream"
		}
		quality := s.Quality
		if quality == "" {
			quality = "HD"
		}
		headers := s.Headers
		if headers == nil {
			headers = map[string]string{}
		}
		if _, ok := headers["User-Agent"]; !ok {
			headers["User-Agent"] = "Vessel/1.0 (Stremio Bridge)"
		}

		sources = append(sources, &pluginv1.StreamSource{
			Title:   title,
			Url:     s.URL,
			Format:  stremioFormatFromURL(s.URL),
			Quality: quality,
			Headers: headers,
		})
	}

	return &pluginv1.GetStreamsResponse{
		Streams: sources,
	}, nil
}
