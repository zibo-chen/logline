#!/bin/bash
# Build macOS .app bundle and .dmg installer
# Usage: ./build-macos.sh <version> <arch>
# Example: ./build-macos.sh 1.4.2 x86_64
#          ./build-macos.sh 1.4.2 aarch64

set -euo pipefail

VERSION="${1:?Usage: $0 <version> <arch>}"
ARCH="${2:?Usage: $0 <version> <arch>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
APP_NAME="Logline"
BUNDLE_NAME="${APP_NAME}.app"
DMG_NAME="logline-${VERSION}-macos-${ARCH}.dmg"

echo "Building macOS installer: ${DMG_NAME}"

# Determine Rust target triple
case "$ARCH" in
    x86_64)  TARGET="x86_64-apple-darwin" ;;
    aarch64) TARGET="aarch64-apple-darwin" ;;
    *)       echo "Unknown arch: $ARCH"; exit 1 ;;
esac

BINARY="$PROJECT_DIR/target/${TARGET}/release/logline"
if [ ! -f "$BINARY" ]; then
    # Try release without target dir (native build)
    BINARY="$PROJECT_DIR/target/release/logline"
fi

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found. Build first with: cargo build --release --target $TARGET"
    exit 1
fi

# Create .app bundle structure
BUILD_DIR="$PROJECT_DIR/target/installer"
APP_DIR="$BUILD_DIR/${BUNDLE_NAME}"
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

# Copy binary
cp "$BINARY" "$APP_DIR/Contents/MacOS/logline"
chmod +x "$APP_DIR/Contents/MacOS/logline"

# Generate Info.plist from template
sed "s/APP_VERSION/${VERSION}/g" "$SCRIPT_DIR/Info.plist" > "$APP_DIR/Contents/Info.plist"

# Convert PNG icon to icns if sips/iconutil are available
ICON_PNG="$PROJECT_DIR/res/icon.png"
if command -v iconutil &> /dev/null && command -v sips &> /dev/null; then
    ICONSET_DIR="$BUILD_DIR/AppIcon.iconset"
    rm -rf "$ICONSET_DIR"
    mkdir -p "$ICONSET_DIR"

    # Generate all required icon sizes
    sips -z 16 16     "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16.png"      > /dev/null 2>&1
    sips -z 32 32     "$ICON_PNG" --out "$ICONSET_DIR/icon_16x16@2x.png"   > /dev/null 2>&1
    sips -z 32 32     "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32.png"      > /dev/null 2>&1
    sips -z 64 64     "$ICON_PNG" --out "$ICONSET_DIR/icon_32x32@2x.png"   > /dev/null 2>&1
    sips -z 128 128   "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128.png"    > /dev/null 2>&1
    sips -z 256 256   "$ICON_PNG" --out "$ICONSET_DIR/icon_128x128@2x.png" > /dev/null 2>&1
    sips -z 256 256   "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256.png"    > /dev/null 2>&1
    sips -z 512 512   "$ICON_PNG" --out "$ICONSET_DIR/icon_256x256@2x.png" > /dev/null 2>&1
    sips -z 512 512   "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512.png"    > /dev/null 2>&1
    sips -z 1024 1024 "$ICON_PNG" --out "$ICONSET_DIR/icon_512x512@2x.png" > /dev/null 2>&1

    iconutil -c icns "$ICONSET_DIR" -o "$APP_DIR/Contents/Resources/AppIcon.icns"
    rm -rf "$ICONSET_DIR"
    echo "Icon converted to icns"
else
    echo "Warning: iconutil not available, icon will not be embedded"
fi

# Create DMG
DMG_PATH="$BUILD_DIR/$DMG_NAME"
rm -f "$DMG_PATH"

# Create a temporary directory for DMG contents
DMG_STAGING="$BUILD_DIR/dmg-staging"
rm -rf "$DMG_STAGING"
mkdir -p "$DMG_STAGING"
cp -R "$APP_DIR" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

# Create DMG using hdiutil
hdiutil create -volname "Logline" \
    -srcfolder "$DMG_STAGING" \
    -ov -format UDZO \
    "$DMG_PATH"

rm -rf "$DMG_STAGING"

echo "DMG created: $DMG_PATH"
echo "App bundle: $APP_DIR"
