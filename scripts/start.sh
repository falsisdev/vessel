#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$ROOT_DIR"

if [ ! -f "./bin/vessel" ] || [ "${1:-}" = "--rebuild" ]; then
    echo "Compiling Vessel executable..."
    go build -o ./bin/vessel ./core/cmd/vessel
fi

exec ./bin/vessel serve "$@"
