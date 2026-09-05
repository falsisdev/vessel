package sync

import (
	"context"
	"testing"
	"time"
)

func TestLANSyncService_DiscoveryAndCommands(t *testing.T) {
	svc := NewLANSyncService(8080)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if err := svc.Start(ctx); err != nil {
		t.Fatalf("Start failed: %v", err)
	}
	defer svc.Stop()

	// Register peer device
	peer := &Device{
		ID:       "vessel-tv-livingroom",
		Name:     "Living Room TV",
		Platform: "tv",
		IP:       "192.168.1.100",
		Port:     8080,
		Version:  "1.0.0",
		LastSeen: time.Now().UTC(),
	}
	svc.RegisterPeer(peer)

	devices := svc.ListDevices()
	foundPeer := false
	for _, d := range devices {
		if d.ID == "vessel-tv-livingroom" {
			foundPeer = true
			break
		}
	}
	if !foundPeer {
		t.Errorf("expected to find living room tv peer")
	}

	// Dispatch remote command
	cmd := &RemoteCommand{
		Action:   "pause",
		MediaID:  "movie-123",
		Position: 120.5,
	}

	if err := svc.SendCommand(cmd); err != nil {
		t.Fatalf("SendCommand failed: %v", err)
	}

	received := svc.PollCommand()
	if received == nil {
		t.Fatalf("expected command, got nil")
	}
	if received.Action != "pause" {
		t.Errorf("expected action pause, got %s", received.Action)
	}
	if received.Position != 120.5 {
		t.Errorf("expected position 120.5, got %v", received.Position)
	}
}
