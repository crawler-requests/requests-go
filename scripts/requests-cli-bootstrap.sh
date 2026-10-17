#!/usr/bin/env bash
# requests-cli bootstrap: ensure the native CLI is present.
# This is the authoritative downloader template injected into every
# requests-<lang> wrapper's lib/requests.sh by create_all_wrappers.py.
#
# Strategy (see docs/delivery-model.md):
#   1. REQUESTS_CLI env override
#   2. local cache  ~/.cache/requests/v{VERSION}/{OS}-{ARCH}/requests-cli
#   3. GitHub Release download (pinned VERSION) + SHA256 verify
#   4. self-build fallback (offline / no prebuilt / custom libcurl)
#
# Source this file from a wrapper, or run:  bash requests-cli-bootstrap.sh ensure

set -uo pipefail

REQUESTS_ENGINE_VERSION="${REQUESTS_ENGINE_VERSION:-0.1.0}"
REQUESTS_RELEASE_BASE="https://github.com/crawler-requests/requests-go/releases/download/v${REQUESTS_ENGINE_VERSION}"
REQUESTS_CACHE_DIR="${REQUESTS_CACHE_DIR:-${HOME}/.cache/requests}"

# ---- platform detection (must match engine build-all.sh naming) ----
_requests_os() {
    case "$(uname -s)" in
        Linux)  echo linux ;;
        Darwin) echo darwin ;;
        MINGW*|MSYS*|CYGWIN*) echo windows ;;
        *)      echo linux ;;
    esac
}
_requests_arch() {
    case "$(uname -m)" in
        x86_64|amd64) echo amd64 ;;
        aarch64|arm64) echo arm64 ;;
        *) echo amd64 ;;
    esac
}

# ---- locate existing CLI: override > cache > sibling repos > cwd ----
_requests_find() {
    local name="requests-cli-$(_requests_os)-$(_requests_arch)"
    local candidates=(
        "${REQUESTS_CLI:-}"
        "${REQUESTS_CACHE_DIR}/v${REQUESTS_ENGINE_VERSION}/$(_requests_os)-$(_requests_arch)/requests-cli"
        "$(dirname "${BASH_SOURCE[0]}")/../libs/$name"
        "$(dirname "${BASH_SOURCE[0]}")/../libs_dist/$name"
        "$(dirname "${BASH_SOURCE[0]}")/$name"
        "$(dirname "${BASH_SOURCE[0]}")/../../requests/libs_dist/$name"
        "$(dirname "${BASH_SOURCE[0]}")/../../../requests/libs_dist/$name"
        "$(dirname "${BASH_SOURCE[0]}")/../../requests-go/libs_dist/$name"
    )
    for c in "${candidates[@]}"; do
        if [ -n "$c" ] && [ -x "$c" ]; then echo "$c"; return 0; fi
    done
    return 1
}

# ---- download from GitHub Release into cache ----
_requests_download() {
    local os="$(_requests_os)" arch="$(_requests_arch)"
    local name="requests-cli-${os}-${arch}"
    local dest_dir="${REQUESTS_CACHE_DIR}/v${REQUESTS_ENGINE_VERSION}/${os}-${arch}"
    local dest="${dest_dir}/requests-cli"
    mkdir -p "$dest_dir"

    local url="${REQUESTS_RELEASE_BASE}/${name}"
    echo "requests: downloading ${url}" >&2
    if command -v curl >/dev/null 2>&1; then
        # --connect-timeout / --max-time / --retry guard against slow or flaky
        # networks (observed ~450KB/s to GitHub; a hung TCP would otherwise stall).
        # --retry-all-errors also retries on transfer-level failures (56/35) that
        # --retry alone ignores (observed: curl:(56) mid-download on flaky links).
        curl -fsSL --connect-timeout 15 --max-time 300 --retry 3 --retry-all-errors -o "$dest" "$url" || return 1
    elif command -v wget >/dev/null 2>&1; then
        wget -q -T 300 -t 3 -O "$dest" "$url" || return 1
    else
        echo "requests: need curl or wget to download the CLI" >&2
        return 1
    fi
    chmod +x "$dest"

    # Also fetch libcurl-impersonate from the same Release so fingerprinting
    # (impersonate=) works out of the box. Asset name mirrors the engine repo.
    local lc="${dest_dir}/libcurl-impersonate.so.4.8.0"
    if [ ! -f "$lc" ]; then
        local lc_url="${REQUESTS_RELEASE_BASE}/libcurl-impersonate.so.4.8.0"
        echo "requests: downloading ${lc_url}" >&2
        if command -v curl >/dev/null 2>&1; then
            curl -fsSL --connect-timeout 15 --max-time 300 --retry 3 --retry-all-errors -o "$lc" "$lc_url" || true
        elif command -v wget >/dev/null 2>&1; then
            wget -q -T 300 -t 3 -O "$lc" "$lc_url" || true
        fi
        # Pointers for the core's candidate_paths (it looks for .so / .so.4 / .so.4.8.0)
        [ -f "$lc" ] && ln -sf libcurl-impersonate.so.4.8.0 "$dest_dir/libcurl-impersonate.so"
        [ -f "$lc" ] && ln -sf libcurl-impersonate.so.4.8.0 "$dest_dir/libcurl-impersonate.so.4"
    fi

    # SHA256 verification (optional: only if a SUM asset exists for the version)
    local sum_url="${REQUESTS_RELEASE_BASE}/SHA256SUMS"
    local sum_file="${dest_dir}/SHA256SUMS"
    if command -v sha256sum >/dev/null 2>&1 \
        && curl -fsSL -o "$sum_file" "$sum_url" 2>/dev/null; then
        local want got
        want=$(grep -E " ${name}\$" "$sum_file" | awk '{print $1}')
        got=$(sha256sum "$dest" | awk '{print $1}')
        if [ -n "$want" ] && [ "$want" != "$got" ]; then
            echo "requests: SHA256 mismatch for $name" >&2
            rm -f "$dest"
            return 1
        fi
    fi
    echo "$dest"
    return 0
}

# ---- self-build fallback (offline / no prebuilt platform) ----
_requests_build() {
    local root
    root="$(dirname "${BASH_SOURCE[0]}")/../../requests"
    echo "requests: no prebuilt binary; attempting self-build via ${root}/build-cli.sh" >&2
    if [ -x "$root/build-cli.sh" ]; then
        (cd "$root" && ./build-cli.sh) >&2 || return 1
        local name="requests-cli-$(_requests_os)-$(_requests_arch)"
        local built="$root/libs_dist/$name"
        [ -x "$built" ] && { echo "$built"; return 0; }
    fi
    echo "requests: self-build failed; install the Rust toolchain or set REQUESTS_CLI" >&2
    return 1
}

# ---- public: ensure a working CLI path is available ----
_requests_ensure_cli() {
    local cli
    if cli="$(_requests_find)" && [ -n "$cli" ]; then echo "$cli"; return 0; fi
    if cli="$(_requests_download)" && [ -n "$cli" ]; then echo "$cli"; return 0; fi
    if cli="$(_requests_build)" && [ -n "$cli" ]; then echo "$cli"; return 0; fi
    return 1
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then
    case "${1:-ensure}" in
        ensure) _requests_ensure_cli ;;
        version) _requests_ensure_cli && "$(_requests_ensure_cli)" version ;;
        *) echo "usage: $0 [ensure|version]" >&2; exit 2 ;;
    esac
fi
