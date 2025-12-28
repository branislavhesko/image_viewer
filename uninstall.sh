#!/bin/bash

echo "========================================="
echo "  Image Viewer Uninstallation Script"
echo "========================================="
echo ""

# Detect platform
if [[ "$OSTYPE" == "darwin"* ]]; then
    PLATFORM="macos"
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    PLATFORM="linux"
else
    echo "Unsupported platform: $OSTYPE"
    exit 1
fi

echo "Detected platform: $PLATFORM"
echo ""

if [ "$PLATFORM" == "macos" ]; then
    # macOS uninstallation
    if [ -d "/Applications/ImageViewer.app" ]; then
        echo "Removing /Applications/ImageViewer.app..."
        sudo rm -rf /Applications/ImageViewer.app
        echo "✓ Removed"
    else
        echo "Image Viewer not found in /Applications"
    fi

elif [ "$PLATFORM" == "linux" ]; then
    # Linux uninstallation
    echo "Checking for installations..."
    echo ""

    FOUND=false

    # Check system-wide installation
    if [ -f "/usr/local/bin/image_viewer" ]; then
        echo "Found system-wide installation"
        read -p "Remove system-wide installation? (y/n): " choice
        if [ "$choice" == "y" ]; then
            sudo rm /usr/local/bin/image_viewer
            sudo rm /usr/share/applications/image-viewer.desktop 2>/dev/null
            sudo rm /usr/share/icons/hicolor/256x256/apps/image-viewer.png 2>/dev/null
            sudo update-desktop-database 2>/dev/null || true
            echo "✓ System-wide installation removed"
        fi
        FOUND=true
    fi

    # Check user installation
    if [ -f "$HOME/.local/bin/image_viewer" ]; then
        echo "Found user installation"
        read -p "Remove user installation? (y/n): " choice
        if [ "$choice" == "y" ]; then
            rm "$HOME/.local/bin/image_viewer"
            rm "$HOME/.local/share/applications/image-viewer.desktop" 2>/dev/null
            update-desktop-database ~/.local/share/applications/ 2>/dev/null || true
            echo "✓ User installation removed"
        fi
        FOUND=true
    fi

    if [ "$FOUND" == false ]; then
        echo "No Image Viewer installation found"
    fi
fi

echo ""
echo "========================================="
echo "✓ Uninstallation complete!"
echo "========================================="
