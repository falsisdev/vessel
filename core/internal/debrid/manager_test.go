package debrid_test

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/storage"
)

func TestDebridManager_ConfigureAndStatuses(t *testing.T) {
	memStore, err := storage.NewSQLiteStorage(":memory:")
	if err != nil {
		t.Fatalf("failed to create sqlite store: %v", err)
	}
	defer memStore.Close()

	mgr := debrid.NewManager(memStore)
	ctx := context.Background()

	// Initially unconfigured
	statuses, err := mgr.GetStatuses(ctx, "")
	if err != nil {
		t.Fatalf("GetStatuses failed: %v", err)
	}
	if len(statuses) < 2 {
		t.Fatalf("expected at least 2 providers, got %d", len(statuses))
	}

	// Mock server for Real-Debrid
	rdServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{
			"id": 1, "username": "rd_user", "email": "rd@example.com",
			"points": 50, "type": "premium", "expiration": "2027-01-01T00:00:00Z"
		}`))
	}))
	defer rdServer.Close()

	rdProvider := debrid.NewRealDebridProvider("")
	rdProvider.SetBaseURL(rdServer.URL)
	mgr.RegisterProvider(rdProvider, true)

	// Configure Real-Debrid
	status, err := mgr.Configure(ctx, "realdebrid", "my_rd_key", true)
	if err != nil {
		t.Fatalf("Configure failed: %v", err)
	}
	if status.Username != "rd_user" || !status.IsPremium {
		t.Errorf("unexpected configured status: %+v", status)
	}

	// Verify persistence in store
	val, err := memStore.GetSetting(ctx, "debrid:realdebrid:token")
	if err != nil || val != "my_rd_key" {
		t.Fatalf("expected persisted token 'my_rd_key', got %s (err: %v)", val, err)
	}
}
