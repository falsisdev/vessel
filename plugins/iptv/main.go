package main

import (
	"flag"
	"fmt"
	"log"
	"net"

	"github.com/falsisdev/vessel/plugins/iptv/provider"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

func main() {
	port := flag.Int("port", 50053, "Port for IPTV gRPC service")
	flag.Parse()

	lis, err := net.Listen("tcp", fmt.Sprintf("127.0.0.1:%d", *port))
	if err != nil {
		log.Fatalf("failed to listen on port %d: %v", *port, err)
	}

	grpcServer := grpc.NewServer()
	svc := provider.NewIPTVService()
	pluginv1.RegisterPluginServiceServer(grpcServer, svc)

	log.Printf("[IPTV] IPTV Plugin running on %s", lis.Addr().String())
	if err := grpcServer.Serve(lis); err != nil {
		log.Fatalf("failed to serve: %v", err)
	}
}
