#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PDF_FILE="$SCRIPT_DIR/readme.pdf"
PNG_FILE="$SCRIPT_DIR/readme.png"
PREVIEW_SIZE="${PREVIEW_SIZE:-2400}"
PREVIEW_DPI="${PREVIEW_DPI:-600}"

if [ ! -f "$PDF_FILE" ]; then
    echo "Error: $PDF_FILE not found"
    exit 1
fi

flatten_white_background() {
    if command -v magick &> /dev/null; then
        magick "$PNG_FILE" -background white -alpha remove -alpha off "$PNG_FILE"
    elif command -v convert &> /dev/null; then
        convert "$PNG_FILE" -background white -alpha remove -alpha off "$PNG_FILE"
    fi
}

echo "Converting $PDF_FILE to PNG..."

# Prefer macOS Quick Look because it uses the system PDF renderer and preserves text
# positioning more faithfully than ImageMagick's PDF delegate path.
if command -v qlmanage &> /dev/null; then
    TMP_DIR="$(mktemp -d)"
    qlmanage -t -s "$PREVIEW_SIZE" -o "$TMP_DIR" "$PDF_FILE" >/dev/null 2>&1
    mv "$TMP_DIR/$(basename "$PDF_FILE").png" "$PNG_FILE"
    rmdir "$TMP_DIR"
    echo "✓ Successfully converted to $PNG_FILE using Quick Look"
# Ghostscript is a stable cross-platform fallback when Quick Look is unavailable.
elif command -v gs &> /dev/null; then
    gs -q -dNOPAUSE -dBATCH -dSAFER -sDEVICE=png16m -dTextAlphaBits=4 -dGraphicsAlphaBits=4 -dBackgroundColor=16#ffffff -r"$PREVIEW_DPI" -sOutputFile="$PNG_FILE" "$PDF_FILE"
    echo "✓ Successfully converted to $PNG_FILE using Ghostscript"
elif command -v magick &> /dev/null; then
    magick -density "$PREVIEW_DPI" -quality 99 -background white -flatten "$PDF_FILE" "$PNG_FILE"
    echo "✓ Successfully converted to $PNG_FILE using ImageMagick"
elif command -v convert &> /dev/null; then
    convert -density "$PREVIEW_DPI" -quality 99 -background white -flatten "$PDF_FILE" "$PNG_FILE"
    echo "✓ Successfully converted to $PNG_FILE using ImageMagick"
else
    echo "Error: No PDF rasterizer found"
    echo "Install one of these to use this script:"
    echo "  - Ghostscript: brew install ghostscript"
    echo "  - ImageMagick: brew install imagemagick"
    exit 1
fi

flatten_white_background
