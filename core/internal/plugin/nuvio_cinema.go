package plugin

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

// Nuvio cinema support maps a Nuvio extension's movie/series scrapers onto
// TMDB-driven catalogs plus the shared "M3U engine" playlist network
// (mooncrown04/m3ubirlestir) used by the Anthology Film/Dizi scrapers.
// Content is matched from a TMDB id, so any item produced by a TMDB-backed
// catalog (e.g. cinemasis) can resolve streams through this bridge.

const (
	nuvioTMDBBase = "https://api.themoviedb.org/3"
	nuvioTMDBKey  = "500330721680edb6d5f7f12ba7cd9023"
	nuvioImgBase  = "https://image.tmdb.org/t/p/w500"

	nuvioFilmBaseURL = "https://raw.githubusercontent.com/mooncrown04/m3ubirlestir/main/nuvio_parcalari/"
	nuvioDiziBaseURL = "https://raw.githubusercontent.com/mooncrown04/m3ubirlestir/main/nuvio_dizi_parcalari/"
)

const (
	nuvioBlockedDomains = "imagebin.pics imagehub.pics imagesbox.cloud photogrids.site picturebox.cloud " +
		"pixtureup.org pixypost.art pixtures.art imglink.info imglink.pro"
)

type nuvioTMDBMovie struct {
	ID               int     `json:"id"`
	Title            string  `json:"title"`
	OriginalTitle    string  `json:"original_title"`
	Overview         string  `json:"overview"`
	PosterPath       string  `json:"poster_path"`
	BackdropPath     string  `json:"backdrop_path"`
	ReleaseDate      string  `json:"release_date"`
	Tagline          string  `json:"tagline"`
	Runtime          int     `json:"runtime"`
	VoteAverage      float64 `json:"vote_average"`
	VoteCount        int     `json:"vote_count"`
	OriginalLanguage string  `json:"original_language"`
	IMDBID           string  `json:"imdb_id"`
	ExternalIDs      struct {
		IMDBID string `json:"imdb_id"`
	} `json:"external_ids"`
	Genres []struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"genres"`
}

type nuvioTMDBShow struct {
	ID               int    `json:"id"`
	Name             string `json:"name"`
	OriginalName     string `json:"original_name"`
	Overview         string `json:"overview"`
	PosterPath       string `json:"poster_path"`
	BackdropPath     string `json:"backdrop_path"`
	FirstAirDate     string `json:"first_air_date"`
	OriginalLanguage string `json:"original_language"`
	IMDBID           string `json:"imdb_id"`
	ExternalIDs      struct {
		IMDBID string `json:"imdb_id"`
	} `json:"external_ids"`
	Genres []struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"genres"`
	Seasons []nuvioTMDBSeason `json:"seasons"`
}

type nuvioTMDBSeason struct {
	ID           int    `json:"id"`
	Name         string `json:"name"`
	Overview     string `json:"overview"`
	SeasonNumber int    `json:"season_number"`
	EpisodeCount int    `json:"episode_count"`
}

type nuvioTMDBSearchItem struct {
	ID               int     `json:"id"`
	MediaType        string  `json:"media_type"`
	Title            string  `json:"title"`
	Name             string  `json:"name"`
	OriginalTitle    string  `json:"original_title"`
	OriginalName     string  `json:"original_name"`
	Overview         string  `json:"overview"`
	PosterPath       string  `json:"poster_path"`
	BackdropPath     string  `json:"backdrop_path"`
	ReleaseDate      string  `json:"release_date"`
	FirstAirDate     string  `json:"first_air_date"`
	VoteAverage      float64 `json:"vote_average"`
	OriginalLanguage string  `json:"original_language"`
	GenreIDs         []int   `json:"genre_ids"`
}

type nuvioTMDBSearchResp struct {
	Results []nuvioTMDBSearchItem `json:"results"`
}

type nuvioTMDBFindResp struct {
	MovieResults []nuvioTMDBMovie `json:"movie_results"`
	TVResults    []nuvioTMDBShow  `json:"tv_results"`
}

type nuvioFilmStream struct {
	Name    string
	Title   string
	URL     string
	Quality string
	Score   int
}

// m3u raw content cache (5 min TTL).
type nuvioM3UCache struct {
	mu sync.Mutex
	m  map[string]nuvioM3UCacheEntry
}

type nuvioM3UCacheEntry struct {
	body string
	at   time.Time
}

var nuvioM3UCaches = nuvioM3UCache{m: make(map[string]nuvioM3UCacheEntry)}

const nuvioM3UCacheTTL = 5 * time.Minute

func nuvioFetchM3UCached(ctx context.Context, c *http.Client, u string) (string, error) {
	nuvioM3UCaches.mu.Lock()
	if e, ok := nuvioM3UCaches.m[u]; ok && time.Since(e.at) < nuvioM3UCacheTTL {
		nuvioM3UCaches.mu.Unlock()
		return e.body, nil
	}
	nuvioM3UCaches.mu.Unlock()

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, u, nil)
	if err != nil {
		return "", err
	}
	req.Header.Set("User-Agent", "Mozilla/5.0")
	resp, err := c.Do(req)
	if err != nil {
		return "", err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return "", fmt.Errorf("fetch %s returned HTTP %d", u, resp.StatusCode)
	}
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", err
	}

	nuvioM3UCaches.mu.Lock()
	nuvioM3UCaches.m[u] = nuvioM3UCacheEntry{body: string(body), at: time.Now()}
	nuvioM3UCaches.mu.Unlock()
	return string(body), nil
}

// nuvioNormalizeMediaKey normalizes the various media id shapes (movie:155,
// tv:1399, series:1399, tmdb:155, 155, tt...) into a TMDB key + media type.
func nuvioNormalizeMediaKey(raw string) (key, mediaType string) {
	key = strings.TrimSpace(strings.TrimPrefix(strings.TrimSpace(raw), "tmdb:"))
	if idx := strings.Index(key, ":"); idx != -1 {
		prefix := strings.ToLower(key[:idx])
		rest := key[idx+1:]
		switch prefix {
		case "movie":
			return rest, "movie"
		case "tv", "series", "show", "episode":
			return rest, "tv"
		}
		return rest, ""
	}
	return key, ""
}

func nuvioParseYear(dateStr string) int32 {
	if len(dateStr) < 4 {
		return 0
	}
	y, err := strconv.Atoi(dateStr[:4])
	if err != nil {
		return 0
	}
	return int32(y)
}

func nuvioIsBlockedStream(u string) bool {
	for _, d := range strings.Fields(nuvioBlockedDomains) {
		if strings.Contains(u, d) {
			return true
		}
	}
	return false
}

func (a *NuvioAdapter) tmdbGet(ctx context.Context, endpoint string) ([]byte, error) {
	full := nuvioTMDBBase + endpoint
	if strings.Contains(endpoint, "?") {
		full += "&api_key=" + nuvioTMDBKey
	} else {
		full += "?api_key=" + nuvioTMDBKey
	}
	full += "&language=tr-TR"

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, full, nil)
	if err != nil {
		return nil, err
	}
	req.Header.Set("User-Agent", "Mozilla/5.0 (Nuvio Bridge)")
	resp, err := a.client().Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("TMDB %s returned HTTP %d", endpoint, resp.StatusCode)
	}
	return io.ReadAll(resp.Body)
}

func (a *NuvioAdapter) resolveTmdbMovie(ctx context.Context, key string) (*nuvioTMDBMovie, error) {
	if strings.HasPrefix(strings.ToLower(key), "tt") {
		body, err := a.tmdbGet(ctx, "/find/"+key+"?external_source=imdb_id")
		if err != nil {
			return nil, err
		}
		var find nuvioTMDBFindResp
		if err := json.Unmarshal(body, &find); err != nil {
			return nil, err
		}
		if len(find.MovieResults) == 0 {
			return nil, fmt.Errorf("no tmdb movie for %s", key)
		}
		return &find.MovieResults[0], nil
	}
	body, err := a.tmdbGet(ctx, "/movie/"+key+"?append_to_response=external_ids")
	if err != nil {
		return nil, err
	}
	var m nuvioTMDBMovie
	if err := json.Unmarshal(body, &m); err != nil {
		return nil, err
	}
	return &m, nil
}

func (a *NuvioAdapter) resolveTmdbShow(ctx context.Context, key string) (*nuvioTMDBShow, error) {
	if strings.HasPrefix(strings.ToLower(key), "tt") {
		body, err := a.tmdbGet(ctx, "/find/"+key+"?external_source=imdb_id")
		if err != nil {
			return nil, err
		}
		var find nuvioTMDBFindResp
		if err := json.Unmarshal(body, &find); err != nil {
			return nil, err
		}
		if len(find.TVResults) == 0 {
			return nil, fmt.Errorf("no tmdb show for %s", key)
		}
		return &find.TVResults[0], nil
	}
	body, err := a.tmdbGet(ctx, "/tv/"+key+"?append_to_response=external_ids")
	if err != nil {
		return nil, err
	}
	var s nuvioTMDBShow
	if err := json.Unmarshal(body, &s); err != nil {
		return nil, err
	}
	return &s, nil
}

func nuvioClassifyMediaType(mediaType, lang string, genreIDs []int) pluginv1.MediaType {
	isAnimation := false
	for _, id := range genreIDs {
		if id == 16 {
			isAnimation = true
			break
		}
	}
	if isAnimation && strings.EqualFold(lang, "ja") {
		return pluginv1.MediaType_MEDIA_TYPE_ANIME
	}
	if mediaType == "movie" {
		return pluginv1.MediaType_MEDIA_TYPE_MOVIE
	}
	return pluginv1.MediaType_MEDIA_TYPE_SERIES
}

// searchCinema performs a TMDB-backed catalog search for the cinema bridge.
func (a *NuvioAdapter) searchCinema(ctx context.Context, query string, page int32) (*pluginv1.SearchResponse, error) {
	if page < 1 {
		page = 1
	}
	var results []nuvioTMDBSearchItem

	q := strings.TrimSpace(query)
	switch strings.ToLower(q) {
	case "", "popular", "featured", "trending":
		for _, ep := range []string{"/movie/popular", "/tv/popular"} {
			body, err := a.tmdbGet(ctx, fmt.Sprintf("%s?page=%d", ep, page))
			if err != nil {
				continue
			}
			var resp nuvioTMDBSearchResp
			if json.Unmarshal(body, &resp) == nil {
				results = append(results, resp.Results...)
			}
		}
	default:
		body, err := a.tmdbGet(ctx, fmt.Sprintf("/search/multi?query=%s&page=%d", url.QueryEscape(q), page))
		if err != nil {
			return nil, err
		}
		var resp nuvioTMDBSearchResp
		if err := json.Unmarshal(body, &resp); err != nil {
			return nil, err
		}
		results = resp.Results
	}

	var items []*pluginv1.MediaItem
	for _, r := range results {
		mediaType := strings.ToLower(r.MediaType)
		if mediaType != "movie" && mediaType != "tv" {
			continue
		}
		title := r.Title
		orig := r.OriginalTitle
		release := r.ReleaseDate
		if mediaType == "tv" {
			title = r.Name
			orig = r.OriginalName
			release = r.FirstAirDate
		}
		items = append(items, &pluginv1.MediaItem{
			Id:        fmt.Sprintf("%s:%d", mediaType, r.ID),
			Title:     title,
			Type:      nuvioClassifyMediaType(mediaType, r.OriginalLanguage, r.GenreIDs),
			Year:      nuvioParseYear(release),
			PosterUrl: nuvioImgBase + r.PosterPath,
			Overview:  r.Overview,
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(r.ID),
				Extra: map[string]string{
					"backdrop_url":  nuvioBackdropURL(r.BackdropPath),
					"rating":        strconv.FormatFloat(r.VoteAverage, 'f', 1, 64),
					"original_title": orig,
				},
			},
		})
	}

	return &pluginv1.SearchResponse{Items: items, HasMore: false}, nil
}

func nuvioBackdropURL(path string) string {
	if path == "" {
		return ""
	}
	return "https://image.tmdb.org/t/p/w780" + path
}

func (a *NuvioAdapter) getCinemaMetadata(ctx context.Context, mediaID string) (*pluginv1.GetMetadataResponse, error) {
	key, mediaType := nuvioNormalizeMediaKey(mediaID)
	if key == "" {
		return nil, fmt.Errorf("invalid media id: %s", mediaID)
	}
	if mediaType == "tv" {
		s, err := a.resolveTmdbShow(ctx, key)
		if err != nil {
			return nil, err
		}

		var genres []string
		for _, g := range s.Genres {
			genres = append(genres, g.Name)
		}
		imdb := s.ExternalIDs.IMDBID
		if imdb == "" {
			imdb = s.IMDBID
		}
		details := &pluginv1.MediaDetails{
			Id:        mediaID,
			Title:     s.Name,
			Type:      nuvioClassifyMediaType("tv", s.OriginalLanguage, nil),
			Year:      nuvioParseYear(s.FirstAirDate),
			PosterUrl: nuvioImgBase + s.PosterPath,
			Overview:  s.Overview,
			Genres:    genres,
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(s.ID),
				ImdbId: imdb,
				Extra: map[string]string{
					"backdrop_url": nuvioBackdropURL(s.BackdropPath),
				},
			},
		}
		for _, ssn := range s.Seasons {
			if ssn.SeasonNumber <= 0 {
				continue
			}
			season := &pluginv1.Season{
				SeasonNumber: int32(ssn.SeasonNumber),
				Title:        ssn.Name,
			}
			for e := 1; e <= ssn.EpisodeCount; e++ {
				season.Episodes = append(season.Episodes, &pluginv1.Episode{
					EpisodeNumber: int32(e),
				})
			}
			details.Seasons = append(details.Seasons, season)
		}
		return &pluginv1.GetMetadataResponse{Details: details}, nil
	}

	m, err := a.resolveTmdbMovie(ctx, key)
	if err != nil {
		return nil, err
	}

	var genres []string
	for _, g := range m.Genres {
		genres = append(genres, g.Name)
	}
	imdb := m.ExternalIDs.IMDBID
	if imdb == "" {
		imdb = m.IMDBID
	}
	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:        mediaID,
			Title:     m.Title,
			Type:      nuvioClassifyMediaType("movie", m.OriginalLanguage, nil),
			Year:      nuvioParseYear(m.ReleaseDate),
			PosterUrl: nuvioImgBase + m.PosterPath,
			Overview:  m.Overview,
			Genres:    genres,
			ExternalIds: &pluginv1.ExternalIDs{
				TmdbId: strconv.Itoa(m.ID),
				ImdbId: imdb,
				Extra: map[string]string{
					"backdrop_url": nuvioBackdropURL(m.BackdropPath),
					"rating":       strconv.FormatFloat(m.VoteAverage, 'f', 1, 64),
				},
			},
		},
	}, nil
}

func (a *NuvioAdapter) getCinemaStreams(ctx context.Context, mediaID string, season, episode int32) (*pluginv1.GetStreamsResponse, error) {
	key, mediaType := nuvioNormalizeMediaKey(mediaID)
	if key == "" {
		return nil, fmt.Errorf("invalid media id: %s", mediaID)
	}

	isShow := mediaType == "tv" || season > 0 || episode > 0

	var results []nuvioFilmStream
	if isShow {
		results = a.searchDiziStreams(ctx, key, season, episode)
		if len(results) == 0 {
			results = a.searchFilmStreams(ctx, key)
		}
	} else {
		results = a.searchFilmStreams(ctx, key)
		if len(results) == 0 {
			results = a.searchDiziStreams(ctx, key, season, episode)
		}
	}

	resp := &pluginv1.GetStreamsResponse{}
	for _, s := range results {
		resp.Streams = append(resp.Streams, &pluginv1.StreamSource{
			Id:      fmt.Sprintf("nuvio-m3u-%d", len(resp.Streams)+1),
			Title:   s.Title,
			Url:     s.URL,
			Format:  nuvioStreamFormat(s.URL),
			Quality: s.Quality,
			Headers: map[string]string{
				"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML like Gecko) Chrome/120.0.0.0 Safari/537.36",
			},
		})
	}
	return resp, nil
}

func nuvioStreamFormat(u string) pluginv1.StreamFormat {
	lower := strings.ToLower(u)
	switch {
	case strings.HasSuffix(lower, ".mp4"):
		return pluginv1.StreamFormat_STREAM_FORMAT_MP4
	case strings.HasSuffix(lower, ".mkv"):
		return pluginv1.StreamFormat_STREAM_FORMAT_MKV
	default:
		return pluginv1.StreamFormat_STREAM_FORMAT_HLS
	}
}

// --- M3U engine (port of anthology providers/m3u_engine.js) ---

func nuvioUltraClean(s string) string {
	r := strings.ToLower(strings.TrimSpace(s))
	r = strings.NewReplacer("ı", "i", "İ", "i", "ü", "u", "Ü", "u",
		"ö", "o", "Ö", "o", "ş", "s", "Ş", "s", "ğ", "g", "Ğ", "g",
		"ç", "c", "Ç", "c", "â", "a", "Â", "a", "î", "i", "Î", "i", "û", "u", "Û", "u").Replace(r)
	return nuvioKeepAlnum(r)
}

func nuvioKeepAlnum(s string) string {
	var sb strings.Builder
	for _, r := range s {
		if r >= 'a' && r <= 'z' || r >= '0' && r <= '9' {
			sb.WriteRune(r)
		}
	}
	return strings.TrimSpace(sb.String())
}

func nuvioGetLetterGroup(title string) string {
	clean := nuvioUltraClean(title)
	if clean == "" {
		return "diger"
	}
	c := clean[0]
	if c >= '0' && c <= '9' {
		return "0_9_rakam"
	}
	if c >= 'a' && c <= 'z' {
		return string(c)
	}
	return "diger"
}

func nuvioTitleTokens(s string) []string {
	folded := strings.NewReplacer("ı", "i", "İ", "i", "ü", "u", "Ü", "u",
		"ö", "o", "Ö", "o", "ş", "s", "Ş", "s", "ğ", "g", "Ğ", "g",
		"ç", "c", "Ç", "c", "â", "a", "Â", "a", "î", "i", "Î", "i", "û", "u", "Û", "u").Replace(strings.ToLower(s))
	var out []string
	for _, tok := range strings.FieldsFunc(folded, func(r rune) bool {
		return !(r >= 'a' && r <= 'z' || r >= '0' && r <= '9')
	}) {
		t := nuvioKeepAlnum(tok)
		if len(t) >= 3 {
			out = append(out, t)
		}
	}
	return out
}

func nuvioTitleBoundaryMatch(rawName string, targets []string) bool {
	if rawName == "" {
		return false
	}
	nameTokens := nuvioTitleTokens(strings.TrimSpace(strings.Split(strings.Split(rawName, "(")[0], "-")[0]))
	if len(nameTokens) == 0 {
		return false
	}
	for _, t := range targets {
		if t == "" {
			continue
		}
		targetTokens := nuvioTitleTokens(t)
		if len(targetTokens) == 0 {
			continue
		}
		for len(targetTokens) > 0 {
			first := targetTokens[0]
			if first == "the" || first == "a" || first == "an" {
				targetTokens = targetTokens[1:]
				continue
			}
			break
		}
		if len(targetTokens) == 0 {
			continue
		}
		if len(targetTokens) == 1 && len(targetTokens[0]) < 4 {
			continue
		}
		if len(targetTokens) > len(nameTokens) {
			continue
		}
		ok := true
		for i := range targetTokens {
			if nameTokens[i] != targetTokens[i] {
				ok = false
				break
			}
		}
		if ok {
			return true
		}
	}
	return false
}

var (
	nuvioAuthorRe  = regexp.MustCompile(`group-author="([^"]+)"`)
	nuvioGroupRe   = regexp.MustCompile(`group-title="([^"]+)"`)
	nuvioYearAttrRe = regexp.MustCompile(`year="(\d{4})"`)
	nuvioYearRawRe  = regexp.MustCompile(`\b(19|20)\d{2}\b`)
)

func nuvioSourceTag(line string) string {
	if m := nuvioAuthorRe.FindStringSubmatch(line); len(m) == 2 {
		return strings.Trim(strings.TrimSpace(m[1]), "[]")
	}
	return "M3U"
}

func nuvioRawName(line string) string {
	idx := strings.LastIndex(line, ",")
	if idx == -1 {
		return ""
	}
	return strings.TrimSpace(line[idx+1:])
}

func nuvioM3UYear(line, rawName string) string {
	if m := nuvioYearAttrRe.FindStringSubmatch(line); len(m) == 2 {
		return m[1]
	}
	if m := nuvioYearRawRe.FindString(rawName); m != "" {
		return m
	}
	return ""
}

func (a *NuvioAdapter) searchFilmStreams(ctx context.Context, key string) []nuvioFilmStream {
	d, err := a.resolveTmdbMovie(ctx, key)
	if err != nil {
		return nil
	}
	if d.Title == "" && d.OriginalTitle == "" {
		return nil
	}

	targetImdb := d.ExternalIDs.IMDBID
	if targetImdb == "" {
		targetImdb = d.IMDBID
	}
	targetTr := nuvioUltraClean(d.Title)
	targetEn := nuvioUltraClean(d.OriginalTitle)
	targetYear := ""
	if len(d.ReleaseDate) >= 4 {
		targetYear = d.ReleaseDate[:4]
	}

	var results []nuvioFilmStream
	seenGrp := map[string]bool{}
	seenURL := map[string]bool{}
	for _, grp := range []string{
		nuvioGetLetterGroup(d.Title),
		nuvioGetLetterGroup(d.OriginalTitle),
		"0_9_rakam",
		"diger",
	} {
		if grp == "" || seenGrp[grp] {
			continue
		}
		seenGrp[grp] = true
		content, err := nuvioFetchM3UCached(ctx, a.client(), nuvioFilmBaseURL+"nuvio_"+grp+".m3u")
		if err != nil {
			continue
		}
		lines := strings.Split(content, "\n")
		for i := 0; i < len(lines); i++ {
			line := strings.TrimSpace(lines[i])
			if !strings.HasPrefix(line, "#EXTINF") {
				continue
			}
			if i+1 >= len(lines) {
				continue
			}
			streamURL := strings.TrimSpace(lines[i+1])
			if !strings.HasPrefix(streamURL, "http") {
				continue
			}
			if nuvioIsBlockedStream(streamURL) || seenURL[streamURL] {
				continue
			}

			rawName := nuvioRawName(line)
			m3uYear := nuvioM3UYear(line, rawName)
			cleanM3U := nuvioUltraClean(strings.TrimSpace(strings.Split(strings.Split(rawName, "(")[0], "-")[0]))

			isMatch := false
			score := 0
			if targetImdb != "" && strings.Contains(streamURL, targetImdb) {
				isMatch = true
				score = 120
			} else if cleanM3U == targetTr || cleanM3U == targetEn {
				if m3uYear == "" || m3uYear == targetYear {
					isMatch = true
					score = 90
					if m3uYear == targetYear {
						score = 100
					}
				}
			} else if len(cleanM3U) > 3 && nuvioTitleBoundaryMatch(rawName, []string{d.Title, d.OriginalTitle}) {
				if m3uYear == "" || m3uYear == targetYear {
					isMatch = true
					score = 80
				}
			}

			if isMatch && score >= 70 {
				seenURL[streamURL] = true
				year := m3uYear
				if year == "" {
					year = targetYear
				}
				title := d.Title
				if title == "" {
					title = d.OriginalTitle
				}
				results = append(results, nuvioFilmStream{
					Name:    fmt.Sprintf("%s (%s)", title, year),
					Title:   fmt.Sprintf("Anthology Film | %s [HD]", nuvioSourceTag(line)),
					URL:     streamURL,
					Quality: nuvioSourceTag(line),
					Score:   score,
				})
			}
		}
	}

	sort.SliceStable(results, func(i, j int) bool { return results[i].Score > results[j].Score })
	return results
}

func (a *NuvioAdapter) searchDiziStreams(ctx context.Context, key string, season, episode int32) []nuvioFilmStream {
	finalSeason := int(season)
	if finalSeason < 1 {
		finalSeason = 1
	}
	finalEpisode := int(episode)
	if finalEpisode < 1 {
		finalEpisode = 1
	}

	d, err := a.resolveTmdbShow(ctx, key)
	if err != nil {
		return nil
	}
	if d.Name == "" && d.OriginalName == "" {
		return nil
	}

	sPad := fmt.Sprintf("%02d", finalSeason)
	ePad := fmt.Sprintf("%02d", finalEpisode)
	searchPatterns := []string{
		"s" + sPad + "e" + ePad,
		"s" + sPad + " e" + ePad,
		fmt.Sprintf("s%de%d", finalSeason, finalEpisode),
		fmt.Sprintf("s%d e%d", finalSeason, finalEpisode),
		fmt.Sprintf("%dx%s", finalSeason, ePad),
		fmt.Sprintf("%dx%d", finalSeason, finalEpisode),
		fmt.Sprintf("bolum%d", finalEpisode),
		fmt.Sprintf("bolum %d", finalEpisode),
	}

	var results []nuvioFilmStream
	seenGrp := map[string]bool{}
	seenURL := map[string]bool{}
	for _, grp := range []string{
		nuvioGetLetterGroup(d.Name),
		nuvioGetLetterGroup(d.OriginalName),
		"0_9_rakam",
		"diger",
	} {
		if grp == "" || seenGrp[grp] {
			continue
		}
		seenGrp[grp] = true
		fileName := fmt.Sprintf("dizi_%s_s%d.m3u", grp, finalSeason)
		content, err := nuvioFetchM3UCached(ctx, a.client(), nuvioDiziBaseURL+fileName)
		if err != nil {
			continue
		}
		lines := strings.Split(content, "\n")
		for i := 0; i < len(lines); i++ {
			line := strings.TrimSpace(lines[i])
			if !strings.HasPrefix(line, "#EXTINF") {
				continue
			}
			if i+1 >= len(lines) {
				continue
			}
			streamURL := strings.TrimSpace(lines[i+1])
			if !strings.HasPrefix(streamURL, "http") {
				continue
			}
			if nuvioIsBlockedStream(streamURL) || seenURL[streamURL] {
				continue
			}

			rawName := nuvioRawName(line)
			if !nuvioTitleBoundaryMatch(rawName, []string{d.Name, d.OriginalName}) {
				continue
			}
			cleanLine := nuvioUltraClean(line)
			epMatch := false
			for _, pat := range searchPatterns {
				if strings.Contains(cleanLine, nuvioUltraClean(pat)) {
					epMatch = true
					break
				}
			}
			if !epMatch {
				continue
			}

			seenURL[streamURL] = true
			name := d.Name
			if name == "" {
				name = d.OriginalName
			}
			results = append(results, nuvioFilmStream{
				Name:    fmt.Sprintf("%s S%02dE%02d", name, finalSeason, finalEpisode),
				Title:   fmt.Sprintf("Anthology Dizi | %s [HD]", nuvioSourceTag(line)),
				URL:     streamURL,
				Quality: nuvioSourceTag(line),
				Score:   100,
			})
		}
	}
	return results
}