#!/usr/bin/env bash
set -euo pipefail

export PATH="$PATH:$HOME/go/bin:/opt/homebrew/bin"

echo "Linting protobuf definitions..."
buf lint

echo "Generating code from protobuf..."
buf generate

echo "Protobuf code generation completed successfully."
