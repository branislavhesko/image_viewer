#!/bin/bash
set -e

echo "========================================="
echo "  Image Viewer Installation Script"
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

# Build release version
echo "Building release version..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed"
    exit 1
fi

echo "✓ Build successful"
echo ""

# Platform-specific installation
if [ "$PLATFORM" == "macos" ]; then
    echo "Creating macOS app bundle..."

    # Clean up old bundle
    rm -rf ImageViewer.app

    # Create directory structure
    mkdir -p ImageViewer.app/Contents/MacOS
    mkdir -p ImageViewer.app/Contents/Resources

    # Copy binary
    cp target/release/image_viewer ImageViewer.app/Contents/MacOS/

    # Copy icon if it exists
    if [ -f "assets/icon.png" ]; then
        cp assets/icon.png ImageViewer.app/Contents/Resources/
    else
        echo "⚠ Warning: assets/icon.png not found, skipping icon"
    fi

    # Create Info.plist
    cat > ImageViewer.app/Contents/Info.plist << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>image_viewer</string>
    <key>CFBundleIconFile</key>
    <string>icon.png</string>
    <key>CFBundleIdentifier</key>
    <string>com.imageviewer.app</string>
    <key>CFBundleName</key>
    <string>Image Viewer</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.1.0</string>
    <key>CFBundleVersion</key>
    <string>1.1.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Image</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.image</string>
                <string>public.png</string>
                <string>public.jpeg</string>
                <string>public.tiff</string>
                <string>com.compuserve.gif</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

    echo "✓ App bundle created"
    echo ""

    # Install to Applications
    echo "Installing to /Applications..."

    # Remove old version
    sudo rm -rf /Applications/ImageViewer.app

    # Copy new version
    sudo cp -r ImageViewer.app /Applications/

    # Make executable
    sudo chmod +x /Applications/ImageViewer.app/Contents/MacOS/image_viewer

    # Fix Gatekeeper issues
    sudo xattr -cr /Applications/ImageViewer.app 2>/dev/null || true

    echo "✓ Installed to /Applications/ImageViewer.app"
    echo ""
    echo "========================================="
    echo "✓ Installation complete!"
    echo "========================================="
    echo ""
    echo "You can now:"
    echo "  • Open 'Image Viewer' from your Applications folder"
    echo "  • Use Spotlight (Cmd+Space) and search 'Image Viewer'"
    echo "  • Right-click images → Open With → Image Viewer"
    echo ""

elif [ "$PLATFORM" == "linux" ]; then
    echo "Installing for Linux..."

    # Ask for installation type
    echo "Choose installation type:"
    echo "  1) System-wide (requires sudo, available for all users)"
    echo "  2) User-only (no sudo required, only for current user)"
    read -p "Enter choice (1 or 2): " choice
    echo ""

    if [ "$choice" == "1" ]; then
        # System-wide installation
        echo "Installing system-wide..."

        sudo cp target/release/image_viewer /usr/local/bin/
        sudo chmod +x /usr/local/bin/image_viewer

        # Create desktop entry
        sudo tee /usr/share/applications/image-viewer.desktop > /dev/null << 'EOF'
[Desktop Entry]
Type=Application
Name=Image Viewer
Comment=A simple image viewer with annotation tools
Exec=/usr/local/bin/image_viewer %f
Icon=image-viewer
Terminal=false
Categories=Graphics;Viewer;
MimeType=image/png;image/jpeg;image/jpg;image/gif;image/bmp;image/tiff;image/webp;
EOF

        # Install icon if available
        if [ -f "assets/icon.png" ]; then
            sudo mkdir -p /usr/share/icons/hicolor/256x256/apps/
            sudo cp assets/icon.png /usr/share/icons/hicolor/256x256/apps/image-viewer.png
            sudo gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true
        fi

        # Update desktop database
        sudo update-desktop-database 2>/dev/null || true

        echo "✓ Installed to /usr/local/bin/image_viewer"

    else
        # User installation
        echo "Installing for current user..."

        mkdir -p ~/.local/bin
        cp target/release/image_viewer ~/.local/bin/
        chmod +x ~/.local/bin/image_viewer

        # Create desktop entry
        mkdir -p ~/.local/share/applications
        cat > ~/.local/share/applications/image-viewer.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Image Viewer
Comment=A simple image viewer with annotation tools
Exec=$HOME/.local/bin/image_viewer %f
Icon=image-viewer
Terminal=false
Categories=Graphics;Viewer;
MimeType=image/png;image/jpeg;image/jpg;image/gif;image/bmp;image/tiff;image/webp;
EOF

        # Update desktop database
        update-desktop-database ~/.local/share/applications/ 2>/dev/null || true

        echo "✓ Installed to ~/.local/bin/image_viewer"
        echo ""
        echo "Note: Make sure ~/.local/bin is in your PATH"
        echo "Add this to your ~/.bashrc if needed:"
        echo '  export PATH="$HOME/.local/bin:$PATH"'
    fi

    echo ""
    echo "========================================="
    echo "✓ Installation complete!"
    echo "========================================="
    echo ""
    echo "You can now run 'image_viewer' from the command line"
    echo "or find 'Image Viewer' in your application menu"
    echo ""
fi
