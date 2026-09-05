package streaming

import (
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
)

func TestTorrentEngine_Lifecycle(t *testing.T) {
	tempDir, err := os.MkdirTemp("", "vessel-torrent-test-*")
	if err != nil {
		t.Fatalf("failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tempDir)

	engine, err := NewTorrentEngine(tempDir)
	if err != nil {
		t.Fatalf("failed to create TorrentEngine: %v", err)
	}

	magnetURI := "magnet:?xt=urn:btih:d123456789abcdef0123456789abcdef01234567&dn=Big+Buck+Bunny+1080p&tr=udp://tracker.opentrackr.org:1337"

	sess, err := engine.AddMagnet(magnetURI, "Big Buck Bunny 1080p")
	if err != nil {
		t.Fatalf("AddMagnet failed: %v", err)
	}

	if sess.InfoHash != "d123456789abcdef0123456789abcdef01234567" {
		t.Errorf("expected infohash d123456789abcdef0123456789abcdef01234567, got %s", sess.InfoHash)
	}
	if sess.Title != "Big Buck Bunny 1080p" {
		t.Errorf("expected title Big Buck Bunny 1080p, got %s", sess.Title)
	}

	// Retrieve session
	fetched, err := engine.GetSession(sess.InfoHash)
	if err != nil {
		t.Fatalf("GetSession failed: %v", err)
	}
	if fetched.InfoHash != sess.InfoHash {
		t.Errorf("fetched hash mismatch: %s vs %s", fetched.InfoHash, sess.InfoHash)
	}

	// Range request stream test
	req := httptest.NewRequest(http.MethodGet, "/stream/torrent?ih="+sess.InfoHash, nil)
	req.Header.Set("Range", "bytes=0-1024")
	rec := httptest.NewRecorder()

	engine.ServeTorrentStream(rec, req, sess.InfoHash)

	if rec.Code != http.StatusPartialContent {
		t.Errorf("expected 206 Partial Content, got %d", rec.Code)
	}
	if rec.Header().Get("Content-Range") != "bytes 0-1024/786432000" {
		t.Errorf("unexpected Content-Range header: %s", rec.Header().Get("Content-Range"))
	}

	// Stop session
	if err := engine.StopSession(sess.InfoHash); err != nil {
		t.Fatalf("StopSession failed: %v", err)
	}

	if _, err := engine.GetSession(sess.InfoHash); err == nil {
		t.Error("expected error retrieving stopped session, got nil")
	}
}
