package cli

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"net"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"runtime"
	"strings"
	"syscall"
	"time"

	"github.com/falsisdev/vessel/core/internal/debrid"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/streaming"
	"github.com/falsisdev/vessel/core/internal/theme"
	"github.com/falsisdev/vessel/pkg/config"
)

// RunServer runs the core daemon server, background IPC, and web/desktop gateway.
func RunServer(args []string) error {
	_ = config.LoadEnv()
	cfg, _ := LoadConfig()

	fs := flag.NewFlagSet("serve", flag.ContinueOnError)
	pluginAddr := fs.String("plugin-addr", "", "Direct plugin gRPC target to connect (e.g. 127.0.0.1:50051)")
	pluginBin := fs.String("plugin-bin", "", "Path to plugin executable to launch as managed subprocess")
	pluginID := fs.String("plugin-id", "plugin-local", "Identifier for managed plugin binary")
	pluginsDir := fs.String("plugins-dir", "", "Directory containing plugins to discover")
	themesDir := fs.String("themes-dir", "", "Directory containing custom themes to discover")
	dbPath := fs.String("db-path", "vessel.db", "Path to SQLite database file")
	listenAddr := fs.String("listen-addr", "127.0.0.1:50050", "Address for Core IPC gRPC server (TCP or unix:///path)")
	noServer := fs.Bool("no-server", false, "Disable Core IPC gRPC server")
	uiAddr := fs.String("ui-addr", "127.0.0.1:8080", "Address for Native Web/Desktop UI Gateway HTTP server")
	noUI := fs.Bool("no-ui", false, "Disable Native UI Gateway HTTP server")
	testQuery := fs.String("search", "", "Query to search on connected plugins")
	oneshot := fs.Bool("oneshot", false, "Exit immediately after executing operations")

	host := fs.String("host", "", "Host to bind Web UI (e.g. 127.0.0.1 or 0.0.0.0 for LAN access)")
	port := fs.Int("port", 0, "Port to bind Web UI (e.g. 8080)")
	openBrowser := fs.Bool("open", false, "Automatically open Web UI in default browser upon start")
	headless := fs.Bool("headless", false, "Run in headless mode (never open browser)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	resolvedUIAddr := *uiAddr
	if *host != "" || *port != 0 {
		h := *host
		if h == "" {
			h = "127.0.0.1"
		}
		p := *port
		if p == 0 {
			p = 8080
		}
		resolvedUIAddr = fmt.Sprintf("%s:%d", h, p)
	}

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	sqliteStorage, err := storage.NewSQLiteStorage(*dbPath)
	if err != nil {
		return fmt.Errorf("failed to open database at %s: %w", *dbPath, err)
	}
	defer sqliteStorage.Close()

	libraryService := service.NewLibraryService(sqliteStorage)
	pluginManager := plugin.NewManager()
	supervisor := plugin.NewSupervisor(pluginManager)

	defer func() {
		if err := supervisor.Shutdown(3 * time.Second); err != nil {
			slog.Error("Failed to shutdown supervisor cleanly", "error", err)
		}
		if err := pluginManager.Close(); err != nil {
			slog.Error("Failed to close plugin manager cleanly", "error", err)
		}
	}()

	cinemaService := service.NewCinemaService(pluginManager, 5*time.Second)
	readingService := service.NewReadingService(pluginManager, 5*time.Second)

	debridManager := debrid.NewManager(sqliteStorage)
	streamingProxy, err := streaming.NewProxy()
	if err != nil {
		slog.Warn("Failed to start local streaming proxy", "error", err)
	} else {
		defer streamingProxy.Close()
	}
	streamService := service.NewStreamService(debridManager, streamingProxy)

	var customThemeDirs []string
	if cfg != nil && cfg.ThemesDir != "" {
		customThemeDirs = append(customThemeDirs, cfg.ThemesDir)
	}
	if *themesDir != "" {
		customThemeDirs = append(customThemeDirs, *themesDir)
	}

	themeManager := theme.NewManager(customThemeDirs...)
	if cfg != nil && cfg.ActiveThemeID != "" {
		_, _ = themeManager.SetActive(cfg.ActiveThemeID, cfg.ActiveVariantID)
	}

	if !*noServer && !*oneshot {
		coreServer := server.NewServer(server.ServerConfig{
			ListenAddr: *listenAddr,
			Version:    "1.0.0",
		}, cinemaService, readingService, libraryService, streamService, pluginManager, themeManager)

		if err := coreServer.Start(); err != nil {
			return fmt.Errorf("failed to start core IPC server at %s: %w", *listenAddr, err)
		}
		defer coreServer.Stop()
	}

	var uiServerURL string
	if !*noUI && !*oneshot {
		uiServer := server.NewGatewayServer(resolvedUIAddr, cinemaService, readingService, libraryService, streamService, pluginManager, themeManager, streamingProxy)
		if err := uiServer.Start(); err != nil {
			slog.Warn("Failed to start Native UI Gateway server", "addr", resolvedUIAddr, "error", err)
		} else {
			defer uiServer.Stop()
			uiServerURL = uiServer.URL()
		}
	}

	if !*oneshot {
		printBanner(uiServerURL, *listenAddr, *noServer)
		if *openBrowser && !*headless && uiServerURL != "" {
			go func() {
				time.Sleep(250 * time.Millisecond)
				openInBrowser(uiServerURL)
			}()
		}
	}

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if *pluginBin != "" {
		slog.Info("Launching plugin subprocess via supervisor", "bin", *pluginBin, "id", *pluginID)
		_, err := supervisor.Launch(ctx, plugin.ProcessConfig{
			ID:             *pluginID,
			ExecutablePath: *pluginBin,
		})
		if err != nil {
			return fmt.Errorf("failed to launch plugin subprocess %s: %w", *pluginBin, err)
		}
	}

	// Discover plugins from configured and specified directories
	var rawDirs []string
	if *pluginsDir != "" {
		rawDirs = append(rawDirs, *pluginsDir)
	}
	if cfg != nil && cfg.PluginsDir != "" {
		rawDirs = append(rawDirs, cfg.PluginsDir)
	}
	rawDirs = append(rawDirs, "plugins")

	seenDirs := make(map[string]bool)
	var searchDirs []string
	for _, d := range rawDirs {
		if d == "" {
			continue
		}
		abs, err := filepath.Abs(d)
		if err != nil {
			abs = filepath.Clean(d)
		}
		if fi, err := os.Stat(abs); err == nil && fi.IsDir() && !seenDirs[abs] {
			seenDirs[abs] = true
			searchDirs = append(searchDirs, abs)
		}
	}

	for _, pDir := range searchDirs {
		if pDir == "" {
			continue
		}
		descriptors, err := plugin.DiscoverPlugins(pDir)
		if err != nil {
			slog.Error("Failed to discover plugins", "dir", pDir, "error", err)
			continue
		}
		for _, desc := range descriptors {
			if cfg != nil && cfg.DisabledPlugins[desc.ID] {
				slog.Info("Skipping disabled plugin", "id", desc.ID)
				continue
			}
			if !desc.Enabled {
				continue
			}
			if _, err := supervisor.LaunchDescriptor(ctx, desc); err != nil {
				slog.Warn("Failed to launch discovered plugin", "id", desc.ID, "error", err)
			}
		}
	}

	if *pluginAddr != "" {
		slog.Info("Connecting to external plugin", "target", *pluginAddr)
		client, err := plugin.Dial(ctx, *pluginAddr)
		if err != nil {
			return fmt.Errorf("failed to connect to plugin at %s: %w", *pluginAddr, err)
		}

		manifest := client.Manifest()
		slog.Info("Connected to plugin",
			"id", manifest.Id,
			"name", manifest.Name,
			"version", manifest.Version,
			"domain", manifest.Domain.String(),
		)

		if err := pluginManager.Register(client); err != nil {
			return fmt.Errorf("failed to register plugin in manager: %w", err)
		}
	}

	if *testQuery != "" {
		slog.Info("Executing search across cinema plugins", "query", *testQuery)
		results, err := cinemaService.Search(ctx, *testQuery)
		if err != nil {
			slog.Error("Search failed", "error", err)
		} else {
			slog.Info("Search completed", "result_count", len(results))
			for i, item := range results {
				fmt.Printf(" [%d] %s (%d) [%s] - ID: %s (Provider: %s)\n",
					i+1, item.Title, item.Year, item.Type, item.ID, item.ProviderID)
			}
		}
	}

	if *oneshot {
		return nil
	}

	slog.Info("Vessel Core is running. Press Ctrl+C to terminate.")
	<-ctx.Done()
	slog.Info("Shutting down Vessel Core runtime...")
	return nil
}

func printBanner(uiURL, ipcAddr string, noServer bool) {
	fmt.Println()
	fmt.Println("==========================================================================")
	fmt.Println("  ⚓ VESSEL - Local-First Digital Media & Reading Platform (v1.0.0)")
	fmt.Println("==========================================================================")
	if uiURL != "" {
		fmt.Printf("  ► Local Web UI:    %s\n", uiURL)
		if strings.Contains(uiURL, "0.0.0.0") {
			parts := strings.Split(uiURL, ":")
			port := parts[len(parts)-1]
			for _, ip := range getLocalIPs() {
				fmt.Printf("  ► Network Access:  http://%s:%s (phone / tablet / LAN)\n", ip, port)
			}
		}
	}
	if !noServer {
		fmt.Printf("  ► Core IPC gRPC:   %s\n", ipcAddr)
	}
	fmt.Println("--------------------------------------------------------------------------")
	fmt.Println("  Open in your browser on any device. Press Ctrl+C to stop.")
	fmt.Println("==========================================================================")
	fmt.Println()
}

func openInBrowser(targetURL string) {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", targetURL)
	case "linux":
		cmd = exec.Command("xdg-open", targetURL)
	case "windows":
		cmd = exec.Command("rundll32", "url.dll,FileProtocolHandler", targetURL)
	}
	if cmd != nil {
		_ = cmd.Start()
	}
}

func getLocalIPs() []string {
	var ips []string
	ifaces, err := net.Interfaces()
	if err != nil {
		return nil
	}
	for _, i := range ifaces {
		addrs, err := i.Addrs()
		if err != nil {
			continue
		}
		for _, addr := range addrs {
			var ip net.IP
			switch v := addr.(type) {
			case *net.IPNet:
				ip = v.IP
			case *net.IPAddr:
				ip = v.IP
			}
			if ip != nil && !ip.IsLoopback() && ip.To4() != nil {
				ips = append(ips, ip.String())
			}
		}
	}
	return ips
}
