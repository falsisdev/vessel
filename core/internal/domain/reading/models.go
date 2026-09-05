package reading

import "github.com/falsisdev/vessel/core/internal/domain/cinema"

type ReadingType int

const (
	ReadingTypeUnspecified ReadingType = iota
	ReadingTypeManga
	ReadingTypeWebtoon
	ReadingTypeWebook
	ReadingTypeBook
)

func (t ReadingType) String() string {
	switch t {
	case ReadingTypeManga:
		return "Manga"
	case ReadingTypeWebtoon:
		return "Webtoon"
	case ReadingTypeWebook:
		return "Webook"
	case ReadingTypeBook:
		return "Book"
	default:
		return "Unspecified"
	}
}

type ReadingItem struct {
	ID          string
	ProviderID  string
	Title       string
	Type        ReadingType
	Year        int32
	PosterURL   string
	Overview    string
	ExternalIDs cinema.ExternalIDs
}

type Chapter struct {
	ID            string
	ChapterNumber float64
	VolumeNumber  float64
	Title         string
}

type ReadingDetails struct {
	ID          string
	ProviderID  string
	Title       string
	Type        ReadingType
	Year        int32
	PosterURL   string
	Overview    string
	Genres      []string
	Chapters    []Chapter
	ExternalIDs cinema.ExternalIDs
}

type Page struct {
	PageNumber int32
	URL        string
	Headers    map[string]string
}

type ChapterContent struct {
	ChapterID     string
	Title         string
	ChapterNumber float64
	Pages         []Page
	TextContent   string
}
