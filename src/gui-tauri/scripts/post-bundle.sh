#!/usr/bin/env bash
# Post-bundle hook for FlowDesk on macOS.
#
# Does two things Tauri's bundler can't do via config:
#   1. Copy barriers from Contents/Resources/bin/ to Contents/Helpers/bin/
#      (Helpers/ is the macOS-standard helper-tools location; unlike MacOS/,
#      it does NOT cause a duplicate Dock icon).
#   2. Inject privacy usage descriptions into Info.plist. macOS 15+ requires
#      these keys to be present before it will honor an Accessibility or
#      Screen Recording grant for the bundle.
set -euo pipefail

APP_BUNDLE="${1:?usage: post-bundle.sh <path-to-FlowDesk.app>}"

if [[ ! -d "$APP_BUNDLE" ]]; then
    echo "post-bundle: app bundle not found at $APP_BUNDLE" >&2
    exit 1
fi

# --- 1. Move barriers to Helpers/ ---
SRC="$APP_BUNDLE/Contents/Resources/bin/barriers"
DST_DIR="$APP_BUNDLE/Contents/Helpers/bin"
DST="$DST_DIR/barriers"

if [[ ! -f "$SRC" ]]; then
    echo "post-bundle: source barriers not found at $SRC" >&2
    exit 1
fi

mkdir -p "$DST_DIR"
cp "$SRC" "$DST"
chmod +x "$DST"
echo "post-bundle: copied barriers to $DST"

# Remove any stale MacOS/bin copy from a prior layout (avoids Dock dup icon).
if [[ -f "$APP_BUNDLE/Contents/MacOS/bin/barriers" ]]; then
    rm -f "$APP_BUNDLE/Contents/MacOS/bin/barriers"
    rmdir "$APP_BUNDLE/Contents/MacOS/bin" 2>/dev/null || true
    echo "post-bundle: removed stale MacOS/bin/barriers"
fi

# --- 2. Inject privacy usage descriptions into Info.plist ---
PLIST="$APP_BUNDLE/Contents/Info.plist"

/usr/libexec/PlistBuddy -c "Add :NSAppleEventsUsageDescription string \
FlowDesk needs Accessibility permission to share your keyboard and mouse across computers." \
    "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSAppleEventsUsageDescription \
FlowDesk needs Accessibility permission to share your keyboard and mouse across computers." \
    "$PLIST"

/usr/libexec/PlistBuddy -c "Add :NSScreenCaptureUsageDescription string \
FlowDesk needs Screen Recording permission to track the mouse position for seamless cursor movement between screens." \
    "$PLIST" 2>/dev/null || \
/usr/libexec/PlistBuddy -c "Set :NSScreenCaptureUsageDescription \
FlowDesk needs Screen Recording permission to track the mouse position for seamless cursor movement between screens." \
    "$PLIST"

echo "post-bundle: injected privacy usage descriptions into Info.plist"
echo "post-bundle: done"
