package main

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/falsisdev/vessel/pkg/config"
	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/storage"
	"github.com/falsisdev/vessel/core/internal/theme"
)

func main() {
	if err := run(); err != nil {
		slog.Error("Fatal runtime error", "error", err)
		os.Exit(1)
	}
}

func run() error {
	_ = config.LoadEnv()

	pluginAddr := flag.String("plugin-addr", "", "Direct plugin gRPC target to connect (e.g. 127.0.0.1:50051)")
	pluginBin := flag.String("plugin-bin", "", "Path to plugin executable to launch as managed subprocess")
	pluginID := flag.String("plugin-id", "plugin-local", "Identifier for managed plugin binary")
	pluginsDir := flag.String("plugins-dir", "", "Directory containing plugins to discover")
	themesDir := flag.String("themes-dir", "", "Directory containing custom themes to discover")
	dbPath := flag.String("db-path", "vessel.db", "Path to SQLite database file")
	listenAddr := flag.String("listen-addr", "127.0.0.1:50050", "Address for Core IPC gRPC server (TCP or unix:///path)")
	noServer := flag.Bool("no-server", false, "Disable Core IPC gRPC server")
	testQuery := flag.String("search", "", "Query to search on connected plugins")
	oneshot := flag.Bool("oneshot", false, "Exit immediately after executing operations")
	flag.Parse()

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

	var customThemeDirs []string
	if *themesDir != "" {
		customThemeDirs = append(customThemeDirs, *themesDir)
	}
	themeManager := theme.NewManager(customThemeDirs...)

	if !*noServer && !*oneshot {
		coreServer := server.NewServer(server.ServerConfig{
			ListenAddr: *listenAddr,
			Version:    "1.0.0",
		}, cinemaService, readingService, libraryService, pluginManager, themeManager)

		if err := coreServer.Start(); err != nil {
			return fmt.Errorf("failed to start core IPC server at %s: %w", *listenAddr, err)
		}
		defer coreServer.Stop()
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

	if *pluginsDir != "" {
		slog.Info("Discovering plugins in directory", "dir", *pluginsDir)
		descriptors, err := plugin.DiscoverPlugins(*pluginsDir)
		if err != nil {
			slog.Error("Failed to discover plugins", "error", err)
		} else {
			for _, desc := range descriptors {
				if _, err := supervisor.LaunchDescriptor(ctx, desc); err != nil {
					slog.Warn("Failed to launch discovered plugin", "id", desc.ID, "error", err)
				}
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
