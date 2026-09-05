#!/usr/bin/env bash
set -euo pipefail

export PATH="$PATH:$HOME/go/bin:/opt/homebrew/bin"

echo "Running protobuf linter..."
buf lint

echo "Running Go tests..."
go test -v -race ./...

echo "All tests passed successfully."
