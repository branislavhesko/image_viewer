#!/bin/bash

# Install desktop entry and icon for the image viewer (KDE optimized)

# Create directories if they don't exist
mkdir -p ~/.local/share/applications
mkdir -p ~/.local/share/icons/hicolor/16x16/apps
mkdir -p ~/.local/share/icons/hicolor/32x32/apps
mkdir -p ~/.local/share/icons/hicolor/48x48/apps
mkdir -p ~/.local/share/icons/hicolor/64x64/apps
mkdir -p ~/.local/share/icons/hicolor/128x128/apps
mkdir -p ~/.local/share/icons/hicolor/256x256/apps
mkdir -p ~/.local/share/icons/hicolor/512x512/apps

# Copy desktop entry
cp assets/image_viewer.desktop ~/.local/share/applications/

# Generate and copy icons in multiple sizes for KDE
convert assets/icon.png -resize 16x16 ~/.local/share/icons/hicolor/16x16/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/16x16/apps/image_viewer.png
convert assets/icon.png -resize 32x32 ~/.local/share/icons/hicolor/32x32/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/32x32/apps/image_viewer.png
convert assets/icon.png -resize 48x48 ~/.local/share/icons/hicolor/48x48/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/48x48/apps/image_viewer.png
convert assets/icon.png -resize 64x64 ~/.local/share/icons/hicolor/64x64/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/64x64/apps/image_viewer.png
convert assets/icon.png -resize 128x128 ~/.local/share/icons/hicolor/128x128/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/128x128/apps/image_viewer.png
cp assets/icon.png ~/.local/share/icons/hicolor/256x256/apps/image_viewer.png
convert assets/icon.png -resize 512x512 ~/.local/share/icons/hicolor/512x512/apps/image_viewer.png 2>/dev/null || cp assets/icon.png ~/.local/share/icons/hicolor/512x512/apps/image_viewer.png

# Update desktop database
update-desktop-database ~/.local/share/applications/ 2>/dev/null || true

# Update icon cache (try both GTK and KDE methods)
gtk-update-icon-cache ~/.local/share/icons/hicolor/ 2>/dev/null || true
kbuildsycoca5 --noincremental 2>/dev/null || kbuildsycoca6 --noincremental 2>/dev/null || true

# Refresh KDE desktop
qdbus org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.refreshCurrentShell 2>/dev/null || true

echo "Desktop entry and icon installed successfully for KDE!"
echo "Icons installed in multiple sizes for better KDE integration."
echo "The icon should now appear in Dolphin and other KDE applications."