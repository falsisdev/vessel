#!/usr/bin/env bash
set -euo pipefail

# Cross-compiles crates/vessel-ffi into Android jniLibs and refreshes UniFFI Kotlin bindings.
# Prerequisites:
#   1. Android Studio -> SDK Manager -> SDK Tools -> NDK (Side by side)
#   2. rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

SDK_ROOT="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
if [ -z "${ANDROID_NDK_HOME:-}" ] && [ -d "$SDK_ROOT/ndk" ]; then
  LATEST_NDK="$(ls -1 "$SDK_ROOT/ndk" | sort -V | tail -n 1 || true)"
  if [ -n "$LATEST_NDK" ]; then
    export ANDROID_NDK_HOME="$SDK_ROOT/ndk/$LATEST_NDK"
  fi
fi

if [ -z "${ANDROID_NDK_HOME:-}" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
  echo "Error: Android NDK not found under '$SDK_ROOT/ndk'." >&2
  exit 1
fi

HOST_TAG="darwin-x86_64"
if [ "$(uname -s)" = "Linux" ]; then
  HOST_TAG="linux-x86_64"
fi

TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG"
SYSROOT="$TOOLCHAIN/sysroot"
BIN_DIR="$TOOLCHAIN/bin"

PROFILE="${1:-release}"
CARGO_PROFILE_FLAG="--release"
TARGET_SUBDIR="release"
if [ "$PROFILE" = "--debug" ] || [ "$PROFILE" = "debug" ]; then
  CARGO_PROFILE_FLAG=""
  TARGET_SUBDIR="debug"
fi

echo "Using ANDROID_NDK_HOME=$ANDROID_NDK_HOME ($TARGET_SUBDIR)"

build_abi() {
  local ABI="$1"
  local RUST_TARGET="$2"
  local CLANG_TARGET="$3"
  local ENV_TARGET="${RUST_TARGET//-/_}"
  local UPPER_TARGET
  UPPER_TARGET="$(echo "$ENV_TARGET" | tr '[:lower:]' '[:upper:]')"

  echo "Building $ABI ($RUST_TARGET -> $CLANG_TARGET)..."
  export "CC_${ENV_TARGET}=$BIN_DIR/${CLANG_TARGET}-clang"
  export "CXX_${ENV_TARGET}=$BIN_DIR/${CLANG_TARGET}-clang++"
  export "AR_${ENV_TARGET}=$BIN_DIR/llvm-ar"
  export "CARGO_TARGET_${UPPER_TARGET}_LINKER=$BIN_DIR/${CLANG_TARGET}-clang"
  export "BINDGEN_EXTRA_CLANG_ARGS_${ENV_TARGET}=--target=${CLANG_TARGET} --sysroot=${SYSROOT} -D__ANDROID_MIN_SDK_VERSION__=26"

  cargo build $CARGO_PROFILE_FLAG --target "$RUST_TARGET" -p vessel-ffi

  local OUT_DIR="$SCRIPT_DIR/app/src/main/jniLibs/$ABI"
  mkdir -p "$OUT_DIR"
  cp "$REPO_ROOT/target/$RUST_TARGET/$TARGET_SUBDIR/libvessel_ffi.so" "$OUT_DIR/libvessel_ffi.so"
}

build_abi "arm64-v8a" "aarch64-linux-android" "aarch64-linux-android26"
build_abi "armeabi-v7a" "armv7-linux-androideabi" "armv7a-linux-androideabi26"
build_abi "x86_64" "x86_64-linux-android" "x86_64-linux-android26"

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
