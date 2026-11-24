#!/usr/bin/env bash
set -euo pipefail

# Cross-compiles crates/vessel-ffi into Android jniLibs and refreshes UniFFI Kotlin bindings.
# Prerequisites:
#   1. Android Studio -> SDK Manager -> SDK Tools -> NDK (Side by side)
#   2. rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   3. cargo install cargo-ndk

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "Error: cargo-ndk is not installed. Run: cargo install cargo-ndk" >&2
  exit 1
fi

SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -d "$SDK_ROOT/ndk" ]; then
  LATEST_NDK="$(ls -1 "$SDK_ROOT/ndk" | sort -V | tail -n 1 || true)"
  if [ -n "$LATEST_NDK" ]; then
    export ANDROID_NDK_HOME="$SDK_ROOT/ndk/$LATEST_NDK"
  fi
fi

if [ -z "${ANDROID_NDK_HOME:-}" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "Error: Android NDK not found under '$SDK_ROOT/ndk'." >&2
  echo "Install 'NDK (Side by side)' via Android Studio -> SDK Manager -> SDK Tools." >&2
  exit 1
fi

echo "Using ANDROID_NDK_HOME=$ANDROID_NDK_HOME"

PROFILE="${1:-release}"
CARGO_PROFILE_FLAG="--release"
if [ "$PROFILE" = "--debug" ] || [ "$PROFILE" = "debug" ]; then
  CARGO_PROFILE_FLAG=""
fi

echo "Building libvessel_ffi.so for Android ABIs (arm64-v8a, armeabi-v7a, x86_64)..."
cargo ndk \
  -t arm64-v8a \
  -t armeabi-v7a \
  -t x86_64 \
  -o "$SCRIPT_DIR/app/src/main/jniLibs" \
  build $CARGO_PROFILE_FLAG -p vessel-ffi

echo "Regenerating UniFFI Kotlin bindings..."
cargo build -p vessel-ffi
LIB_EXT="so"
if [ "$(uname -s)" = "Darwin" ]; then
  LIB_EXT="dylib"
fi
cargo run -p vessel-ffi --bin uniffi-bindgen generate \
  --library "$REPO_ROOT/target/debug/libvessel_ffi.$LIB_EXT" \
  --language kotlin \
  --out-dir "$SCRIPT_DIR/app/src/main/java"

echo "Done! jniLibs placed under apps/android/app/src/main/jniLibs/"
