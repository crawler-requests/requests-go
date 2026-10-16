#!/usr/bin/env bash
# =============================================================================
# requests Rust CLI 跨平台编译脚本
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/dist"
RUST_CLI_DIR="$SCRIPT_DIR/rust/requests-cli"

declare -A TARGETS=(
    ["linux/amd64"]="requests-linux-amd64"
    ["linux/arm64"]="requests-linux-arm64"
    ["windows/amd64"]="requests-windows-amd64.exe"
    ["darwin/amd64"]="requests-darwin-amd64"
    ["darwin/arm64"]="requests-darwin-arm64"
)

mkdir -p "$OUTPUT_DIR"

echo "Building requests Rust CLI for all platforms..."
echo ""

for target in "${!TARGETS[@]}"; do
    goos="${target%/*}"
    goarch="${target#*/}"
    output="$OUTPUT_DIR/${TARGETS[$target]}"
    rust_target=""

    case "$goarch" in
        amd64) rust_target="x86_64-unknown-${goos}-gnu" ;;
        arm64) rust_target="aarch64-unknown-${goos}-gnu" ;;
        *) continue ;;
    esac

    case "$goos" in
        linux) rust_target="${rust_target/linux-gnu/linux-gnu}" ;;
        windows) rust_target="${rust_target/linux-gnu/windows-gnu}" ;;
        darwin) rust_target="${rust_target/linux-gnu/apple-darwin}" ;;
    esac

    printf "  %-20s → %s ... " "$target" "$(basename "$output")"
    if cargo build --release --manifest-path "$RUST_CLI_DIR/Cargo.toml" --target "$rust_target" -p requests-cli 2>/dev/null; then
        src="$RUST_CLI_DIR/target/$rust_target/release/requests"
        if [[ "$goos" == "windows" ]]; then
            src="${src}.exe"
        fi
        cp "$src" "$output" 2>/dev/null || true
        size=$(ls -lh "$output" 2>/dev/null | awk '{print $5}' || echo "?")
        echo "OK ($size)"
    else
        echo "SKIPPED"
    fi
done

echo ""
echo "Done! Binaries in $OUTPUT_DIR/"
ls -lh "$OUTPUT_DIR/" 2>/dev/null || echo "(no binaries built)"
