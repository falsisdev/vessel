package cli

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
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

	if err := fs.Parse(args); err != nil {
		return err
	}

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	slog.Info("Starting Vessel Core runtime")

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

	if !*noUI && !*oneshot {
		uiServer := server.NewGatewayServer(*uiAddr, cinemaService, readingService, libraryService, streamService, pluginManager, themeManager, streamingProxy)
		if err := uiServer.Start(); err != nil {
			slog.Warn("Failed to start Native UI Gateway server", "addr", *uiAddr, "error", err)
		} else {
			defer uiServer.Stop()
			slog.Info("Vessel Native UI running", "url", uiServer.URL())
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
	var searchDirs []string
	if cfg != nil && cfg.PluginsDir != "" {
		searchDirs = append(searchDirs, cfg.PluginsDir)
	}
	if *pluginsDir != "" {
		searchDirs = append(searchDirs, *pluginsDir)
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
