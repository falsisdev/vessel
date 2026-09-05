package debrid

import (
	"context"
	"errors"
)

var (
	ErrNotConfigured   = errors.New("debrid provider not configured")
	ErrNotCached       = errors.New("torrent is not cached on debrid provider")
	ErrNoMatchingFile  = errors.New("no matching video file found in torrent")
	ErrInvalidAPIKey   = errors.New("invalid or unauthorized debrid api key")
	ErrProviderFailure = errors.New("debrid provider api request failed")
)

// AccountStatus represents the subscription and identity status of a Debrid account.
type AccountStatus struct {
	Provider            string
	Username            string
	Email               string
	ExpirationTimestamp int64
	IsPremium           bool
	Points              int32
	Status              string
}

// StreamResult holds the resolved streaming playback target and media details.
type StreamResult struct {
	OriginalURL string
	PlaybackURL string
	StreamType  string // "debrid_cached", "direct", "magnet"
	Provider    string // "realdebrid", "torbox", "direct"
	Filename    string
	FileSize    int64
	Quality     string
	Headers     map[string]string
	IsCached    bool
}

// Provider defines the interface that all Debrid services (Real-Debrid, TorBox) implement.
type Provider interface {
	Name() string
	IsConfigured() bool
	SetAPIKey(apiKey string)
	GetAccountStatus(ctx context.Context) (*AccountStatus, error)
	CheckAvailability(ctx context.Context, hashes []string) (map[string]bool, error)
	ResolveMagnet(ctx context.Context, magnet string, season, episode int) (*StreamResult, error)
}
