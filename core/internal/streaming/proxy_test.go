package streaming_test

import (
	"fmt"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/falsisdev/vessel/core/internal/streaming"
)

func TestStreamingProxy_RangeAndForwarding(t *testing.T) {
	// Mock upstream video server supporting Range requests
	content := "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
	totalLen := len(content)

	upstream := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		rangeHeader := r.Header.Get("Range")
		if rangeHeader != "" {
			// Range: bytes=10-19
			var start, end int
			_, _ = fmt.Sscanf(rangeHeader, "bytes=%d-%d", &start, &end)
			if end >= totalLen {
				end = totalLen - 1
			}
			chunk := content[start : end+1]

			w.Header().Set("Content-Type", "video/mp4")
			w.Header().Set("Content-Range", fmt.Sprintf("bytes %d-%d/%d", start, end, totalLen))
			w.Header().Set("Content-Length", fmt.Sprintf("%d", len(chunk)))
			w.Header().Set("Accept-Ranges", "bytes")
			w.WriteHeader(http.StatusPartialContent)
			_, _ = w.Write([]byte(chunk))
			return
		}

		w.Header().Set("Content-Type", "video/mp4")
		w.Header().Set("Content-Length", fmt.Sprintf("%d", totalLen))
		w.Header().Set("Accept-Ranges", "bytes")
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte(content))
	}))
	defer upstream.Close()

	proxy, err := streaming.NewProxy()
	if err != nil {
		t.Fatalf("failed to create proxy: %v", err)
	}
	defer proxy.Close()

	proxyURL := proxy.BuildProxyURL(upstream.URL)
	if !strings.Contains(proxyURL, "/stream?url=") {
		t.Fatalf("unexpected proxy url: %s", proxyURL)
	}

	// Test 1: Full content GET
	resp, err := http.Get(proxyURL)
	if err != nil {
		t.Fatalf("proxy GET failed: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		t.Errorf("expected status 200, got %d", resp.StatusCode)
	}
	body, _ := io.ReadAll(resp.Body)
	if string(body) != content {
		t.Errorf("expected full content %s, got %s", content, string(body))
	}

	// Test 2: Range request (bytes=10-19)
	req, err := http.NewRequest(http.MethodGet, proxyURL, nil)
	if err != nil {
		t.Fatalf("failed to build range request: %v", err)
	}
	req.Header.Set("Range", "bytes=10-19")

	client := &http.Client{}
	rangeResp, err := client.Do(req)
	if err != nil {
		t.Fatalf("range request failed: %v", err)
	}
	defer rangeResp.Body.Close()

	if rangeResp.StatusCode != http.StatusPartialContent {
		t.Errorf("expected status 206, got %d", rangeResp.StatusCode)
	}
	if rangeResp.Header.Get("Content-Range") != "bytes 10-19/62" {
		t.Errorf("expected Content-Range 'bytes 10-19/62', got %s", rangeResp.Header.Get("Content-Range"))
	}
	rangeBody, _ := io.ReadAll(rangeResp.Body)
	if string(rangeBody) != "ABCDEFGHIJ" {
		t.Errorf("expected 'ABCDEFGHIJ', got '%s'", string(rangeBody))
	}
}
