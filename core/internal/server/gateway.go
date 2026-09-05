package server

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"log/slog"
	"net"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/domain/locale"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/streaming"
	"github.com/falsisdev/vessel/core/internal/theme"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"github.com/falsisdev/vessel/ui"
)

type GatewayServer struct {
	addr       string
	server     *http.Server
	listener   net.Listener
	cinemaSvc  *service.CinemaService
	readingSvc *service.ReadingService
	librarySvc *service.LibraryService
	streamSvc  *service.StreamService
	themeMgr   *theme.Manager
	pluginMgr  *plugin.Manager
	proxy      *streaming.Proxy
	startTime  time.Time
}

func NewGatewayServer(
	addr string,
	cinemaSvc *service.CinemaService,
	readingSvc *service.ReadingService,
	librarySvc *service.LibraryService,
	streamSvc *service.StreamService,
	pluginMgr *plugin.Manager,
	themeMgr *theme.Manager,
	proxy *streaming.Proxy,
) *GatewayServer {
	if addr == "" {
		addr = "127.0.0.1:8080"
	}

	g := &GatewayServer{
		addr:       addr,
		cinemaSvc:  cinemaSvc,
		readingSvc: readingSvc,
		librarySvc: librarySvc,
		streamSvc:  streamSvc,
		themeMgr:   themeMgr,
		pluginMgr:  pluginMgr,
		proxy:      proxy,
		startTime:  time.Now(),
	}

	mux := http.NewServeMux()
	g.registerRoutes(mux)

	g.server = &http.Server{
		Handler:      g.corsMiddleware(mux),
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 0, // Unbounded to support streaming
	}

	return g
}

func (g *GatewayServer) Start() error {
	l, err := net.Listen("tcp", g.addr)
	if err != nil {
		return fmt.Errorf("failed to bind gateway http server at %s: %w", g.addr, err)
	}
	g.listener = l
	g.addr = l.Addr().String()

	slog.Info("Vessel Native UI Gateway server listening", "url", fmt.Sprintf("http://%s", g.addr))

	go func() {
		_ = g.server.Serve(l)
	}()

	return nil
}

func (g *GatewayServer) Stop() error {
	if g.server != nil {
		ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
		defer cancel()
		return g.server.Shutdown(ctx)
	}
	return nil
}

func (g *GatewayServer) Addr() string {
	if g.listener != nil {
		return g.listener.Addr().String()
	}
	return g.addr
}

func (g *GatewayServer) URL() string {
	return fmt.Sprintf("http://%s", g.Addr())
}

func (g *GatewayServer) registerRoutes(mux *http.ServeMux) {
	// API Endpoints
	mux.HandleFunc("/api/ping", g.handlePing)
	mux.HandleFunc("/api/search", g.handleSearch)
	mux.HandleFunc("/api/media", g.handleMedia)
	mux.HandleFunc("/api/streams", g.handleStreams)
	mux.HandleFunc("/api/stream/resolve", g.handleStreamResolve)
	mux.HandleFunc("/api/chapter", g.handleChapter)
	mux.HandleFunc("/api/themes", g.handleThemes)
	mux.HandleFunc("/api/theme/active", g.handleThemeActive)
	mux.HandleFunc("/api/locales", g.handleLocales)
	mux.HandleFunc("/api/library", g.handleLibrary)
	mux.HandleFunc("/api/progress/playback", g.handlePlaybackProgress)
	mux.HandleFunc("/api/progress/playback/recent", g.handleRecentPlayback)
	mux.HandleFunc("/api/progress/reading", g.handleReadingProgress)
	mux.HandleFunc("/api/progress/reading/recent", g.handleRecentReading)
	mux.HandleFunc("/api/debrid/status", g.handleDebridStatus)
	mux.HandleFunc("/api/debrid/configure", g.handleDebridConfigure)
	mux.HandleFunc("/api/plugins", g.handlePlugins)

	// Range-enabled streaming proxy endpoint
	mux.HandleFunc("/stream", g.handleStreamProxy)

	// Embedded Static UI Assets
	subFS, err := fs.Sub(ui.DistFS, ".")
	if err == nil {
		fileServer := http.FileServer(http.FS(subFS))
		mux.Handle("/", fileServer)
	}
}

func (g *GatewayServer) corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, Range")
		w.Header().Set("Access-Control-Expose-Headers", "Content-Range, Content-Length, Accept-Ranges")

		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusOK)
			return
		}

		next.ServeHTTP(w, r)
	})
}

func (g *GatewayServer) writeJSON(w http.ResponseWriter, status int, data any) {
	w.Header().Set("Content-Type", "application/json; charset=utf-8")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(data)
}

func (g *GatewayServer) writeError(w http.ResponseWriter, status int, msg string) {
	g.writeJSON(w, status, map[string]string{"error": msg})
}

// Handlers

func (g *GatewayServer) handlePing(w http.ResponseWriter, r *http.Request) {
	uptime := int64(time.Since(g.startTime).Seconds())
	g.writeJSON(w, http.StatusOK, map[string]any{
		"status":         "ok",
		"version":        "1.0.0",
		"uptime_seconds": uptime,
	})
}

func (g *GatewayServer) handleSearch(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query().Get("query")
	domainStr := r.URL.Query().Get("domain")

	if q == "" {
		q = "popular"
	}

	ctx := r.Context()
	if domainStr == "2" || strings.EqualFold(domainStr, "reading") || strings.EqualFold(domainStr, "manga") {
		if g.readingSvc == nil {
			g.writeError(w, http.StatusServiceUnavailable, "reading service not available")
			return
		}
		items, err := g.readingSvc.Search(ctx, pluginv1.Domain_DOMAIN_MANGA, q)
		if err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]any{"items": items})
		return
	}

	// Default to cinema
	if g.cinemaSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "cinema service not available")
		return
	}
	items, err := g.cinemaSvc.Search(ctx, q)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"items": items})
}

func (g *GatewayServer) handleMedia(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	providerID := r.URL.Query().Get("provider")
	mediaID := r.URL.Query().Get("id")
	domainStr := r.URL.Query().Get("domain")

	if mediaID == "" {
		g.writeError(w, http.StatusBadRequest, "missing media id")
		return
	}

	if domainStr == "2" || strings.EqualFold(domainStr, "reading") || strings.EqualFold(domainStr, "manga") {
		if g.readingSvc == nil {
			g.writeError(w, http.StatusServiceUnavailable, "reading service not available")
			return
		}
		details, err := g.readingSvc.GetMetadata(ctx, providerID, mediaID)
		if err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, details)
		return
	}

	if g.cinemaSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "cinema service not available")
		return
	}
	details, err := g.cinemaSvc.GetMetadata(ctx, providerID, mediaID)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, details)
}

func (g *GatewayServer) handleStreams(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	providerID := r.URL.Query().Get("provider")
	mediaID := r.URL.Query().Get("media")
	season, _ := strconv.Atoi(r.URL.Query().Get("season"))
	episode, _ := strconv.Atoi(r.URL.Query().Get("episode"))

	if mediaID == "" {
		g.writeError(w, http.StatusBadRequest, "missing media id")
		return
	}

	if g.cinemaSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "cinema service not available")
		return
	}

	streams, subtitles, err := g.cinemaSvc.GetStreams(ctx, providerID, mediaID, int32(season), int32(episode))
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{
		"streams":   streams,
		"subtitles": subtitles,
	})
}

func (g *GatewayServer) handleStreamResolve(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		StreamURL         string `json:"stream_url"`
		Title             string `json:"title"`
		SeasonNumber      int    `json:"season_number"`
		EpisodeNumber     int    `json:"episode_number"`
		PreferredProvider string `json:"preferred_provider"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json payload")
		return
	}

	if g.streamSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "stream service not available")
		return
	}

	res, err := g.streamSvc.ResolveStream(r.Context(), req.StreamURL, req.Title, req.SeasonNumber, req.EpisodeNumber, req.PreferredProvider)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{"stream": res})
}

func (g *GatewayServer) handleChapter(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	providerID := r.URL.Query().Get("provider")
	mediaID := r.URL.Query().Get("media")
	chapterID := r.URL.Query().Get("chapter")
	chapterNum, _ := strconv.ParseFloat(r.URL.Query().Get("chapter_num"), 32)

	if g.readingSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "reading service not available")
		return
	}

	content, err := g.readingSvc.GetChapterContent(ctx, providerID, mediaID, chapterID, float32(chapterNum))
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, content)
}

func (g *GatewayServer) handleThemes(w http.ResponseWriter, r *http.Request) {
	if g.themeMgr == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"themes": []any{}})
		return
	}

	themeList := g.themeMgr.List()
	var themes []any
	for _, t := range themeList {
		themes = append(themes, map[string]any{
			"id":             t.Manifest.ID,
			"name":           t.Manifest.Name,
			"version":        t.Manifest.Version,
			"description":    t.Manifest.Description,
			"author":         t.Manifest.Author,
			"active_variant": t.Manifest.DefaultVariant,
			"variants":       t.Manifest.Variants,
			"is_builtin":     t.IsBuiltin,
		})
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"themes": themes})
}

func (g *GatewayServer) handleThemeActive(w http.ResponseWriter, r *http.Request) {
	if g.themeMgr == nil {
		g.writeError(w, http.StatusServiceUnavailable, "theme manager not available")
		return
	}

	if r.Method == http.MethodPost {
		var req struct {
			ThemeID   string `json:"theme_id"`
			VariantID string `json:"variant_id"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			g.writeError(w, http.StatusBadRequest, "invalid json")
			return
		}
		if _, err := g.themeMgr.SetActive(req.ThemeID, req.VariantID); err != nil {
			g.writeError(w, http.StatusBadRequest, err.Error())
			return
		}
	}

	active, err := g.themeMgr.GetActive()
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"theme": map[string]any{
			"id":   active.Theme.Manifest.ID,
			"name": active.Theme.Manifest.Name,
		},
		"active_variant": active.Variant.ID,
		"is_dark":        active.Variant.IsDark,
		"tokens":         active.Tokens,
		"compiled_css":   active.CompiledCSS,
	})
}

func (g *GatewayServer) handleLocales(w http.ResponseWriter, r *http.Request) {
	g.writeJSON(w, http.StatusOK, map[string]any{
		"locales": locale.SupportedLocales,
	})
}

func (g *GatewayServer) handleLibrary(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "library service not available")
		return
	}

	ctx := r.Context()
	switch r.Method {
	case http.MethodGet:
		status := library.Status(r.URL.Query().Get("status"))
		items, total, err := g.librarySvc.ListItems(ctx, library.Filter{Status: status})
		if err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]any{"items": items, "total": total})

	case http.MethodPost:
		var req struct {
			ProviderID string  `json:"provider_id"`
			MediaID    string  `json:"media_id"`
			Domain     int32   `json:"domain"`
			Title      string  `json:"title"`
			Type       int32   `json:"type"`
			PosterURL  string  `json:"poster_url"`
			Status     string  `json:"status"`
			UserRating float32 `json:"user_rating"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			g.writeError(w, http.StatusBadRequest, "invalid json")
			return
		}
		it := library.Item{
			ProviderID: req.ProviderID,
			MediaID:    req.MediaID,
			Domain:     pluginv1.Domain(req.Domain),
			Title:      req.Title,
			Type:       pluginv1.MediaType(req.Type),
			PosterURL:  req.PosterURL,
			Status:     library.Status(req.Status),
			UserRating: req.UserRating,
		}
		if err := g.librarySvc.SaveItem(ctx, &it); err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, it)

	case http.MethodDelete:
		provider := r.URL.Query().Get("provider")
		media := r.URL.Query().Get("media")
		if err := g.librarySvc.DeleteItem(ctx, provider, media); err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]bool{"deleted": true})

	default:
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
	}
}

func (g *GatewayServer) handlePlaybackProgress(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "library service not available")
		return
	}

	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		ProviderID             string  `json:"provider_id"`
		MediaID                string  `json:"media_id"`
		Domain                 int32   `json:"domain"`
		SeasonNumber           int32   `json:"season_number"`
		EpisodeNumber          int32   `json:"episode_number"`
		CurrentPositionSeconds float64 `json:"current_position"`
		TotalDurationSeconds   float64 `json:"total_duration"`
		ProgressPercent        float32 `json:"progress_percent"`
		IsCompleted            bool    `json:"is_completed"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json")
		return
	}

	p := library.PlaybackProgress{
		ProviderID:             req.ProviderID,
		MediaID:                req.MediaID,
		Domain:                 pluginv1.Domain(req.Domain),
		SeasonNumber:           req.SeasonNumber,
		EpisodeNumber:          req.EpisodeNumber,
		CurrentPositionSeconds: req.CurrentPositionSeconds,
		TotalDurationSeconds:   req.TotalDurationSeconds,
		ProgressPercent:        req.ProgressPercent,
		IsCompleted:            req.IsCompleted,
	}

	if err := g.librarySvc.RecordPlayback(r.Context(), &p); err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, p)
}

func (g *GatewayServer) handleRecentPlayback(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"items": []any{}})
		return
	}

	limit, _ := strconv.Atoi(r.URL.Query().Get("limit"))
	if limit <= 0 {
		limit = 10
	}

	items, err := g.librarySvc.ListRecentPlayback(r.Context(), limit)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"items": items})
}

func (g *GatewayServer) handleReadingProgress(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "library service not available")
		return
	}

	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var p library.ReadingProgress
	if err := json.NewDecoder(r.Body).Decode(&p); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json")
		return
	}

	if err := g.librarySvc.RecordReading(r.Context(), &p); err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, p)
}

func (g *GatewayServer) handleRecentReading(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"items": []any{}})
		return
	}

	limit, _ := strconv.Atoi(r.URL.Query().Get("limit"))
	if limit <= 0 {
		limit = 10
	}

	items, err := g.librarySvc.ListRecentReading(r.Context(), limit)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"items": items})
}

func (g *GatewayServer) handleDebridStatus(w http.ResponseWriter, r *http.Request) {
	if g.streamSvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"accounts": []any{}})
		return
	}

	provider := r.URL.Query().Get("provider")
	statuses, err := g.streamSvc.GetDebridStatus(r.Context(), provider)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"accounts": statuses})
}

func (g *GatewayServer) handleDebridConfigure(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		Provider string `json:"provider"`
		ApiKey   string `json:"api_key"`
		Enabled  bool   `json:"enabled"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json")
		return
	}

	if g.streamSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "stream service not available")
		return
	}

	status, err := g.streamSvc.ConfigureDebrid(r.Context(), req.Provider, req.ApiKey, req.Enabled)
	if err != nil {
		g.writeJSON(w, http.StatusOK, map[string]any{
			"success": false,
			"message": err.Error(),
		})
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"success": true,
		"status":  status,
	})
}

func (g *GatewayServer) handlePlugins(w http.ResponseWriter, r *http.Request) {
	if g.pluginMgr == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"plugins": []any{}})
		return
	}

	clients := g.pluginMgr.ListAll()
	list := make([]any, 0, len(clients))
	for _, c := range clients {
		list = append(list, c.Manifest())
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"plugins": list})
}

func (g *GatewayServer) handleStreamProxy(w http.ResponseWriter, r *http.Request) {
	rawTarget := r.URL.Query().Get("url")
	if rawTarget == "" {
		http.Error(w, "missing 'url' parameter", http.StatusBadRequest)
		return
	}

	targetURL, err := url.QueryUnescape(rawTarget)
	if err != nil {
		targetURL = rawTarget
	}

	client := &http.Client{Timeout: 60 * time.Second}
	upstreamReq, err := http.NewRequestWithContext(r.Context(), r.Method, targetURL, nil)
	if err != nil {
		http.Error(w, "invalid target url", http.StatusBadRequest)
		return
	}

	if rangeHeader := r.Header.Get("Range"); rangeHeader != "" {
		upstreamReq.Header.Set("Range", rangeHeader)
	}
	upstreamReq.Header.Set("User-Agent", "Vessel/1.0 (Gateway Proxy)")

	resp, err := client.Do(upstreamReq)
	if err != nil {
		http.Error(w, "upstream fetch failed", http.StatusBadGateway)
		return
	}
	defer resp.Body.Close()

	for _, h := range []string{"Content-Type", "Content-Length", "Content-Range", "Accept-Ranges"} {
		if v := resp.Header.Get(h); v != "" {
			w.Header().Set(h, v)
		}
	}
	if w.Header().Get("Accept-Ranges") == "" {
		w.Header().Set("Accept-Ranges", "bytes")
	}

	w.WriteHeader(resp.StatusCode)
	_, _ = io.Copy(w, resp.Body)
}
