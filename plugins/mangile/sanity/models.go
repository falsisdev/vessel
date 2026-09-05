package sanity

import (
	"fmt"
	"strings"
	"time"
)

type SanityTitle struct {
	ID               string                 `json:"_id"`
	Type             string                 `json:"_type"`
	CreatedAt        time.Time              `json:"_createdAt"`
	UpdatedAt        time.Time              `json:"_updatedAt"`
	Title            string                 `json:"title"`
	Slug             string                 `json:"slug"`
	Description      string                 `json:"description"`
	MyAnimeListID    int                    `json:"myAnimeListId"`
	UploadStatus     string                 `json:"uploadStatus"`
	Tags             []string               `json:"tags"`
	Format           string                 `json:"format"`
	CoverImage       string                 `json:"coverImage"`
	BannerImage      string                 `json:"bannerImage"`
	Chapters         []SanityChapterSummary `json:"chapters"`
	EmbeddedChapters []SanityChapterSummary `json:"embeddedChapters,omitempty"`
	ExternalChapters []SanityChapterSummary `json:"externalChapters,omitempty"`
}

type SanityChapterSummary struct {
	ID            string  `json:"_id"`
	Key           string  `json:"_key"`
	Title         string  `json:"title"`
	ChapterNumber float64 `json:"chapterNumber"`
	VolumeNumber  float64 `json:"volumeNumber"`
}

func (s SanityChapterSummary) GetID() string {
	if s.ID != "" {
		return s.ID
	}
	return s.Key
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

type SanityBlockAsset struct {
	Ref string `json:"_ref"`
	URL string `json:"url"`
}

type SanityBlock struct {
	Type     string            `json:"_type"`
	Style    string            `json:"style"`
	Children []SanitySpan      `json:"children"`
	MarkDefs []SanityMarkDef   `json:"markDefs"`
	Asset    *SanityBlockAsset `json:"asset,omitempty"`
	Alt      string            `json:"alt,omitempty"`
	Caption  string            `json:"caption,omitempty"`
}

type SanityPageAsset struct {
	URL string `json:"url"`
	Ref string `json:"ref,omitempty"`
}

func (p SanityPageAsset) ResolvedURL(projectID, dataset string) string {
	if p.URL != "" {
		return p.URL
	}
	if p.Ref == "" {
		return ""
	}
	parts := strings.Split(p.Ref, "-")
	if len(parts) >= 4 {
		if dataset == "" {
			dataset = "production"
		}
		if projectID == "" {
			projectID = "1yge7tlr"
		}
		return fmt.Sprintf("https://cdn.sanity.io/images/%s/%s/%s-%s.%s", projectID, dataset, parts[1], parts[2], parts[3])
	}
	return ""
}

type SanityChapterDetails struct {
	ID            string            `json:"_id"`
	Key           string            `json:"_key"`
	Type          string            `json:"_type"`
	Title         string            `json:"title"`
	ChapterNumber float64           `json:"chapterNumber"`
	VolumeNumber  float64           `json:"volumeNumber"`
	Pages         []SanityPageAsset `json:"pages"`
	Content       []SanityBlock     `json:"content"`
}

func (c *SanityChapterDetails) GetID() string {
	if c.ID != "" {
		return c.ID
	}
	return c.Key
}

func (c *SanityChapterDetails) ExtractTextContent() string {
	if len(c.Content) == 0 {
		return ""
	}
	var sb strings.Builder
	for _, b := range c.Content {
		if b.Type == "block" {
			// Build map of link markDefs
			linkMap := make(map[string]string)
			for _, md := range b.MarkDefs {
				if md.Type == "link" && md.Href != "" {
					linkMap[md.Key] = md.Href
				}
			}

			var line strings.Builder
			for _, child := range b.Children {
				hasImageLink := false
				for _, mKey := range child.Marks {
					if href, ok := linkMap[mKey]; ok {
						lowHref := strings.ToLower(href)
						if strings.Contains(lowHref, "cdn.sanity.io/images") ||
							strings.HasSuffix(lowHref, ".jpg") || strings.HasSuffix(lowHref, ".jpeg") ||
							strings.HasSuffix(lowHref, ".png") || strings.HasSuffix(lowHref, ".webp") ||
							strings.HasSuffix(lowHref, ".gif") || strings.HasSuffix(lowHref, ".svg") {
							alt := child.Text
							if alt == "" {
								alt = "Görsel"
							}
							if sb.Len() > 0 {
								sb.WriteString("\n\n")
							}
							sb.WriteString(fmt.Sprintf("![%s](%s)", alt, href))
							hasImageLink = true
							break
						}
					}
				}
				if !hasImageLink {
					line.WriteString(child.Text)
				}
			}
			text := strings.TrimSpace(line.String())
			if text != "" {
				if sb.Len() > 0 {
					sb.WriteString("\n\n")
				}
				sb.WriteString(text)
			}
		} else if b.Type == "image" && b.Asset != nil {
			imgURL := b.Asset.URL
			if imgURL == "" && b.Asset.Ref != "" {
				parts := strings.Split(b.Asset.Ref, "-")
				if len(parts) >= 4 {
					imgURL = fmt.Sprintf("https://cdn.sanity.io/images/1yge7tlr/production/%s-%s.%s", parts[1], parts[2], parts[3])
				}
			}
			if imgURL != "" {
				if sb.Len() > 0 {
					sb.WriteString("\n\n")
				}
				alt := b.Caption
				if alt == "" {
					alt = b.Alt
				}
				if alt == "" {
					alt = "Görsel"
				}
				sb.WriteString(fmt.Sprintf("![%s](%s)", alt, imgURL))
			}
		}
	}
	return sb.String()
}
