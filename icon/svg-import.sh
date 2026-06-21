#!/bin/bash
# svg-import.sh - Generate static icon files from SVG
# Run this script manually when you update the SVG icon

set -e

# Check if we're in the icon directory
if [ ! -f "rustyip.svg" ]; then
    echo "❌ Error: rustyip.svg not found in current directory"
    echo "Please run this script from the icon/ directory"
    exit 1
fi

# Check if ImageMagick is installed
MAGICK_CMD=""
if command -v magick &> /dev/null; then
    MAGICK_CMD="magick"
    echo "✅ Found ImageMagick 7+ (magick command)"
elif command -v convert &> /dev/null; then
    MAGICK_CMD="convert"
    echo "✅ Found ImageMagick 6 (convert command)"
else
    echo "❌ Error: ImageMagick not found"
    echo "Please install ImageMagick:"
    echo "  Windows: winget install ImageMagick.ImageMagick"
    echo "  macOS:   brew install imagemagick"
    echo "  Ubuntu:  sudo apt install imagemagick"
    exit 1
fi

echo "🎨 Generating icon files from rustyip.svg..."

# Generate Windows ICO file with multiple resolutions
echo "📦 Creating rustyip.ico (multi-resolution)..."
$MAGICK_CMD rustyip.svg \
    -background none \
    -fill none \
    -fuzz 0% \
    -transparent white \
    -filter point \
    -interpolate nearest \
    -define icon:auto-resize="256,128,64,48,32,24,20,16" \
    rustyip.ico

# Generate PNG
echo "📄 Creating rustyip.png (256x256)..."
$MAGICK_CMD rustyip.svg \
    -background none \
    -fill none \
    -fuzz 0% \
    -transparent white \
    -filter point \
    -interpolate nearest \
    -resize 256x256 \
    PNG32:rustyip.png

echo ""
echo "✅ Icon generation complete!"
echo ""
echo "📋 Generated files:"
ls -la *.ico *.png 2>/dev/null || echo "No files found"
