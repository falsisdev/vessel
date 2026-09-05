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
	ID               string
	ProviderID       string
	MediaID          string
	Domain           pluginv1.Domain
	Title            string
	Type             pluginv1.MediaType
	PosterURL        string
	Status           Status
	UserRating       float32
	LastInteractedAt time.Time
	CreatedAt        time.Time
	UpdatedAt        time.Time
}

type PlaybackProgress struct {
	ProviderID             string
	MediaID                string
	Domain                 pluginv1.Domain
	SeasonNumber           int32
	EpisodeNumber          int32
	CurrentPositionSeconds float64
	TotalDurationSeconds   float64
	ProgressPercent        float32
	IsCompleted            bool
	UpdatedAt              time.Time
}

type ReadingProgress struct {
	ProviderID      string
	MediaID         string
	Domain          pluginv1.Domain
	ChapterID       string
	ChapterNumber   float32
	CurrentPage     int32
	TotalPages      int32
	TextScrollRatio float32
	IsCompleted     bool
	UpdatedAt       time.Time
}

type Filter struct {
	Domain pluginv1.Domain
	Status Status
	Limit  int
	Offset int
}
