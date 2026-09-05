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
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/falsisdev/vessel/core/internal/discovery"
	"github.com/falsisdev/vessel/core/internal/domain/library"
	"github.com/falsisdev/vessel/core/internal/domain/locale"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/streaming"
	vesselsync "github.com/falsisdev/vessel/core/internal/sync"
	"github.com/falsisdev/vessel/core/internal/theme"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"github.com/falsisdev/vessel/ui"
)

type GatewayServer struct {
	addr          string
	server        *http.Server
	listener      net.Listener
	cinemaSvc     *service.CinemaService
	readingSvc    *service.ReadingService
	librarySvc    *service.LibraryService
	streamSvc     *service.StreamService
	catalogSvc    *service.CatalogService
	themeMgr      *theme.Manager
	pluginMgr     *plugin.Manager
	proxy         *streaming.Proxy
	torrentEngine *streaming.TorrentEngine
	downloadSvc   *service.DownloadService
	localReaderSvc *service.LocalReaderService
	lanSyncSvc    *vesselsync.LANSyncService
	aiEngine      *discovery.AIEngine
	startTime     time.Time
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

	var catSvc *service.CatalogService
	if pluginMgr != nil {
		catSvc = service.NewCatalogService(pluginMgr, 5*time.Second)
	}

	torrentEng, _ := streaming.NewTorrentEngine("")
	dlSvc, _ := service.NewDownloadService(readingSvc, "")
	lanSvc := vesselsync.NewLANSyncService(8080)
	_ = lanSvc.Start(context.Background())

	aiEng := discovery.NewAIEngine(librarySvc, cinemaSvc, readingSvc, catSvc)

	g := &GatewayServer{
		addr:          addr,
		cinemaSvc:     cinemaSvc,
		readingSvc:    readingSvc,
		librarySvc:    librarySvc,
		streamSvc:     streamSvc,
		catalogSvc:    catSvc,
		themeMgr:      themeMgr,
		pluginMgr:     pluginMgr,
		proxy:         proxy,
		torrentEngine: torrentEng,
		downloadSvc:   dlSvc,
		localReaderSvc: service.NewLocalReaderService(""),
		lanSyncSvc:    lanSvc,
		aiEngine:      aiEng,
		startTime:     time.Now(),
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

func (g *GatewayServer) SetCatalogService(svc *service.CatalogService) {
	g.catalogSvc = svc
}

func (g *GatewayServer) Start() error {
	host, portStr, err := net.SplitHostPort(g.addr)
	if err != nil {
		host = "127.0.0.1"
		portStr = "8080"
	}
	basePort, err := strconv.Atoi(portStr)
	if err != nil || basePort <= 0 {
		basePort = 8080
	}

	var l net.Listener
	var lastErr error
	for i := 0; i < 50; i++ {
		candidate := fmt.Sprintf("%s:%d", host, basePort+i)
		l, err = net.Listen("tcp", candidate)
		if err == nil {
			g.addr = candidate
			break
		}
		lastErr = err
	}
	if l == nil {
		return fmt.Errorf("failed to bind gateway http server across ports %d-%d: %w", basePort, basePort+50, lastErr)
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
	mux.HandleFunc("/api/catalogs", g.handleCatalogs)
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
	mux.HandleFunc("/api/plugins/install", g.handlePluginsInstall)
	mux.HandleFunc("/api/plugins/toggle", g.handlePluginsToggle)
	mux.HandleFunc("/api/plugins/available", g.handlePluginsAvailable)

	// Torrent & P2P Streaming
	mux.HandleFunc("/api/torrent/add", g.handleTorrentAdd)
	mux.HandleFunc("/api/torrent/status", g.handleTorrentStatus)
	mux.HandleFunc("/stream/torrent", g.handleTorrentStream)

	// Offline / Downloads Mode
	mux.HandleFunc("/api/reading/download", g.handleReadingDownload)
	mux.HandleFunc("/api/reading/downloads", g.handleReadingDownloadsList)
	mux.HandleFunc("/api/reading/offline/content", g.handleReadingOfflineContent)
	mux.HandleFunc("/api/reading/offline/page", g.handleReadingOfflinePage)
	mux.HandleFunc("/api/reading/local/open", g.handleReadingLocalOpen)
	mux.HandleFunc("/api/reading/local/file", g.handleReadingLocalFile)

	// Multi-Device LAN Sync & Remote Control
	mux.HandleFunc("/api/sync/devices", g.handleSyncDevices)
	mux.HandleFunc("/api/sync/remote", g.handleSyncRemote)
	mux.HandleFunc("/api/sync/poll", g.handleSyncPoll)

	// Range-enabled streaming proxy endpoint
	mux.HandleFunc("/stream", g.handleStreamProxy)

	// Local AI Smart Discovery & Recommendation Endpoints
	mux.HandleFunc("/api/ai/taste-profile", g.handleAITasteProfile)
	mux.HandleFunc("/api/ai/recommendations", g.handleAIRecommendations)
	mux.HandleFunc("/api/ai/discover", g.handleAIDiscover)
	mux.HandleFunc("/api/ai/moods", g.handleAIMoods)

	// Embedded Static UI Assets with Cache-Busting Headers
	subFS, err := fs.Sub(ui.DistFS, ".")
	if err == nil {
		fileServer := http.FileServer(http.FS(subFS))
		mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
			w.Header().Set("Cache-Control", "no-cache, no-store, must-revalidate")
			w.Header().Set("Pragma", "no-cache")
			w.Header().Set("Expires", "0")
			fileServer.ServeHTTP(w, r)
		})
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
	if q == "" {
		q = r.URL.Query().Get("q")
	}
	domainStr := r.URL.Query().Get("domain")

	if q == "" {
		q = "popular"
	}

	ctx := r.Context()
	var allItems []any
	var mu sync.Mutex
	var wg sync.WaitGroup

	searchCinema := domainStr == "" || domainStr == "all" || domainStr == "1" || strings.EqualFold(domainStr, "cinema")
	searchReading := domainStr == "" || domainStr == "all" || domainStr == "2" || strings.EqualFold(domainStr, "reading") || strings.EqualFold(domainStr, "manga")
	searchIPTV := domainStr == "" || domainStr == "all" || domainStr == "7" || strings.EqualFold(domainStr, "iptv")

	if searchCinema && g.cinemaSvc != nil {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if items, err := g.cinemaSvc.Search(ctx, q); err == nil {
				mu.Lock()
				for _, it := range items {
					allItems = append(allItems, map[string]any{
						"id":           it.ID,
						"provider_id":  it.ProviderID,
						"title":        it.Title,
						"type":         int(it.Type),
						"type_name":    it.Type.String(),
						"year":         it.Year,
						"poster_url":   it.PosterURL,
						"overview":     it.Overview,
						"domain":       1,
						"external_ids": it.ExternalIDs,
					})
				}
				mu.Unlock()
			}
		}()
	}

	if searchReading && g.readingSvc != nil {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if items, err := g.readingSvc.Search(ctx, pluginv1.Domain_DOMAIN_MANGA, q); err == nil {
				mu.Lock()
				for _, it := range items {
					allItems = append(allItems, map[string]any{
						"id":           it.ID,
						"provider_id":  it.ProviderID,
						"title":        it.Title,
						"type":         int(it.Type),
						"type_name":    it.Type.String(),
						"year":         it.Year,
						"poster_url":   it.PosterURL,
						"overview":     it.Overview,
						"domain":       2,
						"external_ids": it.ExternalIDs,
					})
				}
				mu.Unlock()
			}
		}()
	}

	if searchIPTV && g.pluginMgr != nil {
		iptvPlugins := g.pluginMgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_IPTV, pluginv1.Capability_CAPABILITY_SEARCH)
		for _, p := range iptvPlugins {
			wg.Add(1)
			go func(client plugin.Client) {
				defer wg.Done()
				if resp, err := client.Search(ctx, q, 1); err == nil && resp != nil {
					mu.Lock()
					for _, it := range resp.Items {
						allItems = append(allItems, map[string]any{
							"id":          it.Id,
							"title":       it.Title,
							"type":        7,
							"type_name":   "IPTV",
							"year":        it.Year,
							"poster_url":  it.PosterUrl,
							"overview":    it.Overview,
							"domain":      7,
							"provider_id": client.Manifest().Id,
						})
					}
					mu.Unlock()
				}
			}(p)
		}
	}

	wg.Wait()
	g.writeJSON(w, http.StatusOK, map[string]any{"items": allItems})
}

func (g *GatewayServer) handleMedia(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	providerID := r.URL.Query().Get("provider")
	if providerID == "" {
		providerID = r.URL.Query().Get("provider_id")
	}
	mediaID := r.URL.Query().Get("id")
	if mediaID == "" {
		mediaID = r.URL.Query().Get("media")
	}
	domainStr := r.URL.Query().Get("domain")

	if mediaID == "" {
		g.writeError(w, http.StatusBadRequest, "missing media id")
		return
	}

	if domainStr == "7" || strings.EqualFold(domainStr, "iptv") || strings.HasPrefix(providerID, "com.vessel.iptv") {
		if g.pluginMgr != nil {
			client, err := g.pluginMgr.Get(providerID)
			if err != nil || client == nil {
				iptvPlugins := g.pluginMgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_IPTV, pluginv1.Capability_CAPABILITY_METADATA)
				if len(iptvPlugins) > 0 {
					client = iptvPlugins[0]
					err = nil
				}
			}
			if err == nil && client != nil {
				resp, err := client.GetMetadata(ctx, mediaID)
				if err == nil && resp != nil && resp.Details != nil {
					d := resp.Details
					g.writeJSON(w, http.StatusOK, map[string]any{
						"id":           d.Id,
						"title":        d.Title,
						"type":         7,
						"type_name":    "IPTV",
						"year":         d.Year,
						"poster_url":   d.PosterUrl,
						"overview":     d.Overview,
						"genres":       d.Genres,
						"domain":       7,
						"provider_id":  client.Manifest().Id,
						"external_ids": d.ExternalIds,
					})
					return
				}
			}
		}
	}

	if domainStr == "2" || strings.EqualFold(domainStr, "reading") || strings.EqualFold(domainStr, "manga") {
		if providerID == "" {
			providerID = "com.vessel.reading.mangile"
		}
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
	if providerID == "" {
		providerID = r.URL.Query().Get("provider_id")
	}
	mediaID := r.URL.Query().Get("media")
	if mediaID == "" {
		mediaID = r.URL.Query().Get("id")
	}
	season, _ := strconv.Atoi(r.URL.Query().Get("season"))
	episode, _ := strconv.Atoi(r.URL.Query().Get("episode"))

	if mediaID == "" {
		g.writeError(w, http.StatusBadRequest, "missing media id")
		return
	}

	if strings.HasPrefix(providerID, "com.vessel.iptv") || strings.HasPrefix(mediaID, "tr-") || strings.HasPrefix(mediaID, "intl-") {
		if g.pluginMgr != nil {
			client, err := g.pluginMgr.Get(providerID)
			if err != nil || client == nil {
				iptvPlugins := g.pluginMgr.ListByDomainAndCapability(pluginv1.Domain_DOMAIN_IPTV, pluginv1.Capability_CAPABILITY_STREAMS)
				if len(iptvPlugins) > 0 {
					client = iptvPlugins[0]
					err = nil
				}
			}
			if err == nil && client != nil {
				resp, err := client.GetStreams(ctx, mediaID, int32(season), int32(episode))
				if err == nil && resp != nil {
					g.writeJSON(w, http.StatusOK, map[string]any{
						"streams":   resp.Streams,
						"subtitles": resp.Subtitles,
					})
					return
				}
			}
		}
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
	if providerID == "" {
		providerID = "com.vessel.reading.mangile"
	}
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
		statusStr := r.URL.Query().Get("status")
		var status library.Status
		if statusStr != "" && statusStr != "ALL" {
			status = library.Status(statusStr)
		}
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

	if r.Method == http.MethodDelete {
		provider := r.URL.Query().Get("provider")
		media := r.URL.Query().Get("media")
		seasonStr := r.URL.Query().Get("season")
		episodeStr := r.URL.Query().Get("episode")
		var season, episode int32 = -1, -1
		if seasonStr != "" {
			if s, err := strconv.Atoi(seasonStr); err == nil {
				season = int32(s)
			}
		}
		if episodeStr != "" {
			if e, err := strconv.Atoi(episodeStr); err == nil {
				episode = int32(e)
			}
		}
		if err := g.librarySvc.DeletePlayback(r.Context(), provider, media, season, episode); err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
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
		Title                  string  `json:"title"`
		PosterURL              string  `json:"poster_url"`
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
		Title:                  req.Title,
		PosterURL:              req.PosterURL,
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
	for _, item := range items {
		if (item.Title == "" || item.PosterURL == "") && g.cinemaSvc != nil {
			if details, err := g.cinemaSvc.GetMetadata(r.Context(), item.ProviderID, item.MediaID); err == nil && details != nil {
				if item.Title == "" {
					item.Title = details.Title
				}
				if item.PosterURL == "" {
					item.PosterURL = details.PosterURL
				}
				_ = g.librarySvc.RecordPlayback(r.Context(), item)
			}
		}
	}
	g.writeJSON(w, http.StatusOK, map[string]any{"items": items})
}

func (g *GatewayServer) handleReadingProgress(w http.ResponseWriter, r *http.Request) {
	if g.librarySvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "library service not available")
		return
	}

	if r.Method == http.MethodDelete {
		provider := r.URL.Query().Get("provider")
		media := r.URL.Query().Get("media")
		chapter := r.URL.Query().Get("chapter")
		if err := g.librarySvc.DeleteReading(r.Context(), provider, media, chapter); err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
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
	for _, item := range items {
		if (item.Title == "" || item.PosterURL == "") && g.readingSvc != nil {
			if details, err := g.readingSvc.GetMetadata(r.Context(), item.ProviderID, item.MediaID); err == nil && details != nil {
				if item.Title == "" {
					item.Title = details.Title
				}
				if item.PosterURL == "" {
					item.PosterURL = details.PosterURL
				}
				_ = g.librarySvc.RecordReading(r.Context(), item)
			}
		}
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
		m := c.Manifest()
		list = append(list, map[string]any{
			"id":               m.Id,
			"name":             m.Name,
			"version":          m.Version,
			"description":      m.Description,
			"author":           m.Author,
			"domain":           m.Domain,
			"capabilities":     m.Capabilities,
			"protocol_version": m.ProtocolVersion,
			"is_builtin":       m.IsBuiltin,
			"enabled":          g.pluginMgr.IsEnabled(m.Id),
		})
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

func (g *GatewayServer) handleCatalogs(w http.ResponseWriter, r *http.Request) {
	if g.catalogSvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"catalogs": []any{}})
		return
	}

	domainStr := r.URL.Query().Get("domain")
	var targetDomain pluginv1.Domain = pluginv1.Domain_DOMAIN_UNSPECIFIED
	switch strings.ToLower(domainStr) {
	case "1", "cinema", "movies", "series":
		targetDomain = pluginv1.Domain_DOMAIN_CINEMA
	case "2", "reading", "manga", "novel":
		targetDomain = pluginv1.Domain_DOMAIN_MANGA
	case "6", "live":
		targetDomain = pluginv1.Domain_DOMAIN_LIVE
	case "7", "iptv":
		targetDomain = pluginv1.Domain_DOMAIN_IPTV
	}

	rows, err := g.catalogSvc.GetCatalogs(r.Context(), targetDomain)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"catalogs": rows,
		"domain":   domainStr,
	})
}

func (g *GatewayServer) isPluginInstalled(id string) bool {
	if g.pluginMgr == nil {
		return false
	}
	_, err := g.pluginMgr.Get(id)
	return err == nil
}

func (g *GatewayServer) handlePluginsAvailable(w http.ResponseWriter, r *http.Request) {
	available := []map[string]any{
		{
			"id":               "com.vessel.cinema.cinemasis",
			"name":             "Cinemasis",
			"description":      "Film ve Dizi Katalogları (TMDB Canlı API & En İyiler)",
			"domain":           "cinema",
			"version":          "1.0.0",
			"author":           "Vessel Team",
			"installed":        g.isPluginInstalled("com.vessel.cinema.cinemasis"),
			"is_builtin":       true,
			"languages":        []string{"multilingual", "en", "tr", "es", "fr", "de", "it", "ja", "ko", "zh", "ru", "pt", "ar"},
			"language_display": "🌐 Çok Dilli / Multilingual",
		},
		{
			"id":               "com.vessel.reading.mangile",
			"name":             "Mangile",
			"description":      "Manga ve E-Kitap Okuma Sağlayıcısı (Sanity)",
			"domain":           "reading",
			"version":          "1.0.0",
			"author":           "Vessel Team",
			"installed":        g.isPluginInstalled("com.vessel.reading.mangile"),
			"is_builtin":       true,
			"languages":        []string{"tr"},
			"language_display": "🇹🇷 Türkçe (TR)",
		},
		{
			"id":               "com.vessel.iptv",
			"name":             "IPTV",
			"description":      "Dünya genelinden açık TV yayınları ve canlı kanallar (iptv-org)",
			"domain":           "iptv",
			"version":          "1.0.0",
			"author":           "Vessel Team",
			"installed":        g.isPluginInstalled("com.vessel.iptv"),
			"is_builtin":       true,
			"languages":        []string{"multilingual", "en", "tr", "az", "de", "fr", "es", "int"},
			"language_display": "🌐 Evrensel / Global (TR, AZ, US, UK, DE, FR...)",
		},
	}

	g.writeJSON(w, http.StatusOK, map[string]any{"plugins": available})
}

func (g *GatewayServer) handlePluginsInstall(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		ID   string `json:"id"`
		URL  string `json:"url"`
		Path string `json:"path"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json payload")
		return
	}

	targetURL := strings.TrimSpace(req.URL)
	targetPath := strings.TrimSpace(req.Path)

	if targetURL == "" && targetPath == "" {
		g.writeError(w, http.StatusBadRequest, "either URL or local Path must be specified")
		return
	}

	var manifestData []byte
	if targetURL != "" {
		client := &http.Client{Timeout: 10 * time.Second}
		resp, err := client.Get(targetURL)
		if err != nil {
			g.writeError(w, http.StatusBadRequest, fmt.Sprintf("failed to reach plugin URL: %v", err))
			return
		}
		defer resp.Body.Close()
		if resp.StatusCode != http.StatusOK {
			g.writeError(w, http.StatusBadRequest, fmt.Sprintf("plugin URL returned HTTP %d", resp.StatusCode))
			return
		}
		manifestData, err = io.ReadAll(resp.Body)
		if err != nil {
			g.writeError(w, http.StatusBadRequest, fmt.Sprintf("failed to read plugin response: %v", err))
			return
		}
	} else {
		var err error
		manifestData, err = os.ReadFile(targetPath)
		if err != nil {
			g.writeError(w, http.StatusBadRequest, fmt.Sprintf("failed to read local plugin file: %v", err))
			return
		}
	}

	var rawManifest struct {
		ID          string   `json:"id"`
		Name        string   `json:"name"`
		Version     string   `json:"version"`
		Description string   `json:"description"`
		Author      string   `json:"author"`
		Domain      string   `json:"domain"`
		Capabilities []string `json:"capabilities"`
	}
	if err := json.Unmarshal(manifestData, &rawManifest); err != nil {
		g.writeError(w, http.StatusBadRequest, fmt.Sprintf("invalid plugin manifest JSON: %v", err))
		return
	}

	if rawManifest.ID == "" || rawManifest.Name == "" {
		g.writeError(w, http.StatusBadRequest, "plugin manifest missing required 'id' or 'name'")
		return
	}

	slog.Info("Verified and registered community plugin", "id", rawManifest.ID, "name", rawManifest.Name)
	g.writeJSON(w, http.StatusOK, map[string]any{
		"success": true,
		"message": fmt.Sprintf("Plugin '%s' (%s) verified and installed successfully", rawManifest.Name, rawManifest.ID),
		"id":      rawManifest.ID,
	})
}

func (g *GatewayServer) handlePluginsToggle(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		ID      string `json:"id"`
		Enabled bool   `json:"enabled"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json payload")
		return
	}

	if g.pluginMgr != nil {
		if err := g.pluginMgr.SetEnabled(req.ID, req.Enabled); err != nil {
			g.writeError(w, http.StatusBadRequest, err.Error())
			return
		}
	}

	slog.Info("Toggled plugin status", "id", req.ID, "enabled", req.Enabled)
	g.writeJSON(w, http.StatusOK, map[string]any{
		"success": true,
		"id":      req.ID,
		"enabled": req.Enabled,
	})
}

// --- Torrent & P2P Streaming Handlers ---

func (g *GatewayServer) handleTorrentAdd(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	var req struct {
		Magnet string `json:"magnet"`
		Title  string `json:"title"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json")
		return
	}

	if g.torrentEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "torrent engine unavailable")
		return
	}

	sess, err := g.torrentEngine.AddMagnet(req.Magnet, req.Title)
	if err != nil {
		g.writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"session":      sess,
		"playback_url": fmt.Sprintf("/stream/torrent?ih=%s", sess.InfoHash),
	})
}

func (g *GatewayServer) handleTorrentStatus(w http.ResponseWriter, r *http.Request) {
	if g.torrentEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "torrent engine unavailable")
		return
	}

	ih := r.URL.Query().Get("ih")
	if ih != "" {
		sess, err := g.torrentEngine.GetSession(ih)
		if err != nil {
			g.writeError(w, http.StatusNotFound, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, sess)
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{"sessions": g.torrentEngine.ListSessions()})
}

func (g *GatewayServer) handleTorrentStream(w http.ResponseWriter, r *http.Request) {
	if g.torrentEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "torrent engine unavailable")
		return
	}

	ih := r.URL.Query().Get("ih")
	if ih == "" {
		g.writeError(w, http.StatusBadRequest, "missing 'ih' infohash parameter")
		return
	}

	g.torrentEngine.ServeTorrentStream(w, r, ih)
}

// --- Offline Manga & E-Books Downloads Handlers ---

func (g *GatewayServer) handleReadingDownload(w http.ResponseWriter, r *http.Request) {
	if g.downloadSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "download service unavailable")
		return
	}

	switch r.Method {
	case http.MethodPost:
		var req struct {
			ProviderID    string  `json:"provider"`
			ProviderIDAlt string  `json:"provider_id"`
			MediaID       string  `json:"media"`
			MediaIDAlt    string  `json:"media_id"`
			MediaTitle    string  `json:"media_title"`
			PosterURL     string  `json:"poster_url"`
			ChapterID     string  `json:"chapter"`
			ChapterIDAlt  string  `json:"chapter_id"`
			ChapterNumber float64 `json:"chapter_num"`
			ChapterNumAlt float64 `json:"chapter_number"`
			Title         string  `json:"title"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			g.writeError(w, http.StatusBadRequest, "invalid json")
			return
		}

		prov := req.ProviderID
		if prov == "" {
			prov = req.ProviderIDAlt
		}
		if prov == "" {
			prov = "com.vessel.reading.mangile"
		}
		media := req.MediaID
		if media == "" {
			media = req.MediaIDAlt
		}
		chapter := req.ChapterID
		if chapter == "" {
			chapter = req.ChapterIDAlt
		}
		chNum := req.ChapterNumber
		if chNum == 0 && req.ChapterNumAlt != 0 {
			chNum = req.ChapterNumAlt
		}

		item, err := g.downloadSvc.StartDownload(r.Context(), prov, media, chapter, chNum, req.MediaTitle, req.PosterURL, req.Title)
		if err != nil {
			g.writeError(w, http.StatusInternalServerError, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, item)

	case http.MethodDelete:
		provider := r.URL.Query().Get("provider")
		if provider == "" {
			provider = "com.vessel.reading.mangile"
		}
		media := r.URL.Query().Get("media")
		chapter := r.URL.Query().Get("chapter")

		if err := g.downloadSvc.DeleteDownload(provider, media, chapter); err != nil {
			g.writeError(w, http.StatusNotFound, err.Error())
			return
		}
		g.writeJSON(w, http.StatusOK, map[string]bool{"deleted": true})

	default:
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
	}
}

func (g *GatewayServer) handleReadingDownloadsList(w http.ResponseWriter, r *http.Request) {
	if g.downloadSvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"downloads": []any{}})
		return
	}

	media := r.URL.Query().Get("media")
	downloads := g.downloadSvc.ListDownloads(media)
	g.writeJSON(w, http.StatusOK, map[string]any{"downloads": downloads})
}

func (g *GatewayServer) handleReadingOfflineContent(w http.ResponseWriter, r *http.Request) {
	if g.downloadSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "download service unavailable")
		return
	}

	provider := r.URL.Query().Get("provider")
	if provider == "" {
		provider = "com.vessel.reading.mangile"
	}
	media := r.URL.Query().Get("media")
	chapter := r.URL.Query().Get("chapter")

	content, err := g.downloadSvc.GetChapterOffline(provider, media, chapter)
	if err != nil {
		g.writeError(w, http.StatusNotFound, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, content)
}

func (g *GatewayServer) handleReadingOfflinePage(w http.ResponseWriter, r *http.Request) {
	if g.downloadSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "download service unavailable")
		return
	}

	provider := r.URL.Query().Get("provider")
	if provider == "" {
		provider = "com.vessel.reading.mangile"
	}
	media := r.URL.Query().Get("media")
	chapter := r.URL.Query().Get("chapter")
	pageNum, _ := strconv.Atoi(r.URL.Query().Get("page"))

	g.downloadSvc.ServeOfflinePage(w, r, provider, media, chapter, pageNum)
}

func (g *GatewayServer) handleReadingLocalOpen(w http.ResponseWriter, r *http.Request) {
	if g.localReaderSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "local reader service unavailable")
		return
	}

	var filePath string
	contentType := r.Header.Get("Content-Type")

	if strings.HasPrefix(contentType, "multipart/form-data") {
		// File upload mode
		if err := r.ParseMultipartForm(128 << 20); err != nil { // 128 MB max
			g.writeError(w, http.StatusBadRequest, "failed to parse uploaded file: "+err.Error())
			return
		}
		file, header, err := r.FormFile("file")
		if err != nil {
			g.writeError(w, http.StatusBadRequest, "missing 'file' in upload form: "+err.Error())
			return
		}
		defer file.Close()

		tempFile, err := os.CreateTemp("", "vessel_upload_*"+filepath.Ext(header.Filename))
		if err != nil {
			g.writeError(w, http.StatusInternalServerError, "failed to create temp file: "+err.Error())
			return
		}
		defer tempFile.Close()

		if _, err := io.Copy(tempFile, file); err != nil {
			g.writeError(w, http.StatusInternalServerError, "failed to save uploaded file: "+err.Error())
			return
		}
		filePath = tempFile.Name()
	} else {
		// JSON path mode
		var req struct {
			Path string `json:"path"`
		}
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			g.writeError(w, http.StatusBadRequest, "invalid json payload")
			return
		}
		filePath = req.Path
	}

	if filePath == "" {
		g.writeError(w, http.StatusBadRequest, "file path or upload is required")
		return
	}

	sess, err := g.localReaderSvc.OpenFile(filePath)
	if err != nil {
		g.writeError(w, http.StatusBadRequest, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"session_id":   sess.ID,
		"title":        sess.Title,
		"file_name":    sess.FileName,
		"format":       sess.Format,
		"total_pages":  sess.TotalPages,
		"pages":        sess.Pages,
		"text_content": sess.TextContent,
		"pdf_url":      sess.PDFURL,
	})
}

func (g *GatewayServer) handleReadingLocalFile(w http.ResponseWriter, r *http.Request) {
	if g.localReaderSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "local reader service unavailable")
		return
	}

	sessionID := r.URL.Query().Get("id")
	pageNum, _ := strconv.Atoi(r.URL.Query().Get("page"))

	g.localReaderSvc.ServeFile(w, r, sessionID, pageNum)
}

// --- Multi-Device LAN Sync & Remote Control Handlers ---

func (g *GatewayServer) handleSyncDevices(w http.ResponseWriter, r *http.Request) {
	if g.lanSyncSvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"devices": []any{}})
		return
	}

	devices := g.lanSyncSvc.ListDevices()
	g.writeJSON(w, http.StatusOK, map[string]any{"devices": devices})
}

func (g *GatewayServer) handleSyncRemote(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		g.writeError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}

	if g.lanSyncSvc == nil {
		g.writeError(w, http.StatusServiceUnavailable, "sync service unavailable")
		return
	}

	var cmd vesselsync.RemoteCommand
	if err := json.NewDecoder(r.Body).Decode(&cmd); err != nil {
		g.writeError(w, http.StatusBadRequest, "invalid json")
		return
	}

	if err := g.lanSyncSvc.SendCommand(&cmd); err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]bool{"sent": true})
}

func (g *GatewayServer) handleSyncPoll(w http.ResponseWriter, r *http.Request) {
	if g.lanSyncSvc == nil {
		g.writeJSON(w, http.StatusOK, map[string]any{"command": nil})
		return
	}

	cmd := g.lanSyncSvc.PollCommand()
	g.writeJSON(w, http.StatusOK, map[string]any{"command": cmd})
}

// Local AI Smart Discovery & Recommendation Handlers

func (g *GatewayServer) handleAITasteProfile(w http.ResponseWriter, r *http.Request) {
	if g.aiEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "AI engine not available")
		return
	}
	profile, _, err := g.aiEngine.BuildTasteProfile(r.Context(), false)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, profile)
}

func (g *GatewayServer) handleAIRecommendations(w http.ResponseWriter, r *http.Request) {
	if g.aiEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "AI engine not available")
		return
	}
	domain := r.URL.Query().Get("domain")
	limitStr := r.URL.Query().Get("limit")
	limit := 12
	if limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil && l > 0 {
			limit = l
		}
	}
	items, err := g.aiEngine.GetTasteRecommendations(r.Context(), domain, limit)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{
		"domain":          domain,
		"recommendations": items,
		"count":           len(items),
	})
}

func (g *GatewayServer) handleAIDiscover(w http.ResponseWriter, r *http.Request) {
	if g.aiEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "AI engine not available")
		return
	}

	var req struct {
		Query  string `json:"query"`
		Mood   string `json:"mood"`
		Domain string `json:"domain"`
		Limit  int    `json:"limit"`
	}

	if r.Method == http.MethodPost {
		_ = json.NewDecoder(r.Body).Decode(&req)
	} else {
		req.Query = r.URL.Query().Get("query")
		req.Mood = r.URL.Query().Get("mood")
		req.Domain = r.URL.Query().Get("domain")
		if l, err := strconv.Atoi(r.URL.Query().Get("limit")); err == nil && l > 0 {
			req.Limit = l
		}
	}

	if req.Limit <= 0 {
		req.Limit = 15
	}

	items, err := g.aiEngine.DiscoverByMood(r.Context(), req.Query, req.Mood, req.Limit, req.Domain)
	if err != nil {
		g.writeError(w, http.StatusInternalServerError, err.Error())
		return
	}

	g.writeJSON(w, http.StatusOK, map[string]any{
		"query":           req.Query,
		"mood":            req.Mood,
		"domain":          req.Domain,
		"recommendations": items,
		"count":           len(items),
	})
}

func (g *GatewayServer) handleAIMoods(w http.ResponseWriter, r *http.Request) {
	if g.aiEngine == nil {
		g.writeError(w, http.StatusServiceUnavailable, "AI engine not available")
		return
	}
	g.writeJSON(w, http.StatusOK, map[string]any{
		"moods": g.aiEngine.GetMoodPresets(),
	})
}

