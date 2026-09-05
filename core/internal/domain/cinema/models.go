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
	IMDbID    string
	TMDBID    string
	SIMKLID   string
	MALID     string
	AniListID string
	KitsuID   string
	SanityID  string
	Extra     map[string]string
}

type MediaItem struct {
	ID          string
	ProviderID  string
	Title       string
	Type        MediaType
	Year        int32
	PosterURL   string
	Overview    string
	ExternalIDs ExternalIDs
}

type Episode struct {
	EpisodeNumber   int32
	Title           string
	Overview        string
	DurationSeconds int64
}

type Season struct {
	SeasonNumber int32
	Title        string
	Episodes     []Episode
}

type MediaDetails struct {
	ID          string
	ProviderID  string
	Title       string
	Type        MediaType
	Year        int32
	PosterURL   string
	Overview    string
	Genres      []string
	Seasons     []Season
	ExternalIDs ExternalIDs
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
