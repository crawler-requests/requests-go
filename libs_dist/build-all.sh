#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CORE_DIR="$ROOT_DIR/rust/requests-core"
CLI_DIR="$ROOT_DIR/rust/requests-cli"

TARGETS=(
  "x86_64-unknown-linux-gnu|linux|amd64|requests-linux-amd64.so"
  "aarch64-unknown-linux-gnu|linux|arm64|requests-linux-arm64.so"
  "x86_64-pc-windows-gnu|windows|amd64|requests-windows-amd64.dll"
  "x86_64-apple-darwin|darwin|amd64|requests-darwin-amd64.dylib"
  "aarch64-apple-darwin|darwin|arm64|requests-darwin-arm64.dylib"
)

mkdir -p "$SCRIPT_DIR"

build_target() {
  local target="$1"
  local platform="$2"
  local arch="$3"
  local output="$4"

  echo "Build $output ..."
  cargo build --release --target "$target" --manifest-path "$CORE_DIR/Cargo.toml"

  local src="$CORE_DIR/target/$target/release/librequests_core"
  if [ ! -f "$src.so" ]; then
    src="$CORE_DIR/target/release/librequests_core"
  fi
  if [ ! -f "$src.so" ]; then
    src="$ROOT_DIR/target/$target/release/librequests_core"
  fi
  if [ ! -f "$src.so" ]; then
    src="$ROOT_DIR/target/release/librequests_core"
  fi
  local dst="$SCRIPT_DIR/$output"

  case "$platform" in
    linux)
      cp "$src.so" "$dst"
      ;;
    windows)
      cp "$src.dll" "$dst"
      ;;
    darwin)
      cp "$src.dylib" "$dst"
      ;;
  esac

  echo "Done: $dst"
}

build_cli() {
  local target="$1"
  local platform="$2"
  local arch="$3"
  local cli_output="$4"

  echo "Build CLI $cli_output ..."
  cargo build --release --target "$target" --manifest-path "$CLI_DIR/Cargo.toml"

  local src="$CLI_DIR/target/$target/release/requests"
  local dst="$SCRIPT_DIR/$cli_output"

  case "$platform" in
    linux)
      cp "$src" "$dst"
      ;;
    windows)
      cp "$src.exe" "$dst"
      ;;
    darwin)
      cp "$src" "$dst"
      ;;
  esac

  echo "Done: $dst"
}

for item in "${TARGETS[@]}"; do
  IFS='|' read -r target platform arch output <<< "$item"
  if rustup target list --installed | grep -qx "$target"; then
    build_target "$target" "$platform" "$arch" "$output"
    cli_name="requests-cli-${platform}-${arch}"
    if [[ "$platform" == "windows" ]]; then
      cli_name="${cli_name}.exe"
    fi
    build_cli "$target" "$platform" "$arch" "$cli_name"
  else
    echo "Skip $target (target not installed)"
  fi
done
