package main

import (
	"flag"
	"fmt"
	"log"
	"net"
	"os"

	"github.com/falsisdev/vessel/pkg/config"
	"github.com/falsisdev/vessel/plugins/cinemasis/internal/provider"
	"github.com/falsisdev/vessel/plugins/cinemasis/internal/tmdb"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

func main() {
	_ = config.LoadEnv()

	port := flag.Int("port", 50052, "Port for Cinemasis gRPC server")
	apiKey := flag.String("tmdb-api-key", "", "TMDB API v3 Key (defaults to TMDB_API_KEY env)")
	baseURL := flag.String("tmdb-base-url", "", "TMDB Base URL (defaults to TMDB_BASE_URL env)")
	flag.Parse()

	key := *apiKey
	if key == "" {
		key = os.Getenv("TMDB_API_KEY")
	}
	if key == "" {
		key = os.Getenv("VESSEL_TMDB_API_KEY")
	}

	urlStr := *baseURL
	if urlStr == "" {
		urlStr = os.Getenv("TMDB_BASE_URL")
	}

	if key == "" {
		log.Println("[Cinemasis] Notice: TMDB_API_KEY is not provided. Set TMDB_API_KEY to execute live queries against TMDB.")
	}

	addr := fmt.Sprintf("127.0.0.1:%d", *port)
	listener, err := net.Listen("tcp", addr)
	if err != nil {
		log.Fatalf("[Cinemasis] failed to listen on %s: %v", addr, err)
	}

	var opts []tmdb.Option
	if urlStr != "" {
		opts = append(opts, tmdb.WithBaseURL(urlStr))
	}
	tmdbClient := tmdb.NewClient(key, opts...)
	service := provider.NewCinemasisService(tmdbClient)

	server := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(server, service)

	log.Printf("[Cinemasis] Cinemasis Plugin running on %s", addr)
	if err := server.Serve(listener); err != nil {
		log.Fatalf("[Cinemasis] server error: %v", err)
	}
}
