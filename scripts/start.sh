#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

mkdir -p bin

REBUILD=false
if [ "${1:-}" = "--rebuild" ]; then
    REBUILD=true
    shift
fi

if [ ! -f "./bin/vessel" ] || [ "$REBUILD" = true ]; then
    echo "Compiling Vessel executable..."
    go build -o ./bin/vessel ./core/cmd/vessel
fi

if [ ! -f "./bin/cinemasis" ] || [ "$REBUILD" = true ]; then
    echo "Compiling Cinemasis plugin..."
    go build -o ./bin/cinemasis ./plugins/cinemasis
fi

if [ ! -f "./bin/mangile" ] || [ "$REBUILD" = true ]; then
    echo "Compiling Mangile plugin..."
    go build -o ./bin/mangile ./plugins/mangile
fi

# Ensure local and user config plugin directory copies exist for seamless discovery
mkdir -p plugins/cinemasis plugins/mangile
cp -f ./bin/cinemasis ./plugins/cinemasis/cinemasis 2>/dev/null || true
cp -f ./bin/mangile ./plugins/mangile/mangile 2>/dev/null || true

USER_PLUGINS_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/vessel/plugins"
mkdir -p "$USER_PLUGINS_DIR/cinemasis" "$USER_PLUGINS_DIR/mangile"
cp -f ./plugins/cinemasis/plugin.json "$USER_PLUGINS_DIR/cinemasis/" 2>/dev/null || true
cp -f ./bin/cinemasis "$USER_PLUGINS_DIR/cinemasis/cinemasis" 2>/dev/null || true
cp -f ./plugins/mangile/plugin.json "$USER_PLUGINS_DIR/mangile/" 2>/dev/null || true
cp -f ./bin/mangile "$USER_PLUGINS_DIR/mangile/mangile" 2>/dev/null || true

exec ./bin/vessel serve "$@"
