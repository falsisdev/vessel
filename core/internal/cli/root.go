package cli

import (
	"fmt"
	"io"
	"os"
	"runtime"
	"strings"
)

// Execute routes command-line arguments to the appropriate handler.
func Execute(args []string) error {
	return ExecuteWithOutput(args, os.Stdout, os.Stderr)
}

// ExecuteWithOutput runs the CLI with specified output writers (useful for testing).
func ExecuteWithOutput(args []string, stdout, stderr io.Writer) error {
	if len(args) == 0 {
		return RunServer(nil)
	}

	cmd := args[0]

	// If argument starts with a flag (e.g. -plugin-bin, -ui-addr), treat as serve
	if strings.HasPrefix(cmd, "-") {
		if cmd == "-h" || cmd == "--help" {
			PrintUsage(stdout)
			return nil
		}
		if cmd == "-v" || cmd == "--version" {
			PrintVersion(stdout)
			return nil
		}
		return RunServer(args)
	}

	switch cmd {
	case "serve", "start":
		return RunServer(args[1:])

	case "plugin", "plugins":
		return handlePluginSubcommand(args[1:], stdout)

	case "theme", "themes":
		return handleThemeSubcommand(args[1:], stdout)

	case "version":
		PrintVersion(stdout)
		return nil

	case "help":
		PrintUsage(stdout)
		return nil

	default:
		return fmt.Errorf("unknown command '%s'. Run 'vessel help' for available commands", cmd)
	}
}

func handlePluginSubcommand(args []string, out io.Writer) error {
	if len(args) == 0 {
		return PluginList("", out)
	}

	sub := args[0]
	switch sub {
	case "list", "ls":
		return PluginList("", out)

	case "install", "add":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel plugin install <path-or-url>")
		}
		return PluginInstall(args[1], "", out)

	case "remove", "rm", "uninstall":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel plugin remove <plugin-id>")
		}
		return PluginRemove(args[1], "", out)

	case "enable":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel plugin enable <plugin-id>")
		}
		return PluginEnable(args[1], "", out)

	case "disable":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel plugin disable <plugin-id>")
		}
		return PluginDisable(args[1], "", out)

	default:
		return fmt.Errorf("unknown plugin command '%s'. Available: list, install, remove, enable, disable", sub)
	}
}

func handleThemeSubcommand(args []string, out io.Writer) error {
	if len(args) == 0 {
		return ThemeList("", out)
	}

	sub := args[0]
	switch sub {
	case "list", "ls":
		return ThemeList("", out)

	case "install", "add":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel theme install <path-or-url>")
		}
		return ThemeInstall(args[1], "", out)

	case "apply", "use", "set":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel theme apply <theme-id> [variant-id]")
		}
		variant := ""
		if len(args) > 2 {
			variant = args[2]
		}
		return ThemeApply(args[1], variant, "", out)

	case "remove", "rm", "uninstall":
		if len(args) < 2 {
			return fmt.Errorf("usage: vessel theme remove <theme-id>")
		}
		return ThemeRemove(args[1], "", out)

	default:
		return fmt.Errorf("unknown theme command '%s'. Available: list, install, apply, remove", sub)
	}
}

// PrintVersion outputs version and path information.
func PrintVersion(out io.Writer) {
	fmt.Fprintf(out, "Vessel Core v1.0.0 (%s/%s)\n", runtime.GOOS, runtime.GOARCH)
	fmt.Fprintf(out, "Config directory:  %s\n", DefaultConfigDir())
	fmt.Fprintf(out, "Plugins directory: %s\n", DefaultPluginsDir())
	fmt.Fprintf(out, "Themes directory:  %s\n", DefaultThemesDir())
}

// PrintUsage outputs help instructions.
func PrintUsage(out io.Writer) {
	usage := `Vessel - Local-First Digital Media & Reading Consumption Platform

USAGE:
  vessel [command] [options]

COMMANDS:
  serve                 Run Vessel runtime daemon (core IPC server and web/desktop gateway)
  plugin                Manage catalog & streaming plugins
    list                List all discovered and built-in plugins
    install <src>       Install a plugin from local directory, archive (.zip/.tar.gz), or URL
    remove <id>         Uninstall a plugin
    enable <id>         Enable a plugin
    disable <id>        Disable a plugin
  theme                 Manage UI palettes and themes
    list                List available themes and active selection
    install <src>       Install a community theme package from path or archive
    apply <id> [var]    Activate a theme and optional variant (e.g. vessel theme apply mangile mist)
    remove <id>         Remove an installed custom theme
  version               Show version and configuration paths
  help                  Show this help reference

FLAGS (for 'vessel' or 'vessel serve'):
  -ui-addr <addr>       HTTP address for UI Gateway (default: 127.0.0.1:8080)
  -listen-addr <addr>   Address for Core IPC gRPC server (default: 127.0.0.1:50050)
  -plugins-dir <dir>    Additional plugins discovery directory
  -themes-dir <dir>     Additional themes discovery directory
  -db-path <path>       SQLite database file path (default: vessel.db)
  -no-server            Disable gRPC Core IPC server
  -no-ui                Disable HTTP UI Gateway
`
	fmt.Fprint(out, usage)
}
