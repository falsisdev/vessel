package service_test

import (
	"context"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/streaming"
)

type mockProvider struct {
	name       string
	configured bool
	status     *debrid.AccountStatus
	result     *debrid.StreamResult
}

func (m *mockProvider) Name() string                                         { return m.name }
func (m *mockProvider) IsConfigured() bool                                   { return m.configured }
func (m *mockProvider) SetAPIKey(k string)                                   { m.configured = k != "" }
func (m *mockProvider) GetAccountStatus(ctx context.Context) (*debrid.AccountStatus, error) {
	return m.status, nil
}
func (m *mockProvider) CheckAvailability(ctx context.Context, h []string) (map[string]bool, error) {
	return map[string]bool{}, nil
}
func (m *mockProvider) ResolveMagnet(ctx context.Context, mag string, s, e int) (*debrid.StreamResult, error) {
	if m.result != nil {
		return m.result, nil
	}
	return nil, debrid.ErrNotCached
}

func TestStreamService_DirectStream(t *testing.T) {
	proxy, err := streaming.NewProxy()
	if err != nil {
		t.Fatalf("failed to create proxy: %v", err)
	}
	defer proxy.Close()

	svc := service.NewStreamService(nil, proxy)
	res, err := svc.ResolveStream(context.Background(), "https://example.com/video.mp4", "Sample Video", 0, 0, "")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !strings.Contains(res.PlaybackUrl, "/stream?url=") {
		t.Errorf("expected playback URL to be proxied, got: %s", res.PlaybackUrl)
	}
	if res.StreamType != "direct" {
		t.Errorf("expected stream type 'direct', got %s", res.StreamType)
	}
}

func TestStreamService_MagnetUncached(t *testing.T) {
	svc := service.NewStreamService(nil, nil)
	mag := "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=Big+Buck+Bunny+1080p"

	res, err := svc.ResolveStream(context.Background(), mag, "", 0, 0, "")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if res.StreamType != "magnet" || res.IsCached {
		t.Errorf("expected uncached magnet stream, got: %+v", res)
	}
	if res.Quality != "1080p" {
		t.Errorf("expected quality '1080p', got %s", res.Quality)
	}
}

func TestStreamService_MagnetCachedWithDebrid(t *testing.T) {
	memStore, _ := storage.NewSQLiteStorage(":memory:")
	defer memStore.Close()

	mgr := debrid.NewManager(memStore)
	mock := &mockProvider{
		name:       "mockdebrid",
		configured: true,
		status: &debrid.AccountStatus{
			Provider:  "mockdebrid",
			Username:  "mockuser",
			IsPremium: true,
		},
		result: &debrid.StreamResult{
			OriginalURL: "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b",
			PlaybackURL: "https://fastcdn.mockdebrid.com/stream/video.mkv",
			StreamType:  "debrid_cached",
			Provider:    "mockdebrid",
			Filename:    "video.1080p.mkv",
			Quality:     "1080p",
			IsCached:    true,
		},
	}
	mgr.RegisterProvider(mock, true)

	proxy, _ := streaming.NewProxy()
	defer proxy.Close()

	svc := service.NewStreamService(mgr, proxy)
	mag := "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=video"

	res, err := svc.ResolveStream(context.Background(), mag, "Sample", 1, 1, "mockdebrid")
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if !res.IsCached || res.StreamType != "debrid_cached" {
		t.Errorf("expected cached stream, got %+v", res)
	}
	if res.Provider != "mockdebrid" {
		t.Errorf("expected provider mockdebrid, got %s", res.Provider)
	}
}
