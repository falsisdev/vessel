package streaming_test

import (
	"testing"

	"github.com/falsisdev/vessel/core/internal/streaming"
)

func TestParseMagnet_Hex(t *testing.T) {
	raw := "magnet:?xt=urn:btih:c12fe1c06bba254a9dc9f519b335380dc742230b&dn=Big+Buck+Bunny&tr=udp%3A%2F%2Ftracker.opentrackr.org%3A1337"
	m, err := streaming.ParseMagnet(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if m.InfoHash != "c12fe1c06bba254a9dc9f519b335380dc742230b" {
		t.Errorf("expected lowercase hex hash, got %s", m.InfoHash)
	}
	if m.DisplayName != "Big Buck Bunny" {
		t.Errorf("expected dn 'Big Buck Bunny', got %s", m.DisplayName)
	}
	if len(m.Trackers) != 1 || m.Trackers[0] != "udp://tracker.opentrackr.org:1337" {
		t.Errorf("unexpected trackers: %+v", m.Trackers)
	}
}

func TestParseMagnet_Base32(t *testing.T) {
	// Base32 representation of hash
	// "c12fe1c06bba254a9dc9f519b335380dc742230b" in base32 is "YEX6DQDLXIRUVTOL6UM3GNI4BXBUEIYLM"
	// Let's test a known 32-char base32 string:
	// "urn:btih:4XN3MVKJ23SFEQW6W3P3K24YDFL2B6H5"
	raw := "magnet:?xt=urn:btih:4XN3MVKJ23SFEQW6W3P3K24YDFL2B6H5&dn=Test+Movie"
	m, err := streaming.ParseMagnet(raw)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}

	if len(m.InfoHash) != 40 {
		t.Errorf("expected 40-char hex, got %s (length %d)", m.InfoHash, len(m.InfoHash))
	}
	if m.DisplayName != "Test Movie" {
		t.Errorf("expected dn 'Test Movie', got %s", m.DisplayName)
	}
}

func TestParseMagnet_Invalid(t *testing.T) {
	_, err := streaming.ParseMagnet("http://example.com/video.mp4")
	if err != streaming.ErrInvalidMagnetURI {
		t.Errorf("expected ErrInvalidMagnetURI, got %v", err)
	}

	_, err = streaming.ParseMagnet("magnet:?dn=No+Hash")
	if err != streaming.ErrMissingInfoHash {
		t.Errorf("expected ErrMissingInfoHash, got %v", err)
	}
}
