#!/usr/bin/env bash
# Post-bundle hook: copy barriers from Contents/Resources/bin/ to
# Contents/Helpers/bin/ so it sits in the macOS-standard helper-tools
# location.
#
# Why Helpers/ instead of MacOS/?
#   - MacOS/ : any executable here shows up as a separate app instance in
#     the Dock (duplicate icon problem).
#   - Resources/ : Tauri's default; works but doesn't inherit Accessibility.
#   - Helpers/ : macOS-standard for bundled helper tools. No Dock icon,
#     and a single bundle grant covers the whole app.
#
# Tauri's bundle.resources can only place files under Contents/Resources/,
# so we copy into Contents/Helpers/ after bundling.
set -euo pipefail

APP_BUNDLE="${1:?usage: move-barriers-to-helpers.sh <path-to-FlowDesk.app>}"

if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "move-barriers: app bundle not found at $APP_BUNDLE" >&2
    exit 1
fi

SRC="$APP_BUNDLE/Contents/Resources/bin/barriers"
DST_DIR="$APP_BUNDLE/Contents/Helpers/bin"
DST="$DST_DIR/barriers"

if [[ ! -f "$SRC" ]]; then
    echo "move-barriers: source barriers not found at $SRC" >&2
    exit 1
fi

mkdir -p "$DST_DIR"
cp "$SRC" "$DST"
chmod +x "$DST"
echo "move-barriers: copied barriers to $DST"

# Remove the MacOS/bin copy if a prior run left one (avoids Dock dup icon).
if [[ -f "$APP_BUNDLE/Contents/MacOS/bin/barriers" ]]; then
    rm -f "$APP_BUNDLE/Contents/MacOS/bin/barriers"
    rmdir "$APP_BUNDLE/Contents/MacOS/bin" 2>/dev/null || true
    echo "move-barriers: removed stale MacOS/bin/barriers"
fi
