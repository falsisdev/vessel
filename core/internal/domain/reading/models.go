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
	ID          string             `json:"id"`
	ProviderID  string             `json:"provider_id"`
	Title       string             `json:"title"`
	Type        ReadingType        `json:"type"`
	Year        int32              `json:"year"`
	PosterURL   string             `json:"poster_url"`
	Overview    string             `json:"overview"`
	ExternalIDs cinema.ExternalIDs `json:"external_ids"`
}

type Chapter struct {
	ID            string  `json:"id"`
	ChapterNumber float64 `json:"chapter_number"`
	VolumeNumber  float64 `json:"volume_number"`
	Title         string  `json:"title"`
}

type ReadingDetails struct {
	ID          string             `json:"id"`
	ProviderID  string             `json:"provider_id"`
	Title       string             `json:"title"`
	Type        ReadingType        `json:"type"`
	Year        int32              `json:"year"`
	PosterURL   string             `json:"poster_url"`
	Overview    string             `json:"overview"`
	Genres      []string           `json:"genres"`
	Chapters    []Chapter          `json:"chapters"`
	ExternalIDs cinema.ExternalIDs `json:"external_ids"`
}

type Page struct {
	PageNumber int32             `json:"page_number"`
	URL        string            `json:"url"`
	Headers    map[string]string `json:"headers,omitempty"`
}

type ChapterContent struct {
	ChapterID     string   `json:"chapter_id"`
	Title         string   `json:"title"`
	ChapterNumber float64  `json:"chapter_number"`
	Pages         []Page   `json:"pages"`
	TextContent   string   `json:"text_content"`
}

