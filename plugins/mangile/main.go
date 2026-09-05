package main

import (
	"flag"
	"fmt"
	"log"
	"net"
	"os"

	"github.com/falsisdev/vessel/pkg/config"
	"github.com/falsisdev/vessel/plugins/mangile/internal/provider"
	"github.com/falsisdev/vessel/plugins/mangile/internal/sanity"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

func main() {
	_ = config.LoadEnv()

	port := flag.Int("port", 50053, "Port for Mangile gRPC server")
	projectID := flag.String("sanity-project-id", "1yge7tlr", "Sanity Project ID (defaults to SANITY_PROJECT_ID env or 1yge7tlr)")
	dataset := flag.String("sanity-dataset", "production", "Sanity Dataset (defaults to SANITY_DATASET env or production)")
	token := flag.String("sanity-token", "", "Sanity API Token (defaults to SANITY_TOKEN env)")
	baseURL := flag.String("sanity-base-url", "", "Custom Sanity Base URL (defaults to SANITY_BASE_URL env)")
	flag.Parse()

	pID := *projectID
	if envPID := os.Getenv("SANITY_PROJECT_ID"); envPID != "" {
		pID = envPID
	}

	ds := *dataset
	if envDS := os.Getenv("SANITY_DATASET"); envDS != "" {
		ds = envDS
	}

	tok := *token
	if tok == "" {
		tok = os.Getenv("SANITY_TOKEN")
	}

	urlStr := *baseURL
	if urlStr == "" {
		urlStr = os.Getenv("SANITY_BASE_URL")
	}

	addr := fmt.Sprintf("127.0.0.1:%d", *port)
	listener, err := net.Listen("tcp", addr)
	if err != nil {
		log.Fatalf("[Mangile] failed to listen on %s: %v", addr, err)
	}

	var opts []sanity.Option
	if ds != "" {
		opts = append(opts, sanity.WithDataset(ds))
	}
	if urlStr != "" {
		opts = append(opts, sanity.WithBaseURL(urlStr))
	}

	sanityClient := sanity.NewClient(pID, tok, opts...)
	service := provider.NewMangileService(sanityClient)

	server := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(server, service)

	log.Printf("[Mangile] Mangile Plugin running on %s", listener.Addr().String())
	if err := server.Serve(listener); err != nil {
		log.Fatalf("[Mangile] server error: %v", err)
	}
}
