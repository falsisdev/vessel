package plugin

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"sync"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type nuvioScraper struct {
	ID             string   `json:"id"`
	Name           string   `json:"name"`
	Description    string   `json:"description"`
	Version        string   `json:"version"`
	Author         string   `json:"author"`
	SupportedTypes []string `json:"supportedTypes"`
	Filename       string   `json:"filename"`
	Formats        []string `json:"formats"`
	Logo           string   `json:"logo"`
}

type nuvioManifest struct {
	Name        string          `json:"name"`
	Version     string          `json:"version"`
	Author      string          `json:"author"`
	Description string          `json:"description"`
	Scrapers    []nuvioScraper  `json:"scrapers"`
}

type nuvioChannel struct {
	ID        string
	Name      string
	LogoURL   string
	Category  string
	StreamURL string
	Country   string
	Quality   string
}

type NuvioAdapter struct {
	httpClient  *http.Client
	manifestURL string
	rawBase     string
	m           nuvioManifest
	hasLive     bool

	mu       sync.RWMutex
	channels []nuvioChannel
	byID     map[string]nuvioChannel
	loaded   bool
}

func (a *NuvioAdapter) client() *http.Client {
	if a.httpClient == nil {
		a.httpClient = &http.Client{Timeout: 15 * time.Second}
	}
	return a.httpClient
}

func nuvioSlugify(name string) string {
	lower := strings.ToLower(strings.TrimSpace(name))
	var sb strings.Builder
	for _, r := range lower {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') {
			sb.WriteRune(r)
		} else if r == ' ' || r == '-' || r == '_' {
			sb.WriteRune('.')
		}
	}
	return strings.Trim(sb.String(), ".")
}

func nuvioRawBase(manifestURL string) string {
	u, err := url.Parse(manifestURL)
	if err != nil {
		return ""
	}
	host := u.Hostname()
	path := strings.TrimPrefix(u.Path, "/")
	parts := strings.Split(path, "/")

	if strings.HasSuffix(host, "github.io") {
		user := strings.TrimSuffix(host, ".github.io")
		repo := ""
		if len(parts) > 0 {
			repo = parts[0]
		}
		if user != "" && repo != "" {
			return fmt.Sprintf("https://raw.githubusercontent.com/%s/%s/main", user, repo)
		}
	}
	if strings.Contains(host, "raw.githubusercontent.com") {
		branch := "main"
		if len(parts) > 2 {
			branch = parts[2]
		}
		if len(parts) >= 2 {
			return fmt.Sprintf("https://raw.githubusercontent.com/%s/%s/%s", parts[0], parts[1], branch)
		}
	}
	return ""
}

func nuvioHasLiveTVScrapers(scrapers []nuvioScraper) bool {
	for _, s := range scrapers {
		for _, t := range s.SupportedTypes {
			if t == "channel" {
				return true
			}
		}
		f := strings.ToLower(s.Filename)
		if strings.Contains(f, "m3u") || strings.Contains(f, "iptv") || strings.Contains(f, "live") {
			return true
		}
	}
	return false
}

func (a *NuvioAdapter) GetManifest(ctx context.Context, manifestURL string) (*pluginv1.PluginManifest, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, manifestURL, nil)
	if err != nil {
		return nil, err
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch nuvio manifest: %w", err)
	}
	defer resp.Body.Close()
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read nuvio manifest: %w", err)
	}

	var m nuvioManifest
	if err := json.Unmarshal(body, &m); err != nil {
		return nil, fmt.Errorf("invalid nuvio manifest JSON: %w", err)
	}
	if m.Name == "" {
		return nil, fmt.Errorf("nuvio manifest missing required 'name' field")
	}

	a.manifestURL = manifestURL
	a.m = m
	a.rawBase = nuvioRawBase(manifestURL)
	a.hasLive = nuvioHasLiveTVScrapers(m.Scrapers)
	a.byID = make(map[string]nuvioChannel)

	slug := nuvioSlugify(m.Name)
	if slug == "" {
		slug = "unknown"
	}
	pluginID := fmt.Sprintf("com.vessel.bridge.nuvio.%s", slug)

	domain := pluginv1.Domain_DOMAIN_CINEMA
	if a.hasLive {
		domain = pluginv1.Domain_DOMAIN_IPTV
	}

	return &pluginv1.PluginManifest{
		Id:              pluginID,
		Name:            m.Name,
		Version:         m.Version,
		Description:     m.Description,
		Author:          m.Author,
		Domain:          domain,
		Capabilities: []pluginv1.Capability{
			pluginv1.Capability_CAPABILITY_SEARCH,
			pluginv1.Capability_CAPABILITY_METADATA,
			pluginv1.Capability_CAPABILITY_STREAMS,
		},
		ProtocolVersion: "1.0.0",
	}, nil
}

func (a *NuvioAdapter) loadChannels(ctx context.Context) error {
	a.mu.RLock()
	if a.loaded {
		a.mu.RUnlock()
		return nil
	}
	a.mu.RUnlock()

	if a.rawBase == "" {
		return fmt.Errorf("no raw base URL derived from manifest")
	}

	m3uURL := a.rawBase + "/providers/M3U/Liste/canli.m3u"
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, m3uURL, nil)
	if err != nil {
		return fmt.Errorf("failed to build M3U request: %w", err)
	}
	resp, err := a.client().Do(req)
	if err != nil {
		return fmt.Errorf("failed to fetch M3U: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("M3U endpoint returned HTTP %d", resp.StatusCode)
	}

	var parsed []nuvioChannel
	var curName, curLogo, curGroup, curID string

	scanner := bufio.NewScanner(resp.Body)
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "#EXTINF:") {
			curName = ""
			curLogo = ""
			curGroup = "General"
			curID = ""

			if idx := strings.Index(line, `tvg-id="`); idx != -1 {
				sub := line[idx+8:]
				if end := strings.Index(sub, `"`); end != -1 {
					curID = sub[:end]
				}
			}
			if idx := strings.Index(line, `tvg-logo="`); idx != -1 {
				sub := line[idx+10:]
				if end := strings.Index(sub, `"`); end != -1 {
					curLogo = sub[:end]
				}
			}
			if idx := strings.Index(line, `group-title="`); idx != -1 {
				sub := line[idx+13:]
				if end := strings.Index(sub, `"`); end != -1 {
					curGroup = sub[:end]
				}
			}
			if idx := strings.LastIndex(line, ","); idx != -1 {
				curName = strings.TrimSpace(line[idx+1:])
			}
		} else if strings.HasPrefix(line, "http://") || strings.HasPrefix(line, "https://") {
			if curName != "" {
				chID := curID
				if chID == "" {
					chID = fmt.Sprintf("nuvio-ch-%d", len(parsed)+1)
				}

				country := "TR"
				for _, tok := range strings.Fields(curGroup) {
					upper := strings.ToUpper(tok)
					if len(upper) == 2 && upper[0] >= 'A' && upper[0] <= 'Z' && upper[1] >= 'A' && upper[1] <= 'Z' {
						country = upper
						break
					}
				}

				parsed = append(parsed, nuvioChannel{
					ID:        chID,
					Name:      curName,
					LogoURL:   curLogo,
					Category:  curGroup,
					StreamURL: line,
					Country:   country,
					Quality:   "HD",
				})
			}
		}
	}

	if len(parsed) > 0 {
		a.mu.Lock()
		a.channels = parsed
		a.byID = make(map[string]nuvioChannel, len(parsed))
		for _, ch := range parsed {
			a.byID[ch.ID] = ch
		}
		a.loaded = true
		a.mu.Unlock()
	}

	return nil
}

func (a *NuvioAdapter) Search(ctx context.Context, baseURL string, query string, page int32) (*pluginv1.SearchResponse, error) {
	if err := a.loadChannels(ctx); err != nil {
		return nil, err
	}

	a.mu.RLock()
	defer a.mu.RUnlock()

	lower := strings.ToLower(strings.TrimSpace(query))
	countryFilter := ""
	catFilter := ""
	textQ := lower

	for _, token := range strings.Fields(lower) {
		if strings.HasPrefix(token, "country:") {
			countryFilter = strings.ToUpper(strings.TrimPrefix(token, "country:"))
			textQ = strings.TrimSpace(strings.ReplaceAll(textQ, token, ""))
		} else if strings.HasPrefix(token, "category:") {
			catFilter = strings.ToLower(strings.TrimPrefix(token, "category:"))
			textQ = strings.TrimSpace(strings.ReplaceAll(textQ, token, ""))
		}
	}

	if countryFilter == "ALL" {
		countryFilter = ""
	}

	var matches []nuvioChannel
	for _, ch := range a.channels {
		if countryFilter != "" && !strings.EqualFold(ch.Country, countryFilter) {
			continue
		}
		if catFilter != "" && !strings.EqualFold(ch.Category, catFilter) {
			continue
		}

		if textQ == "" || textQ == "popular" || textQ == "featured" || textQ == "trending" {
			matches = append(matches, ch)
		} else if textQ == "news" || textQ == "haber" {
			if strings.EqualFold(ch.Category, "News") || strings.Contains(strings.ToLower(ch.Name), "haber") || strings.Contains(strings.ToLower(ch.Name), "news") {
				matches = append(matches, ch)
			}
		} else if textQ == "sports" || textQ == "spor" {
			if strings.EqualFold(ch.Category, "Sports") || strings.Contains(strings.ToLower(ch.Name), "spor") || strings.Contains(strings.ToLower(ch.Name), "sport") {
				matches = append(matches, ch)
			}
		} else if textQ == "belgesel" || textQ == "documentary" {
			if strings.Contains(strings.ToLower(ch.Category), "belgesel") || strings.Contains(strings.ToLower(ch.Category), "documentary") || strings.Contains(strings.ToLower(ch.Name), "belgesel") {
				matches = append(matches, ch)
			}
		} else if textQ == "muzik" || textQ == "müzik" || textQ == "music" {
			if strings.Contains(strings.ToLower(ch.Category), "muzik") || strings.Contains(strings.ToLower(ch.Category), "müzik") || strings.Contains(strings.ToLower(ch.Category), "music") {
				matches = append(matches, ch)
			}
		} else {
			if strings.Contains(strings.ToLower(ch.Name), textQ) ||
				strings.Contains(strings.ToLower(ch.Category), textQ) ||
				strings.Contains(strings.ToLower(ch.Country), textQ) {
				matches = append(matches, ch)
			}
		}
	}

	var items []*pluginv1.MediaItem
	for _, ch := range matches {
		items = append(items, &pluginv1.MediaItem{
			Id:        ch.ID,
			Title:     ch.Name,
			Type:      pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED,
			Year:      2024,
			PosterUrl: ch.LogoURL,
			Overview:  fmt.Sprintf("%s (%s • %s Live TV)", ch.Name, ch.Category, ch.Country),
			ExternalIds: &pluginv1.ExternalIDs{
				Extra: map[string]string{
					"stream_url": ch.StreamURL,
					"category":   ch.Category,
					"country":    ch.Country,
					"quality":    ch.Quality,
				},
			},
		})
	}

	return &pluginv1.SearchResponse{
		Items:   items,
		HasMore: false,
	}, nil
}

func (a *NuvioAdapter) GetMetadata(ctx context.Context, baseURL string, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	if err := a.loadChannels(ctx); err != nil {
		return nil, err
	}

	a.mu.RLock()
	defer a.mu.RUnlock()

	ch, exists := a.byID[mediaID]
	if !exists {
		return nil, fmt.Errorf("nuvio channel not found: %s", mediaID)
	}

	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:        ch.ID,
			Title:     ch.Name,
			Type:      pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED,
			Year:      2024,
			PosterUrl: ch.LogoURL,
			Overview:  fmt.Sprintf("%s Live TV. Category: %s, Country: %s, Resolution: %s.", ch.Name, ch.Category, ch.Country, ch.Quality),
			Genres:    []string{ch.Category, "Live", "IPTV"},
			ExternalIds: &pluginv1.ExternalIDs{
				Extra: map[string]string{
					"stream_url": ch.StreamURL,
					"category":   ch.Category,
					"country":    ch.Country,
					"quality":    ch.Quality,
				},
			},
		},
	}, nil
}

func (a *NuvioAdapter) GetStreams(ctx context.Context, baseURL string, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	if err := a.loadChannels(ctx); err != nil {
		return nil, err
	}

	a.mu.RLock()
	defer a.mu.RUnlock()

	ch, exists := a.byID[mediaID]
	if !exists {
		return nil, fmt.Errorf("nuvio channel not found: %s", mediaID)
	}

	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{
			{
				Id:      fmt.Sprintf("nuvio-%s", ch.ID),
				Title:   fmt.Sprintf("%s (Live HLS)", ch.Name),
				Url:     ch.StreamURL,
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_HLS,
				Quality: ch.Quality,
				Headers: map[string]string{
					"User-Agent": "Vessel/1.0 (Nuvio Bridge)",
				},
			},
		},
	}, nil
}
