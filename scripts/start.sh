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
    echo "Compiling Vessel single-binary runtime..."
    go build -o ./bin/vessel ./core/cmd/vessel
fi

exec ./bin/vessel serve "$@"
EOF && chmod +x scripts/start.sh