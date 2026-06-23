#!/usr/bin/env bash
# Build the barriers helper that FlowDesk embeds/runs from the Tauri app.
#
# Copyright (C) 2026 helloxkk (FlowDesk)
# Licensed under GPLv2.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"

HOST_OS="$(uname -s)"
HOST_ARCH="$(uname -m)"

if [[ -n "${FLOWDESK_BARRIERS_ARCH:-}" ]]; then
    BARRIER_ARCH="$FLOWDESK_BARRIERS_ARCH"
elif [[ "$HOST_OS" == "Darwin" && "$HOST_ARCH" == "arm64" ]]; then
    # Old Barrier.app runs its server helper as x86_64 under Rosetta, and that
    # path is the known-smooth baseline for macOS server -> Windows client.
    BARRIER_ARCH="x86_64"
else
    BARRIER_ARCH="$HOST_ARCH"
fi

BUILD_DIR="$ROOT_DIR/build/flowdesk-helper-$BARRIER_ARCH"
OUTPUT_DIR="$ROOT_DIR/build/flowdesk-helper/bin"
OUTPUT_BIN="$OUTPUT_DIR/barriers"

CMAKE_ARGS=(
    -S "$ROOT_DIR"
    -B "$BUILD_DIR"
    -DCMAKE_BUILD_TYPE="${B_BUILD_TYPE:-Release}"
    -DCMAKE_POLICY_VERSION_MINIMUM=3.5
    -DBARRIER_BUILD_GUI=OFF
    -DBARRIER_BUILD_INSTALLER=OFF
    -DBARRIER_BUILD_TESTS=OFF
)

if [[ "$HOST_OS" == "Darwin" ]]; then
    CMAKE_ARGS+=(
        "-DCMAKE_OSX_SYSROOT=$(xcode-select --print-path)/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk"
        -DCMAKE_OSX_DEPLOYMENT_TARGET=10.15
        "-DCMAKE_OSX_ARCHITECTURES=$BARRIER_ARCH"
    )

    if [[ "$BARRIER_ARCH" == "x86_64" ]]; then
        OPENSSL_LIB="/usr/local/opt/openssl/lib/libssl.a"
        if [[ ! -f "$OPENSSL_LIB" ]] || ! lipo -archs "$OPENSSL_LIB" 2>/dev/null | grep -qw x86_64; then
            LEGACY_BARRIER="/Applications/Barrier.app/Contents/MacOS/barriers"
            if [[ -f "$LEGACY_BARRIER" ]] && lipo -archs "$LEGACY_BARRIER" 2>/dev/null | grep -qw x86_64; then
                mkdir -p "$OUTPUT_DIR"
                cp "$LEGACY_BARRIER" "$OUTPUT_BIN"
                chmod +x "$OUTPUT_BIN"
                echo "FlowDesk bundled barriers helper (reused local x86_64 Barrier.app because x86_64 OpenSSL is unavailable):"
                file "$OUTPUT_BIN"
                exit 0
            else
                cat >&2 <<'EOF'
FlowDesk needs an x86_64 OpenSSL install to build the macOS bundled barriers helper.

Install OpenSSL under Rosetta/Homebrew (usually /usr/local/opt/openssl), or set
FLOWDESK_BARRIERS_ARCH=arm64 to build the native helper for local diagnostics.
EOF
                exit 1
            fi
        fi
    fi
fi

cmake "${CMAKE_ARGS[@]}"
cmake --build "$BUILD_DIR" --target barriers --parallel

mkdir -p "$OUTPUT_DIR"
cp "$BUILD_DIR/bin/barriers" "$OUTPUT_BIN"
chmod +x "$OUTPUT_BIN"

echo "FlowDesk bundled barriers helper:"
file "$OUTPUT_BIN"
