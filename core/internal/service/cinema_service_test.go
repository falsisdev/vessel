package service_test

import (
	"context"
	"testing"
	"time"

	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
)

func TestCinemaServiceEmptyManager(t *testing.T) {
	mgr := plugin.NewManager()
	svc := service.NewCinemaService(mgr, 1*time.Second)

	results, err := svc.Search(context.Background(), "Batman")
	if err != nil {
		t.Fatalf("expected nil error on empty manager, got: %v", err)
	}
	if len(results) != 0 {
		t.Fatalf("expected 0 results, got %d", len(results))
	}

	_, err = svc.GetMetadata(context.Background(), "unknown-provider", "id-123")
	if err == nil {
		t.Fatal("expected error when provider not found")
	}

	_, _, err = svc.GetStreams(context.Background(), "unknown-provider", "id-123", 0, 0)
	if err == nil {
		t.Fatal("expected error when provider not found")
	}
}
