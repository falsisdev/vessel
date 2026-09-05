package provider

import (
	"bufio"
	"context"
	"fmt"
	"net/http"
	"regexp"
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
			Description:     "Open-source live television channels and streams worldwide (iptv-org)",
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

	rawQ := strings.TrimSpace(strings.ToLower(req.Query))
	countryFilter := ""
	catFilter := ""
	textQ := rawQ

	for _, token := range strings.Fields(rawQ) {
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

	var matches []Channel
	for _, ch := range s.channels {
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

func (s *IPTVService) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	ch, exists := s.byID[req.MediaId]
	if !exists {
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
				Title:   fmt.Sprintf("%s (Live HLS)", ch.Name),
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
		// Global / International Top Channels (shown first for ALL)
		{ID: "uk-bbcnews", Name: "BBC News", Category: "News", StreamURL: "https://vs-hls-push-ww-live.akamaized.net/x=4/i=urn:bbc:pips:service:bbc_news_channel_hd/t=3840/v=pv14/b=5070016/main.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/6/62/BBC_News_2019.svg", Country: "UK", Quality: "1080p"},
		{ID: "us-bloomberg", Name: "Bloomberg TV", Category: "News", StreamURL: "https://liveproduseast.akamaized.net/us/Channel-USTV-AWS-virginia-1/Source-4000-1080p_pa.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/40/Bloomberg_Television_logo.svg", Country: "US", Quality: "1080p"},
		{ID: "uk-skynews", Name: "Sky News", Category: "News", StreamURL: "https://linear417-gb-dash1-prd-cf.cdn.sky.com/13214/1/video/hd/live.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/8/87/Sky_News_logo_2015.svg", Country: "UK", Quality: "1080p"},
		{ID: "fr-france24", Name: "France 24", Category: "News", StreamURL: "https://stream.france24.com/hls/en/live.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/2/23/France_24_logo.svg", Country: "FR", Quality: "720p"},
		{ID: "de-dw", Name: "Deutsche Welle", Category: "News", StreamURL: "https://dwamdstream102.akamaized.net/hls/live/2015525/dwstream102/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/75/Deutsche_Welle_logo.svg", Country: "DE", Quality: "720p"},
		{ID: "us-cbsnews", Name: "CBS News Live", Category: "News", StreamURL: "https://cbsn-us.cbsnstream.cbsnews.com/out/v1/55a8648e8f134e82a470f83d562de701/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/1/19/CBS_News_logo_2020.svg", Country: "US", Quality: "1080p"},
		{ID: "us-abcnews", Name: "ABC News Live", Category: "News", StreamURL: "https://content.uplynk.com/channel/3324f2467c414329b3b0cc5cd987b6be.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/6/60/ABC_News_Live_logo.svg", Country: "US", Quality: "1080p"},
		{ID: "us-nasatv", Name: "NASA TV", Category: "Science", StreamURL: "https://ntv1.akamaized.net/hls/live/2014075/NASA-NTV1-HLS/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/e5/NASA_logo.svg", Country: "US", Quality: "1080p"},
		{ID: "fr-euronews", Name: "Euronews Français", Category: "News", StreamURL: "https://euronews-euronews-world-1-au.samsung.wurl.tv/playlist.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/4b/Euronews_2016_logo.svg", Country: "FR", Quality: "1080p"},
		{ID: "tr-trtworld", Name: "TRT World", Category: "News", StreamURL: "https://tv-trtworld.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/8/87/TRT_World_logo.svg", Country: "TR", Quality: "1080p"},
		{ID: "jp-nhk", Name: "NHK World Japan", Category: "News", StreamURL: "https://nhkwlive-ojproto.akamaized.net/hls/live/2003459/nhkwlive-oj-en/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/7b/NHK_World-Japan_logo.svg", Country: "JP", Quality: "1080p"},

		// Turkey (TR) Channels
		{ID: "tr-trt1", Name: "TRT 1", Category: "General", StreamURL: "https://tv-trt1.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/47/TRT_1_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trthaber", Name: "TRT Haber", Category: "News", StreamURL: "https://tv-trthaber.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/e0/TRT_Haber_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trtspore", Name: "TRT Spor", Category: "Sports", StreamURL: "https://tv-trtspor.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/2/23/TRT_Spor_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-trtbelgesel", Name: "TRT Belgesel", Category: "Documentary", StreamURL: "https://tv-trtbelgesel.medya.trt.com.tr/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/b/b8/TRT_Belgesel_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-ahaber", Name: "A Haber", Category: "News", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/ahaber/ahaber.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/7c/Ahaber_Logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-aspor", Name: "A Spor", Category: "Sports", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/aspor/aspor.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/ea/A_Spor_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-atv", Name: "ATV", Category: "General", StreamURL: "https://rnttwmjcin.turknet.ercdn.net/lcpmvefbyo/atv/atv_1080p.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/4c/Atv_logo.png", Country: "TR", Quality: "1080p"},
		{ID: "tr-tv360", Name: "360 TV", Category: "General", StreamURL: "https://turkmedya-live.ercdn.net/tv360/tv360.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/4/41/360_TV_logo.png", Country: "TR", Quality: "720p"},

		// Azerbaijan (AZ) Channels
		{ID: "az-aztv", Name: "AzTV", Category: "General", StreamURL: "https://stream.aztv.az/live/aztv/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/5/52/AzTV_logo.png", Country: "AZ", Quality: "1080p"},
		{ID: "az-ictimai", Name: "İctimai TV", Category: "General", StreamURL: "https://live.itv.az/hls/live.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/7/7a/%C4%B0ctimai_Television_logo.png", Country: "AZ", Quality: "1080p"},
		{ID: "az-idman", Name: "İdman TV", Category: "Sports", StreamURL: "https://stream.aztv.az/live/idman/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/2/2a/Idman_Azerbaycan_TV_logo.png", Country: "AZ", Quality: "720p"},
		{ID: "az-medeniyyet", Name: "Mədəniyyət TV", Category: "Documentary", StreamURL: "https://stream.aztv.az/live/medeniyyet/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/a/a2/Medeniyyet_TV_logo.png", Country: "AZ", Quality: "720p"},

		// Germany (DE) Channels
		{ID: "de-zdf", Name: "ZDF Info", Category: "Documentary", StreamURL: "https://zdf-hls-17.akamaized.net/hls/live/2016500/de/veryhigh/master.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/a/af/ZDFinfo_logo_2021.svg", Country: "DE", Quality: "1080p"},

		// France (FR) Channels
		{ID: "fr-arte", Name: "Arte TV", Category: "Culture", StreamURL: "https://artesimulcast.akamaized.net/hls/live/2031003/artelive_de/index.m3u8", LogoURL: "https://upload.wikimedia.org/wikipedia/commons/e/e8/Arte_logo_2017.svg", Country: "FR", Quality: "720p"},
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
				curName = cleanChannelTitle(curName)
			}
		} else if strings.HasPrefix(line, "http://") || strings.HasPrefix(line, "https://") {
			if curName != "" {
				chID := curID
				if chID == "" {
					chID = fmt.Sprintf("ch-%d", len(parsed)+1)
				}
				curLogo = resolveVerifiedLogo(curName, curLogo)
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

var (
	reResolution = regexp.MustCompile(`(?i)\s*\(\d+p\)`)
	reBracketTag = regexp.MustCompile(`(?i)\s*\[[^\]]*\]`)
)

func cleanChannelTitle(raw string) string {
	cleaned := reResolution.ReplaceAllString(raw, "")
	cleaned = reBracketTag.ReplaceAllString(cleaned, "")
	return strings.TrimSpace(cleaned)
}

func resolveVerifiedLogo(name, existingLogo string) string {
	lower := strings.ToLower(name)

	// Direct High-Resolution Wikimedia Verified Logos Map
	verifiedLogos := map[string]string{
		"trt 1":        "https://upload.wikimedia.org/wikipedia/commons/4/47/TRT_1_logo.png",
		"trt haber":    "https://upload.wikimedia.org/wikipedia/commons/e/e0/TRT_Haber_logo.png",
		"trt spor":     "https://upload.wikimedia.org/wikipedia/commons/2/23/TRT_Spor_logo.png",
		"trt belgesel": "https://upload.wikimedia.org/wikipedia/commons/b/b8/TRT_Belgesel_logo.png",
		"trt world":    "https://upload.wikimedia.org/wikipedia/commons/8/87/TRT_World_logo.svg",
		"trt müzik":    "https://upload.wikimedia.org/wikipedia/commons/a/a9/TRT_M%C3%BCzik_logo.png",
		"trt çocuk":    "https://upload.wikimedia.org/wikipedia/commons/4/47/TRT_%C3%87ocuk_logo.png",
		"atv":          "https://upload.wikimedia.org/wikipedia/commons/4/4c/Atv_logo.png",
		"a haber":      "https://upload.wikimedia.org/wikipedia/commons/7/7c/Ahaber_Logo.png",
		"a spor":       "https://upload.wikimedia.org/wikipedia/commons/e/ea/A_Spor_logo.png",
		"360 tv":       "https://upload.wikimedia.org/wikipedia/commons/4/41/360_TV_logo.png",
		"360":          "https://upload.wikimedia.org/wikipedia/commons/4/41/360_TV_logo.png",
		"tv8":          "https://upload.wikimedia.org/wikipedia/commons/1/14/Tv8_logo.png",
		"show tv":      "https://upload.wikimedia.org/wikipedia/commons/4/41/Show_TV_logo_2014.png",
		"kanal d":      "https://upload.wikimedia.org/wikipedia/commons/1/1c/Kanal_D_logo_2018.png",
		"star tv":      "https://upload.wikimedia.org/wikipedia/commons/7/72/Star_TV_logo_2011.png",
		"now":          "https://upload.wikimedia.org/wikipedia/commons/4/49/NOW_T%C3%BCrkiye_logo.png",
		"now tv":       "https://upload.wikimedia.org/wikipedia/commons/4/49/NOW_T%C3%BCrkiye_logo.png",
		"cnn türk":     "https://upload.wikimedia.org/wikipedia/commons/2/24/CNN_T%C3%BCrk_logo.png",
		"habertürk":    "https://upload.wikimedia.org/wikipedia/commons/6/65/Habert%C3%BCrk_TV_logo.png",
		"ntv":          "https://upload.wikimedia.org/wikipedia/commons/6/64/NTV_logo.png",
		"halk tv":      "https://upload.wikimedia.org/wikipedia/commons/2/2a/Halk_TV_logo_2020.png",
		"tele1":        "https://upload.wikimedia.org/wikipedia/commons/5/52/Tele1_logo.png",
		"beyaz tv":     "https://upload.wikimedia.org/wikipedia/commons/8/87/Beyaz_TV_logo.png",
		"aztv":         "https://upload.wikimedia.org/wikipedia/commons/5/52/AzTV_logo.png",
		"i̇ctimai tv":   "https://upload.wikimedia.org/wikipedia/commons/7/7a/%C4%B0ctimai_Television_logo.png",
		"ictimai tv":   "https://upload.wikimedia.org/wikipedia/commons/7/7a/%C4%B0ctimai_Television_logo.png",
		"i̇dman tv":     "https://upload.wikimedia.org/wikipedia/commons/2/2a/Idman_Azerbaycan_TV_logo.png",
		"idman tv":     "https://upload.wikimedia.org/wikipedia/commons/2/2a/Idman_Azerbaycan_TV_logo.png",
		"bbc news":     "https://upload.wikimedia.org/wikipedia/commons/6/62/BBC_News_2019.svg",
		"bloomberg tv": "https://upload.wikimedia.org/wikipedia/commons/4/40/Bloomberg_Television_logo.svg",
		"bloomberg":    "https://upload.wikimedia.org/wikipedia/commons/4/40/Bloomberg_Television_logo.svg",
		"sky news":     "https://upload.wikimedia.org/wikipedia/commons/8/87/Sky_News_logo_2015.svg",
		"france 24":    "https://upload.wikimedia.org/wikipedia/commons/2/23/France_24_logo.svg",
		"deutsche welle": "https://upload.wikimedia.org/wikipedia/commons/7/75/Deutsche_Welle_logo.svg",
		"cbs news":     "https://upload.wikimedia.org/wikipedia/commons/1/19/CBS_News_logo_2020.svg",
		"abc news":     "https://upload.wikimedia.org/wikipedia/commons/6/60/ABC_News_Live_logo.svg",
		"nasa tv":      "https://upload.wikimedia.org/wikipedia/commons/e/e5/NASA_logo.svg",
		"euronews":     "https://upload.wikimedia.org/wikipedia/commons/4/4b/Euronews_2016_logo.svg",
		"nhk world":    "https://upload.wikimedia.org/wikipedia/commons/7/7b/NHK_World-Japan_logo.svg",
		"zdf":          "https://upload.wikimedia.org/wikipedia/commons/a/af/ZDFinfo_logo_2021.svg",
		"arte":         "https://upload.wikimedia.org/wikipedia/commons/e/e8/Arte_logo_2017.svg",
	}

	for k, logo := range verifiedLogos {
		if strings.Contains(lower, k) {
			return logo
		}
	}

	if existingLogo != "" {
		return existingLogo
	}
	return ""
}
