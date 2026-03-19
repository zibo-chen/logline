#!/bin/bash
# Build Linux .deb package for Logline
# Usage: ./build-deb.sh <version> <arch>
# Example: ./build-deb.sh 1.4.2 x86_64
#          ./build-deb.sh 1.4.2 aarch64

set -euo pipefail

VERSION="${1:?Usage: $0 <version> <arch>}"
ARCH="${2:?Usage: $0 <version> <arch>}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Map arch to dpkg architecture name
case "$ARCH" in
    x86_64)  DEB_ARCH="amd64"; TARGET="x86_64-unknown-linux-gnu" ;;
    aarch64) DEB_ARCH="arm64";  TARGET="aarch64-unknown-linux-gnu" ;;
    *)       echo "Unknown arch: $ARCH"; exit 1 ;;
esac

DEB_NAME="logline_${VERSION}_${DEB_ARCH}"
echo "Building deb package: ${DEB_NAME}.deb"

BINARY="$PROJECT_DIR/target/${TARGET}/release/logline"
if [ ! -f "$BINARY" ]; then
    BINARY="$PROJECT_DIR/target/release/logline"
fi

if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found. Build first with: cargo build --release --target $TARGET"
    exit 1
fi

# Create deb package structure
BUILD_DIR="$PROJECT_DIR/target/installer"
DEB_DIR="$BUILD_DIR/$DEB_NAME"
rm -rf "$DEB_DIR"

# Directory structure
mkdir -p "$DEB_DIR/DEBIAN"
mkdir -p "$DEB_DIR/usr/bin"
mkdir -p "$DEB_DIR/usr/share/applications"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/256x256/apps"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/128x128/apps"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/64x64/apps"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/48x48/apps"
mkdir -p "$DEB_DIR/usr/share/icons/hicolor/32x32/apps"
mkdir -p "$DEB_DIR/usr/share/mime/packages"

# Copy binary
cp "$BINARY" "$DEB_DIR/usr/bin/logline"
chmod 755 "$DEB_DIR/usr/bin/logline"

# Strip binary to reduce size
strip "$DEB_DIR/usr/bin/logline" 2>/dev/null || true

# Copy desktop entry
cp "$SCRIPT_DIR/logline.desktop" "$DEB_DIR/usr/share/applications/logline.desktop"

# Copy MIME type definition
cp "$SCRIPT_DIR/logline-mime.xml" "$DEB_DIR/usr/share/mime/packages/logline.xml"

# Generate icons at multiple sizes from source PNG
ICON_PNG="$PROJECT_DIR/res/icon.png"
if command -v convert &> /dev/null; then
    for SIZE in 256 128 64 48 32; do
        convert "$ICON_PNG" -resize "${SIZE}x${SIZE}" \
            "$DEB_DIR/usr/share/icons/hicolor/${SIZE}x${SIZE}/apps/logline.png"
    done
elif command -v magick &> /dev/null; then
    for SIZE in 256 128 64 48 32; do
        magick "$ICON_PNG" -resize "${SIZE}x${SIZE}" \
            "$DEB_DIR/usr/share/icons/hicolor/${SIZE}x${SIZE}/apps/logline.png"
    done
else
    # Fallback: copy original to 256x256
    cp "$ICON_PNG" "$DEB_DIR/usr/share/icons/hicolor/256x256/apps/logline.png"
    echo "Warning: ImageMagick not found, only 256x256 icon installed"
fi

# Calculate installed size in KB
INSTALLED_SIZE=$(du -sk "$DEB_DIR" | cut -f1)

# Create control file
cat > "$DEB_DIR/DEBIAN/control" << EOF
Package: logline
Version: ${VERSION}
Section: utils
Priority: optional
Architecture: ${DEB_ARCH}
Installed-Size: ${INSTALLED_SIZE}
Depends: libgtk-3-0, libxcb-render0, libxcb-shape0, libxcb-xfixes0, libxkbcommon0
Maintainer: Zibo Chen <qw.54@163.com>
Homepage: https://github.com/zibo-chen/logline
Description: High-performance real-time log viewer
 Logline is a cross-platform log viewer with advanced features including
 real-time log monitoring, powerful search and filtering, syntax highlighting,
 remote log streaming, and AI-powered log analysis via MCP.
EOF

# Post-install script to update caches
cat > "$DEB_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/bash
set -e
# Update icon cache
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
fi
# Update MIME database
if command -v update-mime-database &> /dev/null; then
    update-mime-database /usr/share/mime 2>/dev/null || true
fi
# Update desktop database
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
EOF
chmod 755 "$DEB_DIR/DEBIAN/postinst"

# Post-remove script
cat > "$DEB_DIR/DEBIAN/postrm" << 'EOF'
#!/bin/bash
set -e
if command -v gtk-update-icon-cache &> /dev/null; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
fi
if command -v update-mime-database &> /dev/null; then
    update-mime-database /usr/share/mime 2>/dev/null || true
fi
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi
EOF
chmod 755 "$DEB_DIR/DEBIAN/postrm"

# Build the deb package
DEB_PATH="$BUILD_DIR/${DEB_NAME}.deb"
dpkg-deb --build --root-owner-group "$DEB_DIR" "$DEB_PATH"

rm -rf "$DEB_DIR"

echo "Deb package created: $DEB_PATH"
