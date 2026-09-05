package provider

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

const (
	PluginID   = "com.vessel.iptv"
	PluginName = "IPTV"
)

type Channel struct {
	ID        string
	Name      string
	LogoURL   string
	Category  string
	StreamURL string
	Country   string
	Quality   string
}

type IPTVService struct {
	pluginv1.UnimplementedPluginServiceServer

	httpClient *http.Client
	mu         sync.RWMutex
	channels   []Channel
	byID       map[string]Channel
	loaded     bool
}

func NewIPTVService() *IPTVService {
	s := &IPTVService{
		httpClient: &http.Client{Timeout: 6 * time.Second},
		byID:       make(map[string]Channel),
	}
	s.loadFallbackChannels()
	// Asynchronously refresh channels from iptv-org in background
	go s.refreshFromIPTVOrg()
	return s
}

func (s *IPTVService) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:              PluginID,
			Name:            PluginName,
			Version:         "1.0.0",
			Description:     "Dünya genelinden açık TV yayınları ve kanalları (iptv-org)",
			Author:          "Vessel Team",
			Domain:          pluginv1.Domain_DOMAIN_IPTV,
			Capabilities:    []pluginv1.Capability{
				pluginv1.Capability_CAPABILITY_SEARCH,
				pluginv1.Capability_CAPABILITY_METADATA,
				pluginv1.Capability_CAPABILITY_STREAMS,
			},
			ProtocolVersion: "1.0.0",
			IsBuiltin:       true,
		},
	}, nil
}

func (s *IPTVService) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	q := strings.TrimSpace(strings.ToLower(req.Query))
	var matches []Channel

	for _, ch := range s.channels {
		if q == "" || q == "popular" || q == "featured" || q == "trending" {
			matches = append(matches, ch)
		} else if q == "news" || q == "haber" {
			if strings.EqualFold(ch.Category, "News") || strings.Contains(strings.ToLower(ch.Name), "haber") || strings.Contains(strings.ToLower(ch.Name), "news") {
				matches = append(matches, ch)
			}
		} else if q == "sports" || q == "spor" {
			if strings.EqualFold(ch.Category, "Sports") || strings.Contains(strings.ToLower(ch.Name), "spor") || strings.Contains(strings.ToLower(ch.Name), "sport") {
				matches = append(matches, ch)
			}
		} else {
			if strings.Contains(strings.ToLower(ch.Name), q) || strings.Contains(strings.ToLower(ch.Category), q) || strings.Contains(strings.ToLower(ch.Country), q) {
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
			Overview:  fmt.Sprintf("%s (%s • %s Canlı Yayın)", ch.Name, ch.Category, ch.Country),
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

func (s *IPTVService) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	ch, exists := s.byID[req.MediaId]
	if !exists {
		// Fallback search by ID
		for _, c := range s.channels {
			if c.ID == req.MediaId {
				ch = c
				exists = true
				break
			}
		}
	}

	if !exists {
		return nil, fmt.Errorf("channel not found: %s", req.MediaId)
	}

	details := &pluginv1.MediaDetails{
		Id:        ch.ID,
		Title:     ch.Name,
		Type:      pluginv1.MediaType_MEDIA_TYPE_UNSPECIFIED,
		Year:      2024,
		PosterUrl: ch.LogoURL,
		Overview:  fmt.Sprintf("%s Canlı TV Yayını. Kategori: %s, Ülke: %s, Çözünürlük: %s.", ch.Name, ch.Category, ch.Country, ch.Quality),
		Genres:    []string{ch.Category, "Live", "IPTV"},
		ExternalIds: &pluginv1.ExternalIDs{
			Extra: map[string]string{
				"stream_url": ch.StreamURL,
				"category":   ch.Category,
				"country":    ch.Country,
				"quality":    ch.Quality,
			},
		},
	}

	return &pluginv1.GetMetadataResponse{Details: details}, nil
}

func (s *IPTVService) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	ch, exists := s.byID[req.MediaId]
	if !exists {
		return nil, fmt.Errorf("channel not found: %s", req.MediaId)
	}

	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{
			{
				Id:      fmt.Sprintf("iptv-%s", ch.ID),
				Title:   fmt.Sprintf("%s (Canlı HLS)", ch.Name),
				Url:     ch.StreamURL,
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_HLS,
				Quality: ch.Quality,
			},
		},
	}, nil
}

func (s *IPTVService) GetChapterContent(ctx context.Context, req *pluginv1.GetChapterContentRequest) (*pluginv1.GetChapterContentResponse, error) {
	return nil, fmt.Errorf("reading not supported on iptv plugin")
}

func (s *IPTVService) loadFallbackChannels() {
	s.mu.Lock()
	defer s.mu.Unlock()

	fallbacks := []Channel{
		// Turkey Channels
		{ID: "tr-trt1", Name: "TRT 1", Category: "General", StreamURL: "https://tv-trt1.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/47/TRT_1_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trthaber", Name: "TRT Haber", Category: "News", StreamURL: "https://tv-trthaber.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/e0/TRT_Haber_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trtspore", Name: "TRT Spor", Category: "Sports", StreamURL: "https://tv-trtspor.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/2/23/TRT_Spor_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trtbelgesel", Name: "TRT Belgesel", Category: "Documentary", StreamURL: "https://tv-trtbelgesel.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/b/b8/TRT_Belgesel_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-ahaber", Name: "A Haber", Category: "News", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/ahaber/ahaber.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/7c/Ahaber_Logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-aspor", Name: "A Spor", Category: "Sports", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/aspor/aspor.m3u8", LogoURL: "https://i.imgur.com/ZhkZzLf.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-atv", Name: "ATV", Category: "General", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/atv/atv_1080p.m3u8", LogoURL: "https://i.imgur.com/HyVUwFC.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-tv360", Name: "360 TV", Category: "General", StreamURL: "https://turkmedya-live.ercdn.net/tv360/tv360.m3u8", LogoURL: "https://i.imgur.com/agn47sQ.png", Country: "TR", Quality: "720p"},

		// International News & Entertainment
		{ID: "intl-bbcnews", Name: "BBC News", Category: "News", StreamURL: "https://vs-hls-push-ww-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_news_channel_hd/t=3840/v=pv14/b=5070016/main.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/6/62/BBC_News_2019.svg", Country: "UK", Quality: "1080p"},
		{ID: "intl-skynews", Name: "Sky News", Category: "News", StreamURL: "https://linear417-gb-dash1-prd-cf.cdn.sky.com/13214/1/video/hd/live.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/8/87/Sky_News_logo_2015.svg", Country: "UK", Quality: "1080p"},
		{ID: "intl-euronews-en", Name: "Euronews English", Category: "News", StreamURL: "https://euronews-euronews-world-1-au.samsung.wurl.tv/playlist.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/4b/Euronews_2016_logo.svg", Country: "EU", Quality: "1080p"},
		{ID: "intl-dw-en", Name: "Deutsche Welle (English)", Category: "News", StreamURL: "https://dwamdstream102.akamaized.net/hls/live/2015525/dwstream102/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/75/Deutsche_Welle_logo.svg", Country: "DE", Quality: "720p"},
		{ID: "intl-france24-en", Name: "France 24 English", Category: "News", StreamURL: "https://stream.france24.com/hls/en/live.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/2/23/France_24_logo.svg", Country: "FR", Quality: "720p"},
		{ID: "intl-aljazeera-en", Name: "Al Jazeera English", Category: "News", StreamURL: "https://live-hls-web-aje.getaj.net/AJE/03.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/en/f/f2/Al_Jazeera_English_logo.svg", Country: "QA", Quality: "1080p"},
		{ID: "intl-bloomberg", Name: "Bloomberg TV", Category: "News", StreamURL: "https://liveproduseast.akamaized.net/us/Channel-USTV-AWS-virginia-1/Source-4000-1080p_pa.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/40/Bloomberg_Television_logo.svg", Country: "US", Quality: "1080p"},
		{ID: "intl-nasatv", Name: "NASA TV", Category: "Science", StreamURL: "https://ntv1.akamaized.net/hls/live/2014075/NASA-NTV1-HLS/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/e5/NASA_logo.svg", Country: "US", Quality: "1080p"},
		{ID: "intl-redbulltv", Name: "Red Bull TV", Category: "Sports", StreamURL: "https://rbmn-live.akamaized.net/hls/live/590964/BoRB-AT/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/6/60/Red_Bull_TV_logo.png", Country: "AT", Quality: "1080p"},
	}

	s.channels = fallbacks
	for _, ch := range fallbacks {
		s.byID[ch.ID] = ch
	}
}

func (s *IPTVService) refreshFromIPTVOrg() {
	// Try fetching live Turkish channels M3U from iptv-org
	resp, err := s.httpClient.Get("https://iptv-org.github.io/iptv/countries/tr.m3u")
	if err != nil || resp.StatusCode != http.StatusOK {
		return
	}
	defer resp.Body.Close()

	scanner := bufio.NewScanner(resp.Body)
	var parsed []Channel
	var curName, curLogo, curGroup, curID string

	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if strings.HasPrefix(line, "#EXTINF:") {
			curName = ""
			curLogo = ""
			curGroup = "General"
			curID = ""

			// Parse tvg-logo
			if idx := strings.Index(line, `tvg-logo="`); idx != -1 {
				sub := line[idx+10:]
				if end := strings.Index(sub, `"`); end != -1 {
					curLogo = sub[:end]
				}
			}

			// Parse tvg-id
			if idx := strings.Index(line, `tvg-id="`); idx != -1 {
				sub := line[idx+8:]
				if end := strings.Index(sub, `"`); end != -1 {
					curID = sub[:end]
				}
			}

			// Parse group-title
			if idx := strings.Index(line, `group-title="`); idx != -1 {
				sub := line[idx+13:]
				if end := strings.Index(sub, `"`); end != -1 {
					curGroup = sub[:end]
				}
			}

			// Channel name is after the last comma
			if idx := strings.LastIndex(line, ","); idx != -1 {
				curName = strings.TrimSpace(line[idx+1:])
			}
		} else if strings.HasPrefix(line, "http://") || strings.HasPrefix(line, "https://") {
			if curName != "" {
				chID := curID
				if chID == "" {
					chID = fmt.Sprintf("ch-%d", len(parsed)+1)
				}
				parsed = append(parsed, Channel{
					ID:        chID,
					Name:      curName,
					LogoURL:   curLogo,
					Category:  curGroup,
					StreamURL: line,
					Country:   "TR",
					Quality:   "HD",
				})
			}
		}
	}

	if len(parsed) > 0 {
		s.mu.Lock()
		defer s.mu.Unlock()
		// Merge parsed with international channels
		for _, ch := range parsed {
			if _, exists := s.byID[ch.ID]; !exists {
				s.channels = append(s.channels, ch)
				s.byID[ch.ID] = ch
			}
		}
		s.loaded = true
	}
}
