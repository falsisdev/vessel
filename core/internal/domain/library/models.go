package library

import (
	"time"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

type Status string

const (
	StatusUnspecified Status = "UNSPECIFIED"
	StatusPlanToWatch Status = "PLAN_TO_WATCH"
	StatusWatching    Status = "WATCHING"
	StatusCompleted   Status = "COMPLETED"
	StatusOnHold      Status = "ON_HOLD"
	StatusDropped     Status = "DROPPED"
	StatusFavorite    Status = "FAVORITE"
)

type Item struct {
	ID               string             `json:"id"`
	ProviderID       string             `json:"provider_id"`
	MediaID          string             `json:"media_id"`
	Domain           pluginv1.Domain    `json:"domain"`
	Title            string             `json:"title"`
	Type             pluginv1.MediaType `json:"type"`
	PosterURL        string             `json:"poster_url"`
	Status           Status             `json:"status"`
	UserRating       float32            `json:"user_rating"`
	LastInteractedAt time.Time          `json:"last_interacted_at"`
	CreatedAt        time.Time          `json:"created_at"`
	UpdatedAt        time.Time          `json:"updated_at"`
}

type PlaybackProgress struct {
	ProviderID             string          `json:"provider_id"`
	MediaID                string          `json:"media_id"`
	Domain                 pluginv1.Domain `json:"domain"`
	Title                  string          `json:"title,omitempty"`
	PosterURL              string          `json:"poster_url,omitempty"`
	SeasonNumber           int32           `json:"season_number"`
	EpisodeNumber          int32           `json:"episode_number"`
	CurrentPositionSeconds float64         `json:"current_position_seconds"`
	TotalDurationSeconds   float64         `json:"total_duration_seconds"`
	ProgressPercent        float32         `json:"progress_percent"`
	IsCompleted            bool            `json:"is_completed"`
	UpdatedAt              time.Time       `json:"updated_at"`
}

type ReadingProgress struct {
	ProviderID      string          `json:"provider_id"`
	MediaID         string          `json:"media_id"`
	Domain          pluginv1.Domain `json:"domain"`
	Title           string          `json:"title,omitempty"`
	PosterURL       string          `json:"poster_url,omitempty"`
	ChapterID       string          `json:"chapter_id"`
	ChapterNumber   float32         `json:"chapter_number"`
	CurrentPage     int32           `json:"current_page"`
	TotalPages      int32           `json:"total_pages"`
	TextScrollRatio float32         `json:"text_scroll_ratio"`
	IsCompleted     bool            `json:"is_completed"`
	UpdatedAt       time.Time       `json:"updated_at"`
}

type Filter struct {
	Domain pluginv1.Domain
	Status Status
	Limit  int
	Offset int
}
