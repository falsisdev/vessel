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

	"github.com/falsisdev/vessel/core/internal/plugin"
	"github.com/falsisdev/vessel/core/internal/service"
)

func main() {
	pluginAddr := flag.String("plugin-addr", "", "Direct plugin gRPC target to connect (e.g. 127.0.0.1:50051)")
	pluginBin := flag.String("plugin-bin", "", "Path to plugin executable to launch as managed subprocess")
	pluginID := flag.String("plugin-id", "plugin-local", "Identifier for managed plugin binary")
	pluginsDir := flag.String("plugins-dir", "", "Directory containing plugins to discover")
	testQuery := flag.String("search", "", "Query to search on connected plugins")
	oneshot := flag.Bool("oneshot", false, "Exit immediately after executing operations")
	flag.Parse()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	slog.Info("Starting Vessel Core runtime")

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

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if *pluginBin != "" {
		slog.Info("Launching plugin subprocess via supervisor", "bin", *pluginBin, "id", *pluginID)
		_, err := supervisor.Launch(ctx, plugin.ProcessConfig{
			ID:             *pluginID,
			ExecutablePath: *pluginBin,
		})
		if err != nil {
			slog.Error("Failed to launch plugin subprocess", "error", err)
			os.Exit(1)
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
			slog.Error("Failed to connect to plugin", "target", *pluginAddr, "error", err)
			os.Exit(1)
		}

		manifest := client.Manifest()
		slog.Info("Connected to plugin",
			"id", manifest.Id,
			"name", manifest.Name,
			"version", manifest.Version,
			"domain", manifest.Domain.String(),
		)

		if err := pluginManager.Register(client); err != nil {
			slog.Error("Failed to register plugin in manager", "error", err)
			os.Exit(1)
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
		return
	}

	slog.Info("Vessel Core is running. Press Ctrl+C to terminate.")
	<-ctx.Done()
	slog.Info("Shutting down Vessel Core runtime...")
}
