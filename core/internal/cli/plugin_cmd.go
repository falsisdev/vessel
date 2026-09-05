package cli

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"text/tabwriter"

	"github.com/falsisdev/vessel/core/internal/plugin"
)

var builtinPlugins = []struct {
	ID   string
	Name string
}{
	{ID: "com.vessel.cinema.cinemasis", Name: "Cinemasis (TMDB Cinema)"},
	{ID: "com.vessel.reading.mangile", Name: "Mangile (Sanity Manga/Novel)"},
	{ID: "com.vessel.iptv", Name: "IPTV (Global Live TV)"},
}

// PluginList lists all installed and built-in plugins.
func PluginList(pluginsDir string, out io.Writer) error {
	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	w := tabwriter.NewWriter(out, 0, 0, 3, ' ', 0)
	fmt.Fprintln(w, "ID\tNAME\tTYPE\tSTATUS\tPATH")
	fmt.Fprintln(w, "--\t----\t----\t------\t----")

	// Builtin plugins
	for _, b := range builtinPlugins {
		status := "ENABLED"
		if cfg.DisabledPlugins[b.ID] {
			status = "DISABLED"
		}
		fmt.Fprintf(w, "%s\t%s\tBUILTIN\t%s\t(embedded runtime)\n", b.ID, b.Name, status)
	}

	// External plugins from pluginsDir
	if pluginsDir != "" {
		descriptors, _ := plugin.DiscoverPlugins(pluginsDir)
		for _, d := range descriptors {
			status := "ENABLED"
			if !d.Enabled || cfg.DisabledPlugins[d.ID] {
				status = "DISABLED"
			}
			name := d.Name
			if name == "" {
				name = d.ID
			}
			fmt.Fprintf(w, "%s\t%s\tEXTERNAL\t%s\t%s\n", d.ID, name, status, d.ExecutablePath)
		}
	}

	return w.Flush()
}

// PluginInstall installs a plugin package from a directory, archive, or URL.
func PluginInstall(source, pluginsDir string, out io.Writer) error {
	if source == "" {
		return errors.New("source path or URL is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	if err := os.MkdirAll(pluginsDir, 0755); err != nil {
		return fmt.Errorf("failed to create plugins directory %s: %w", pluginsDir, err)
	}

	// Extract to temporary directory for inspection
	stagingDir, err := os.MkdirTemp("", "vessel-plugin-install-*")
	if err != nil {
		return fmt.Errorf("failed to create temp staging directory: %w", err)
	}
	defer os.RemoveAll(stagingDir)

	if err := ExtractSource(source, stagingDir); err != nil {
		return fmt.Errorf("failed to extract plugin source: %w", err)
	}

	// Locate plugin.json
	manifestPath := filepath.Join(stagingDir, "plugin.json")
	if _, err := os.Stat(manifestPath); err != nil {
		// Check subdirectories if archive was wrapped in an outer folder
		entries, _ := os.ReadDir(stagingDir)
		found := false
		for _, e := range entries {
			if e.IsDir() {
				sub := filepath.Join(stagingDir, e.Name(), "plugin.json")
				if _, err := os.Stat(sub); err == nil {
					manifestPath = sub
					stagingDir = filepath.Join(stagingDir, e.Name())
					found = true
					break
				}
			}
		}
		if !found {
			return errors.New("invalid plugin package: plugin.json manifest not found")
		}
	}

	desc, err := plugin.LoadDescriptor(manifestPath)
	if err != nil {
		return fmt.Errorf("failed to parse plugin.json: %w", err)
	}

	if desc.ID == "" {
		return errors.New("invalid plugin: id is required in plugin.json")
	}

	// Verify executable exists
	binPath := desc.ExecutablePath
	if !filepath.IsAbs(binPath) {
		binPath = filepath.Join(stagingDir, binPath)
	}
	binInfo, err := os.Stat(binPath)
	if err != nil {
		return fmt.Errorf("plugin executable not found at %s: %w", desc.ExecutablePath, err)
	}

	// Make binary executable
	_ = os.Chmod(binPath, binInfo.Mode()|0111)

	// Destination target directory: pluginsDir/<id>
	targetDir := filepath.Join(pluginsDir, desc.ID)
	_ = os.RemoveAll(targetDir)

	if err := copyDir(stagingDir, targetDir); err != nil {
		return fmt.Errorf("failed to copy plugin to %s: %w", targetDir, err)
	}

	// Re-verify target binary permissions
	targetBin := filepath.Join(targetDir, filepath.Base(desc.ExecutablePath))
	if _, err := os.Stat(targetBin); err == nil {
		_ = os.Chmod(targetBin, 0755)
	}

	// Enable plugin in config
	delete(cfg.DisabledPlugins, desc.ID)
	_ = SaveConfig(cfg)

	fmt.Fprintf(out, "✓ Successfully installed plugin '%s' (%s) into %s\n", desc.Name, desc.ID, targetDir)
	return nil
}

// PluginRemove uninstalls a non-builtin plugin.
func PluginRemove(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	for _, b := range builtinPlugins {
		if b.ID == id {
			return fmt.Errorf("cannot remove built-in plugin '%s'", id)
		}
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}

	targetDir := filepath.Join(pluginsDir, id)
	if _, err := os.Stat(targetDir); err != nil {
		return fmt.Errorf("plugin '%s' not found in %s", id, pluginsDir)
	}

	if err := os.RemoveAll(targetDir); err != nil {
		return fmt.Errorf("failed to delete plugin directory %s: %w", targetDir, err)
	}

	delete(cfg.DisabledPlugins, id)
	_ = SaveConfig(cfg)

	fmt.Fprintf(out, "✓ Successfully removed plugin '%s'\n", id)
	return nil
}

// PluginEnable enables a plugin.
func PluginEnable(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	delete(cfg.DisabledPlugins, id)
	if err := SaveConfig(cfg); err != nil {
		return err
	}

	// Also update plugin.json if it exists in pluginsDir
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}
	jsonPath := filepath.Join(pluginsDir, id, "plugin.json")
	if data, err := os.ReadFile(jsonPath); err == nil {
		var desc plugin.PluginDescriptor
		if json.Unmarshal(data, &desc) == nil {
			desc.Enabled = true
			if updated, err := json.MarshalIndent(desc, "", "  "); err == nil {
				_ = os.WriteFile(jsonPath, updated, 0644)
			}
		}
	}

	fmt.Fprintf(out, "✓ Plugin '%s' enabled\n", id)
	return nil
}

// PluginDisable disables a plugin.
func PluginDisable(id, pluginsDir string, out io.Writer) error {
	if id == "" {
		return errors.New("plugin ID is required")
	}

	cfg, err := LoadConfig()
	if err != nil {
		return err
	}

	cfg.DisabledPlugins[id] = true
	if err := SaveConfig(cfg); err != nil {
		return err
	}

	// Also update plugin.json if it exists in pluginsDir
	if pluginsDir == "" {
		pluginsDir = cfg.PluginsDir
	}
	jsonPath := filepath.Join(pluginsDir, id, "plugin.json")
	if data, err := os.ReadFile(jsonPath); err == nil {
		var desc plugin.PluginDescriptor
		if json.Unmarshal(data, &desc) == nil {
			desc.Enabled = false
			if updated, err := json.MarshalIndent(desc, "", "  "); err == nil {
				_ = os.WriteFile(jsonPath, updated, 0644)
			}
		}
	}

	fmt.Fprintf(out, "✓ Plugin '%s' disabled\n", id)
	return nil
}

// PluginInit scaffolds a new community plugin template.
func PluginInit(args []string, out io.Writer) error {
	if len(args) == 0 {
		return errors.New("usage: vessel plugin init <target-dir> [--id <id>] [--name <name>] [--domain <cinema|reading|iptv>]")
	}

	targetDir := args[0]
	pluginID := "com.community.myplugin"
	pluginName := "My Community Plugin"
	domain := "cinema"

	for i := 1; i < len(args); i++ {
		if args[i] == "--id" && i+1 < len(args) {
			pluginID = args[i+1]
			i++
		} else if args[i] == "--name" && i+1 < len(args) {
			pluginName = args[i+1]
			i++
		} else if args[i] == "--domain" && i+1 < len(args) {
			domain = args[i+1]
			i++
		}
	}

	if err := os.MkdirAll(targetDir, 0755); err != nil {
		return fmt.Errorf("failed to create directory %s: %w", targetDir, err)
	}

	// 1. Scaffold plugin.json
	manifestContent := fmt.Sprintf(`{
  "id": "%s",
  "name": "%s",
  "version": "1.0.0",
  "description": "Vessel community %s plugin extension",
  "author": "Community Developer",
  "domain": "%s",
  "protocol_version": "1.0.0",
  "executable": "./%s",
  "capabilities": ["search", "metadata", "streams"],
  "languages": ["multilingual", "en"]
}
`, pluginID, pluginName, domain, domain, filepath.Base(targetDir))

	if err := os.WriteFile(filepath.Join(targetDir, "plugin.json"), []byte(manifestContent), 0644); err != nil {
		return fmt.Errorf("failed to write plugin.json: %w", err)
	}

	// 2. Scaffold main.go
	mainContent := fmt.Sprintf(`package main

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/signal"
	"syscall"

	pluginv1 "github.com/falsisdev/vessel/proto/gen/go/plugin/v1"
	"google.golang.org/grpc"
)

type server struct {
	pluginv1.UnimplementedPluginServiceServer
}

func (s *server) GetManifest(ctx context.Context, req *pluginv1.GetManifestRequest) (*pluginv1.GetManifestResponse, error) {
	return &pluginv1.GetManifestResponse{
		Manifest: &pluginv1.PluginManifest{
			Id:              "%s",
			Name:            "%s",
			Version:         "1.0.0",
			Description:     "Vessel community %s plugin extension",
			Author:          "Community Developer",
			Domain:          pluginv1.Domain_DOMAIN_CINEMA,
			Capabilities:    []pluginv1.Capability{
				pluginv1.Capability_CAPABILITY_SEARCH,
				pluginv1.Capability_CAPABILITY_METADATA,
				pluginv1.Capability_CAPABILITY_STREAMS,
			},
			ProtocolVersion: "1.0.0",
		},
	}, nil
}

func (s *server) Search(ctx context.Context, req *pluginv1.SearchRequest) (*pluginv1.SearchResponse, error) {
	return &pluginv1.SearchResponse{
		Items: []*pluginv1.MediaItem{
			{
				Id:        "sample-item-1",
				Title:     "Sample Media Item",
				Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
				Year:      2024,
				Overview:  "Scaffolded sample media item from %s",
				PosterUrl: "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=500&q=80",
			},
		},
		HasMore: false,
	}, nil
}

func (s *server) GetMetadata(ctx context.Context, req *pluginv1.GetMetadataRequest) (*pluginv1.GetMetadataResponse, error) {
	return &pluginv1.GetMetadataResponse{
		Details: &pluginv1.MediaDetails{
			Id:        req.MediaId,
			Title:     "Sample Media Details",
			Type:      pluginv1.MediaType_MEDIA_TYPE_MOVIE,
			Year:      2024,
			Overview:  "Detailed overview for sample item",
			PosterUrl: "https://images.unsplash.com/photo-1489599849927-2ee91cede3ba?w=500&q=80",
		},
	}, nil
}

func (s *server) GetStreams(ctx context.Context, req *pluginv1.GetStreamsRequest) (*pluginv1.GetStreamsResponse, error) {
	return &pluginv1.GetStreamsResponse{
		Streams: []*pluginv1.StreamSource{
			{
				Id:      "stream-1",
				Title:   "1080p High Quality Direct Stream",
				Url:     "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4",
				Quality: "1080p",
				Format:  pluginv1.StreamFormat_STREAM_FORMAT_MP4,
			},
		},
	}, nil
}

func main() {
	lis, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		fmt.Fprintf(os.Stderr, "failed to listen: %%v\n", err)
		os.Exit(1)
	}

	// Signal Vessel Core the ephemeral port
	fmt.Printf("VESSEL_PLUGIN_PORT=%%d\n", lis.Addr().(*net.TCPAddr).Port)

	grpcServer := grpc.NewServer()
	pluginv1.RegisterPluginServiceServer(grpcServer, &server{})

	go func() {
		sigCh := make(chan os.Signal, 1)
		signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
		<-sigCh
		grpcServer.GracefulStop()
	}()

	if err := grpcServer.Serve(lis); err != nil {
		fmt.Fprintf(os.Stderr, "server error: %%v\n", err)
	}
}
`, pluginID, pluginName, domain, pluginName)

	if err := os.WriteFile(filepath.Join(targetDir, "main.go"), []byte(mainContent), 0644); err != nil {
		return fmt.Errorf("failed to write main.go: %w", err)
	}

	// 3. Scaffold README.md
	readmeContent := fmt.Sprintf(`# %s

Community plugin for the **Vessel** platform.

## Build & Test
`+"```bash"+`
# Build executable
go build -o ./%s main.go

# Validate plugin manifest and binary
vessel plugin validate .

# Install into Vessel local environment
vessel plugin install .
`+"```"+`
`, pluginName, filepath.Base(targetDir))

	_ = os.WriteFile(filepath.Join(targetDir, "README.md"), []byte(readmeContent), 0644)

	fmt.Fprintf(out, "✓ Scaffolded plugin in %s\n", targetDir)
	fmt.Fprintf(out, "  ID: %s\n", pluginID)
	fmt.Fprintf(out, "  Name: %s\n", pluginName)
	fmt.Fprintf(out, "  Domain: %s\n", domain)
	fmt.Fprintf(out, "\nTo build and validate:\n  cd %s && go build -o ./%s main.go && vessel plugin validate .\n", targetDir, filepath.Base(targetDir))
	return nil
}

// PluginValidate validates a plugin directory or manifest.
func PluginValidate(target string, out io.Writer) error {
	manifestPath := target
	stat, err := os.Stat(target)
	if err != nil {
		return fmt.Errorf("cannot access target '%s': %w", target, err)
	}

	if stat.IsDir() {
		manifestPath = filepath.Join(target, "plugin.json")
	}

	fmt.Fprintf(out, "🔍 Validating Vessel Plugin: %s\n", manifestPath)

	data, err := os.ReadFile(manifestPath)
	if err != nil {
		return fmt.Errorf("[FAIL] Could not read manifest file: %w", err)
	}

	var m struct {
		ID              string   `json:"id"`
		Name            string   `json:"name"`
		Version         string   `json:"version"`
		Domain          string   `json:"domain"`
		ProtocolVersion string   `json:"protocol_version"`
		Executable      string   `json:"executable"`
		ExecutablePath  string   `json:"executable_path"`
		Capabilities    []string `json:"capabilities"`
	}

	if err := json.Unmarshal(data, &m); err != nil {
		return fmt.Errorf("[FAIL] Invalid JSON syntax in plugin manifest: %w", err)
	}

	fmt.Fprintln(out, "  [✓] Manifest JSON syntax valid")

	if m.ID == "" {
		return errors.New("[FAIL] Missing required field: 'id'")
	}
	if m.Name == "" {
		return errors.New("[FAIL] Missing required field: 'name'")
	}
	if m.Version == "" {
		m.Version = "1.0.0"
		fmt.Fprintf(out, "  [INFO] 'version' omitted, defaulting to 1.0.0\n")
	}
	if m.ProtocolVersion == "" {
		m.ProtocolVersion = "1.0.0"
	}
	if m.Domain == "" {
		if strings.Contains(m.ID, "cinema") {
			m.Domain = "cinema"
		} else if strings.Contains(m.ID, "reading") || strings.Contains(m.ID, "manga") {
			m.Domain = "reading"
		} else if strings.Contains(m.ID, "iptv") {
			m.Domain = "iptv"
		} else {
			m.Domain = "cinema"
		}
	}

	fmt.Fprintf(out, "  [✓] Required metadata present: ID=%s, Name=%s, Domain=%s, Version=%s\n", m.ID, m.Name, m.Domain, m.Version)

	validDomains := map[string]bool{
		"cinema": true, "movies": true, "series": true,
		"reading": true, "manga": true, "webtoon": true, "webook": true,
		"live": true, "iptv": true,
	}
	if !validDomains[m.Domain] {
		fmt.Fprintf(out, "  [WARN] Unknown domain '%s'. Expected: cinema, reading, manga, iptv\n", m.Domain)
	} else {
		fmt.Fprintf(out, "  [✓] Domain '%s' recognized\n", m.Domain)
	}

	// Check executable
	exe := m.Executable
	if exe == "" {
		exe = m.ExecutablePath
	}
	if exe != "" {
		baseDir := filepath.Dir(manifestPath)
		exePath := filepath.Join(baseDir, exe)
		exeStat, err := os.Stat(exePath)
		if err != nil {
			fmt.Fprintf(out, "  [WARN] Executable '%s' not found at %s. Ensure it is compiled before running.\n", m.Executable, exePath)
		} else {
			if exeStat.Mode()&0111 == 0 {
				fmt.Fprintf(out, "  [WARN] Executable '%s' does not have execute permissions (chmod +x needed).\n", exePath)
			} else {
				fmt.Fprintf(out, "  [✓] Executable verified: %s\n", exePath)
			}
		}
	}

	fmt.Fprintln(out, "\n🎉 Plugin validation passed successfully!")
	return nil
}
