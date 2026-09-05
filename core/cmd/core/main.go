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
	pluginAddr := flag.String("plugin-addr", "", "Initial plugin gRPC target to connect (e.g. 127.0.0.1:50051)")
	testQuery := flag.String("search", "", "Query to search on connected plugins")
	flag.Parse()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: slog.LevelInfo,
	}))
	slog.SetDefault(logger)

	slog.Info("Starting Vessel Core runtime")

	pluginManager := plugin.NewManager()
	defer func() {
		if err := pluginManager.Close(); err != nil {
			slog.Error("Failed to close plugin manager cleanly", "error", err)
		}
	}()

	cinemaService := service.NewCinemaService(pluginManager, 5*time.Second)

	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()

	if *pluginAddr != "" {
		slog.Info("Connecting to plugin", "target", *pluginAddr)
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
	}

	slog.Info("Vessel Core is running. Press Ctrl+C to terminate.")
	<-ctx.Done()
	slog.Info("Shutting down Vessel Core runtime...")
}
