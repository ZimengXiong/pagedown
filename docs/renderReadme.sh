#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PDF_FILE="$SCRIPT_DIR/readme.pdf"
PNG_FILE="$SCRIPT_DIR/readme.png"

if [ ! -f "$PDF_FILE" ]; then
    echo "Error: $PDF_FILE not found"
    exit 1
fi

echo "Converting $PDF_FILE to PNG..."

# Try using ImageMagick first (preferred, better quality)
if command -v convert &> /dev/null; then
    convert -density 150 -quality 90 -background white -flatten "$PDF_FILE" "$PNG_FILE"
    echo "✓ Successfully converted to $PNG_FILE using ImageMagick"
# Fallback to Ghostscript
elif command -v gs &> /dev/null; then
    gs -q -dNOPAUSE -dBATCH -dSAFER -sDEVICE=png16m -dBackgroundColor=16#ffffff -r150 -sOutputFile="$PNG_FILE" "$PDF_FILE"
    echo "✓ Successfully converted to $PNG_FILE using Ghostscript"
else
    echo "Error: Neither ImageMagick (convert) nor Ghostscript (gs) found"
    echo "Install one of them to use this script:"
    echo "  - ImageMagick: brew install imagemagick"
    echo "  - Ghostscript: brew install ghostscript"
    exit 1
fi
