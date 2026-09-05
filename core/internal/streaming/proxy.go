package streaming

import (
	"context"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"sync"
	"time"
)

// Proxy is a local HTTP progressive streaming proxy that handles
// Range requests, header forwarding, and smooth media seeking for media players.
type Proxy struct {
	listener net.Listener
	server   *http.Server
	client   *http.Client
	baseURL  string
	mu       sync.RWMutex
}

// NewProxy creates and starts a new streaming proxy on localhost with an ephemeral port.
func NewProxy() (*Proxy, error) {
	listener, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return nil, fmt.Errorf("failed to bind streaming proxy listener: %w", err)
	}

	addr := listener.Addr().String()
	p := &Proxy{
		listener: listener,
		client: &http.Client{
			Timeout: 60 * time.Second,
		},
		baseURL: fmt.Sprintf("http://%s", addr),
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/stream", p.handleStream)
	mux.HandleFunc("/ping", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		_, _ = w.Write([]byte("pong"))
	})

	p.server = &http.Server{
		Handler:      mux,
		ReadTimeout:  30 * time.Second,
		WriteTimeout: 0, // Unbounded for continuous progressive video streaming
	}

	go func() {
		_ = p.server.Serve(listener)
	}()

	return p, nil
}

// BaseURL returns the proxy's base URL (e.g., http://127.0.0.1:54321).
func (p *Proxy) BaseURL() string {
	p.mu.RLock()
	defer p.mu.RUnlock()
	return p.baseURL
}

// BuildProxyURL creates a local proxy stream URL wrapping the upstream target URL.
func (p *Proxy) BuildProxyURL(targetURL string) string {
	p.mu.RLock()
	base := p.baseURL
	p.mu.RUnlock()

	return fmt.Sprintf("%s/stream?url=%s", base, url.QueryEscape(targetURL))
}

// Close gracefully stops the proxy server.
func (p *Proxy) Close() error {
	p.mu.Lock()
	defer p.mu.Unlock()

	if p.server != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		return p.server.Shutdown(ctx)
	}
	return nil
}

func (p *Proxy) handleStream(w http.ResponseWriter, r *http.Request) {
	rawTarget := r.URL.Query().Get("url")
	if rawTarget == "" {
		http.Error(w, "missing 'url' query parameter", http.StatusBadRequest)
		return
	}

	targetURL, err := url.QueryUnescape(rawTarget)
	if err != nil {
		targetURL = rawTarget
	}

	upstreamReq, err := http.NewRequestWithContext(r.Context(), r.Method, targetURL, nil)
	if err != nil {
		http.Error(w, fmt.Sprintf("invalid target url: %v", err), http.StatusBadRequest)
		return
	}

	// Forward Range header if present
	if rangeHeader := r.Header.Get("Range"); rangeHeader != "" {
		upstreamReq.Header.Set("Range", rangeHeader)
	}
	upstreamReq.Header.Set("User-Agent", "Vessel/1.0 (Core Streaming Engine)")

	upstreamResp, err := p.client.Do(upstreamReq)
	if err != nil {
		http.Error(w, fmt.Sprintf("failed to fetch upstream stream: %v", err), http.StatusBadGateway)
		return
	}
	defer upstreamResp.Body.Close()

	// Forward relevant streaming headers
	for _, headerName := range []string{"Content-Type", "Content-Length", "Content-Range", "Accept-Ranges"} {
		if val := upstreamResp.Header.Get(headerName); val != "" {
			w.Header().Set(headerName, val)
		}
	}

	// Ensure Accept-Ranges is advertised to video players
	if w.Header().Get("Accept-Ranges") == "" {
		w.Header().Set("Accept-Ranges", "bytes")
	}

	w.WriteHeader(upstreamResp.StatusCode)

	// Stream payload directly to client
	_, _ = io.Copy(w, upstreamResp.Body)
}
