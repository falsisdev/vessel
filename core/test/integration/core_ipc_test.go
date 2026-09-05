package integration_test

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	coreclient "github.com/falsisdev/vessel/core/internal/client"
	"github.com/falsisdev/vessel/core/internal/plugin"
	coreserver "github.com/falsisdev/vessel/core/internal/server"
	"github.com/falsisdev/vessel/core/internal/service"
	"github.com/falsisdev/vessel/core/internal/theme"
	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
)

func buildMockBinary(t *testing.T) string {
	t.Helper()
	binPath := filepath.Join(t.TempDir(), "cinema-mock")
	cmd := exec.Command("go", "build", "-o", binPath, "../../../plugins/examples/cinema-mock")
	out, err := cmd.CombinedOutput()
	if err != nil {
		t.Fatalf("failed to build mock cinema plugin: %v, output: %s", err, string(out))
	}
	return binPath
}

func TestCoreIPCServerTCP(t *testing.T) {
	// Allocate free TCP port
	l, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		t.Fatalf("failed to listen on ephemeral port: %v", err)
	}
	port := l.Addr().(*net.TCPAddr).Port
	_ = l.Close()

	tcpAddr := fmt.Sprintf("127.0.0.1:%d", port)

	mgr := plugin.NewManager()
	supervisor := plugin.NewSupervisor(mgr)
	defer func() { _ = supervisor.Shutdown(2 * time.Second) }()

	// Launch mock cinema plugin
	mockBin := buildMockBinary(t)
	ctx, cancel := context.WithTimeout(context.Background(), 15*time.Second)
	defer cancel()

	_, err = supervisor.Launch(ctx, plugin.ProcessConfig{
		ID:             "com.vessel.cinema.mock",
		ExecutablePath: mockBin,
	})
	if err != nil {
		t.Fatalf("failed to launch mock plugin: %v", err)
	}

	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)
	readingSvc := service.NewReadingService(mgr, 3*time.Second)
	themeMgr := theme.NewManager()
	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: tcpAddr,
		Version:    "1.0.0-test",
	}, cinemaSvc, readingSvc, nil, nil, mgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start core server: %v", err)
	}
	defer srv.Stop()

	// Connect client
	client, err := coreclient.Dial(ctx, tcpAddr)
	if err != nil {
		t.Fatalf("failed to dial core server at %s: %v", tcpAddr, err)
	}
	defer func() { _ = client.Close() }()

	// 1. Ping
	pingResp, err := client.Ping(ctx)
	if err != nil {
		t.Fatalf("ping error: %v", err)
	}
	if pingResp.Status != "OK" || pingResp.Version != "1.0.0-test" {
		t.Fatalf("unexpected ping response: %+v", pingResp)
	}

	// 2. ListPlugins
	listResp, err := client.ListPlugins(ctx)
	if err != nil {
		t.Fatalf("list plugins error: %v", err)
	}
	if len(listResp.Plugins) != 1 || listResp.Plugins[0].Id != "com.vessel.cinema.mock" {
		t.Fatalf("unexpected plugins list: %+v", listResp.Plugins)
	}

	// 3. SearchMedia
	searchResp, err := client.SearchMedia(ctx, pluginv1.Domain_DOMAIN_CINEMA, "Batman", 1)
	if err != nil {
		t.Fatalf("search media error: %v", err)
	}
	if len(searchResp.Items) != 3 {
		t.Fatalf("expected 3 items, got %d", len(searchResp.Items))
	}

	// 4. GetMediaDetails
	detailsResp, err := client.GetMediaDetails(ctx, pluginv1.Domain_DOMAIN_CINEMA, "com.vessel.cinema.mock", "batman-animated")
	if err != nil {
		t.Fatalf("get details error: %v", err)
	}
	if detailsResp.Title != "Batman: The Animated Series" {
		t.Errorf("unexpected details title: %s", detailsResp.Title)
	}
	if len(detailsResp.Seasons) != 1 || len(detailsResp.Seasons[0].Episodes) != 2 {
		t.Errorf("unexpected seasons/episodes: %+v", detailsResp.Seasons)
	}

	// 5. GetStreams
	streamsResp, err := client.GetStreams(ctx, "com.vessel.cinema.mock", "batman-animated", 1, 1)
	if err != nil {
		t.Fatalf("get streams error: %v", err)
	}
	if len(streamsResp.Streams) != 2 || len(streamsResp.Subtitles) != 2 {
		t.Errorf("unexpected streams/subs count: %+v", streamsResp)
	}
}

func TestCoreIPCServerUnixSocket(t *testing.T) {
	sockPath := filepath.Join("/tmp", fmt.Sprintf("vessel-ipc-%d.sock", time.Now().UnixNano()))
	udsAddr := "unix://" + sockPath
	defer func() { _ = os.Remove(sockPath) }()

	mgr := plugin.NewManager()
	cinemaSvc := service.NewCinemaService(mgr, 3*time.Second)
	readingSvc := service.NewReadingService(mgr, 3*time.Second)
	themeMgr := theme.NewManager()
	srv := coreserver.NewServer(coreserver.ServerConfig{
		ListenAddr: udsAddr,
		Version:    "1.0.0-uds",
	}, cinemaSvc, readingSvc, nil, nil, mgr, themeMgr)

	if err := srv.Start(); err != nil {
		t.Fatalf("failed to start core UDS server: %v", err)
	}
	defer srv.Stop()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	client, err := coreclient.Dial(ctx, udsAddr)
	if err != nil {
		t.Fatalf("failed to connect over UDS: %v", err)
	}
	defer func() { _ = client.Close() }()

	pingResp, err := client.Ping(ctx)
	if err != nil {
		t.Fatalf("ping error over UDS: %v", err)
	}
	if pingResp.Status != "OK" || pingResp.Version != "1.0.0-uds" {
		t.Fatalf("unexpected ping response: %+v", pingResp)
	}
}
