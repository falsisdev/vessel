package sanity

import (
	"strings"
	"time"
)

type SanityTitle struct {
	ID            string                 `json:"_id"`
	Type          string                 `json:"_type"`
	CreatedAt     time.Time              `json:"_createdAt"`
	UpdatedAt     time.Time              `json:"_updatedAt"`
	Title         string                 `json:"title"`
	Slug          string                 `json:"slug"`
	Description   string                 `json:"description"`
	MyAnimeListID int                    `json:"myAnimeListId"`
	UploadStatus  string                 `json:"uploadStatus"`
	Tags          []string               `json:"tags"`
	Format        string                 `json:"format"`
	CoverImage    string                 `json:"coverImage"`
	BannerImage   string                 `json:"bannerImage"`
	Chapters      []SanityChapterSummary `json:"chapters"`
}

type SanityChapterSummary struct {
	ID            string  `json:"_id"`
	Title         string  `json:"title"`
	ChapterNumber float64 `json:"chapterNumber"`
	VolumeNumber  float64 `json:"volumeNumber"`
}

type SanitySpan struct {
	Type  string   `json:"_type"`
	Text  string   `json:"text"`
	Marks []string `json:"marks"`
}

type SanityMarkDef struct {
	Key  string `json:"_key"`
	Type string `json:"_type"`
	Href string `json:"href,omitempty"`
}

type SanityBlock struct {
	Type     string          `json:"_type"`
	Style    string          `json:"style"`
	Children []SanitySpan    `json:"children"`
	MarkDefs []SanityMarkDef `json:"markDefs"`
}

type SanityPageAsset struct {
	URL string `json:"url"`
}

type SanityChapterDetails struct {
	ID            string            `json:"_id"`
	Type          string            `json:"_type"`
	Title         string            `json:"title"`
	ChapterNumber float64           `json:"chapterNumber"`
	VolumeNumber  float64           `json:"volumeNumber"`
	Pages         []SanityPageAsset `json:"pages"`
	Content       []SanityBlock     `json:"content"`
}

func (c *SanityChapterDetails) ExtractTextContent() string {
	if len(c.Content) == 0 {
		return ""
	}
	var sb strings.Builder
	for _, b := range c.Content {
		if b.Type == "block" {
			var line strings.Builder
			for _, child := range b.Children {
				line.WriteString(child.Text)
			}
			text := strings.TrimSpace(line.String())
			if text != "" {
				if sb.Len() > 0 {
					sb.WriteString("\n\n")
				}
				sb.WriteString(text)
			}
		}
	}
	return sb.String()
}
