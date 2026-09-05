package cinema

type MediaType int

const (
	MediaTypeUnspecified MediaType = iota
	MediaTypeMovie
	MediaTypeSeries
	MediaTypeAnime
)

func (t MediaType) String() string {
	switch t {
	case MediaTypeMovie:
		return "Movie"
	case MediaTypeSeries:
		return "Series"
	case MediaTypeAnime:
		return "Anime"
	default:
		return "Unspecified"
	}
}

type ExternalIDs struct {
	IMDbID    string            `json:"imdb_id"`
	TMDBID    string            `json:"tmdb_id"`
	SIMKLID   string            `json:"simkl_id"`
	MALID     string            `json:"mal_id"`
	AniListID string            `json:"anilist_id"`
	KitsuID   string            `json:"kitsu_id"`
	SanityID  string            `json:"sanity_id"`
	Extra     map[string]string `json:"extra,omitempty"`
}

type MediaItem struct {
	ID          string      `json:"id"`
	ProviderID  string      `json:"provider_id"`
	Title       string      `json:"title"`
	Type        MediaType   `json:"type"`
	Year        int32       `json:"year"`
	PosterURL   string      `json:"poster_url"`
	Overview    string      `json:"overview"`
	ExternalIDs ExternalIDs `json:"external_ids"`
}

type Episode struct {
	EpisodeNumber   int32  `json:"episode_number"`
	Title           string `json:"title"`
	Overview        string `json:"overview"`
	DurationSeconds int64  `json:"duration_seconds"`
}

type Season struct {
	SeasonNumber int32     `json:"season_number"`
	Title        string    `json:"title"`
	Episodes     []Episode `json:"episodes"`
}

type MediaDetails struct {
	ID          string      `json:"id"`
	ProviderID  string      `json:"provider_id"`
	Title       string      `json:"title"`
	Type        MediaType   `json:"type"`
	Year        int32       `json:"year"`
	PosterURL   string      `json:"poster_url"`
	Overview    string      `json:"overview"`
	Genres      []string    `json:"genres"`
	Seasons     []Season    `json:"seasons"`
	ExternalIDs ExternalIDs `json:"external_ids"`
}

type StreamFormat int

const (
	StreamFormatUnspecified StreamFormat = iota
	StreamFormatHLS
	StreamFormatDASH
	StreamFormatMP4
	StreamFormatMKV
)

type SubtitleFormat int

const (
	SubtitleFormatUnspecified SubtitleFormat = iota
	SubtitleFormatVTT
	SubtitleFormatSRT
)

type StreamSource struct {
	ID         string
	ProviderID string
	Title      string
	URL        string
	Format     StreamFormat
	Quality    string
	Headers    map[string]string
}

type Subtitle struct {
	Language  string
	Label     string
	URL       string
	Format    SubtitleFormat
	IsDefault bool
}
