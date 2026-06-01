#!/bin/bash
#
# package-dmg.sh
#
# Build Claude Code Launcher for macOS, code-sign, notarize, and
# package as a distributable .dmg installer.
#
# Usage:
#   bash scripts/package-dmg.sh              # unsigned (local use only)
#   bash scripts/package-dmg.sh --sign       # signed + notarized (for distribution)
#
# Prerequisites for --sign:
#   - Apple Developer Program membership
#   - Developer ID Application certificate in Keychain
#   - App-specific password stored in Keychain:
#       xcrun notarytool store-credentials "notary-profile" \
#           --apple-id "your@email.com" \
#           --team-id "YOURTEAMID" \
#           --password "app-specific-password"
#
#   - Rust toolchain (cargo)
#   - macOS

set -euo pipefail

# ── Config ────────────────────────────────────────────────────────────

PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT_DIR"

APP_NAME="Claude Code Launcher"
BUNDLE_DIR="$PROJECT_DIR/target/Claude Code Launcher.app"
VERSION="${VERSION:-2.1.0}"
DMG_NAME="Claude-Code-Launcher-${VERSION}.dmg"
DMG_PATH="$PROJECT_DIR/target/$DMG_NAME"
BINARY_PATH="$PROJECT_DIR/target/release/claude-launcher"

# Code signing / notarization
DO_SIGN=false
DEVELOPER_ID="${DEVELOPER_ID:-}"
NOTARY_PROFILE="${NOTARY_PROFILE:-notary-profile}"

# ── Parse args ────────────────────────────────────────────────────────

while [[ $# -gt 0 ]]; do
    case "$1" in
        --sign) DO_SIGN=true; shift ;;
        --developer-id) DEVELOPER_ID="$2"; shift 2 ;;
        --notary-profile) NOTARY_PROFILE="$2"; shift 2 ;;
        --version) VERSION="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: bash scripts/package-dmg.sh [--sign] [--developer-id ID] [--notary-profile PROFILE]"
            echo ""
            echo "  --sign              Enable code signing + notarization"
            echo "  --developer-id ID   Developer ID Application certificate name"
            echo "  --notary-profile P  Keychain notary profile name (default: notary-profile)"
            echo "  --version V         Version string (default: 2.1.0)"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# ── Build ──────────────────────────────────────────────────────────────

echo "==> Building release binary..."
cargo build --release

# ── App bundle ─────────────────────────────────────────────────────────

echo "==> Creating .app bundle structure..."
rm -rf "$BUNDLE_DIR"
mkdir -p "$BUNDLE_DIR/Contents/MacOS"
mkdir -p "$BUNDLE_DIR/Contents/Resources"

cp "$BINARY_PATH" "$BUNDLE_DIR/Contents/MacOS/"
cp "$PROJECT_DIR/assets/Info.plist" "$BUNDLE_DIR/Contents/"

# ── Entitlements (required for notarization) ───────────────────────────

if $DO_SIGN; then
    ENTITLEMENTS_PATH="$PROJECT_DIR/target/entitlements.plist"
    cat > "$ENTITLEMENTS_PATH" <<'ENTITLEMENTS'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>com.apple.security.cs.allow-unsigned-executable-memory</key>
    <true/>
    <key>com.apple.security.cs.disable-library-validation</key>
    <true/>
</dict>
</plist>
ENTITLEMENTS
fi

# ── Icon ───────────────────────────────────────────────────────────────

echo "==> Generating app icon..."
ICONSET_DIR="$PROJECT_DIR/target/icon.iconset"
mkdir -p "$ICONSET_DIR"

if [ -f "$PROJECT_DIR/assets/icon.png" ]; then
    sips -z 16 16   "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_16x16.png" > /dev/null
    sips -z 32 32   "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_16x16@2x.png" > /dev/null
    sips -z 32 32   "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_32x32.png" > /dev/null
    sips -z 64 64   "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_32x32@2x.png" > /dev/null
    sips -z 128 128 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_128x128.png" > /dev/null
    sips -z 256 256 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_128x128@2x.png" > /dev/null
    sips -z 256 256 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_256x256.png" > /dev/null
    sips -z 512 512 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_256x256@2x.png" > /dev/null
    sips -z 512 512 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_512x512.png" > /dev/null
    sips -z 1024 1024 "$PROJECT_DIR/assets/icon.png" --out "$ICONSET_DIR/icon_512x512@2x.png" > /dev/null
    iconutil -c icns "$ICONSET_DIR" -o "$BUNDLE_DIR/Contents/Resources/icon.icns"
    rm -rf "$ICONSET_DIR"
else
    echo "WARNING: assets/icon.png not found, skipping icon generation"
fi

# ── Code signing ───────────────────────────────────────────────────────

if $DO_SIGN; then
    # Auto-detect Developer ID if not specified
    if [ -z "$DEVELOPER_ID" ]; then
        DEVELOPER_ID=$(security find-identity -v -p codesigning 2>/dev/null \
            | grep "Developer ID Application" \
            | head -1 \
            | sed -E 's/.*"([^"]+)".*/\1/' || true)
    fi

    if [ -z "$DEVELOPER_ID" ]; then
        echo "ERROR: No Developer ID Application certificate found in Keychain."
        echo "       Import your certificate first, or pass --developer-id explicitly."
        exit 1
    fi

    echo "==> Signing binary: $DEVELOPER_ID"
    codesign --force --options runtime --timestamp \
        --entitlements "$ENTITLEMENTS_PATH" \
        --sign "$DEVELOPER_ID" \
        "$BUNDLE_DIR/Contents/MacOS/claude-launcher"

    echo "==> Signing app bundle..."
    codesign --force --options runtime --timestamp \
        --entitlements "$ENTITLEMENTS_PATH" \
        --sign "$DEVELOPER_ID" \
        --deep "$BUNDLE_DIR"

    # Verify signature
    echo "==> Verifying signature..."
    codesign -dvv "$BUNDLE_DIR"
else
    echo "==> Skipping code signing (use --sign to enable)"
fi

# ── DMG ────────────────────────────────────────────────────────────────

echo "==> Creating DMG..."
rm -f "$DMG_PATH"
hdiutil create \
    -volname "$APP_NAME" \
    -srcfolder "$BUNDLE_DIR" \
    -ov \
    -format UDZO \
    "$DMG_PATH"

# ── Sign DMG ───────────────────────────────────────────────────────────

if $DO_SIGN; then
    echo "==> Signing DMG..."
    codesign --force --options runtime --timestamp \
        --sign "$DEVELOPER_ID" \
        "$DMG_PATH"

    # ── Notarization ────────────────────────────────────────────────

    echo "==> Submitting for notarization..."
    notary_output=$(xcrun notarytool submit "$DMG_PATH" \
        --keychain-profile "$NOTARY_PROFILE" \
        --wait 2>&1)
    echo "$notary_output"

    notary_id=$(echo "$notary_output" | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)

    if [ -n "$notary_id" ]; then
        echo "==> Checking notarization log..."
        xcrun notarytool log "$notary_id" --keychain-profile "$NOTARY_PROFILE"
    fi

    # ── Staple ─────────────────────────────────────────────────────

    echo "==> Stapling notarization ticket..."
    xcrun stapler staple "$DMG_PATH"

    # Verify staple
    xcrun stapler validate "$DMG_PATH"
fi

# ── Cleanup ────────────────────────────────────────────────────────────

echo "==> Cleaning up..."
rm -rf "$BUNDLE_DIR"

# ── Done ───────────────────────────────────────────────────────────────

echo "==> Done: $DMG_PATH"
echo "    Attach with: hdiutil attach '$DMG_PATH'"

if $DO_SIGN; then
    echo ""
    echo "    To verify notarization:"
    echo "      spctl -a -vvv --type install '$DMG_PATH'"
fi
