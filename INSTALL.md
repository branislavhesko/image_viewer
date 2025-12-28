# Image Viewer Installation Guide

## Quick Download (Prebuilt Binaries)

Download the latest release from the [Releases page](https://github.com/branislavhesko/image_viewer/releases):

- **macOS (Apple Silicon/ARM64)**: Download `ImageViewer-macos-arm64.app.zip`, unzip, and move to Applications
- **Linux (x86_64)**: Download `image_viewer-linux-x86_64` and make executable
- **Windows (x86_64)**: Download `image_viewer-windows-x86_64.exe`

**Important**: macOS Intel (x86_64) is NOT supported. Only Apple Silicon Macs (M1/M2/M3/M4) are supported.

## Prerequisites (Building from Source)

- Rust toolchain (install from https://rustup.rs/)
- Git (for cloning the repository)

## macOS Installation

### Method 1: Install to Applications Folder (Recommended)

1. **Build the release version:**
   ```bash
   cd /path/to/image_viewer
   cargo build --release
   ```

2. **Create the application bundle:**
   ```bash
   # Create the app structure
   mkdir -p ImageViewer.app/Contents/MacOS
   mkdir -p ImageViewer.app/Contents/Resources

   # Copy the binary
   cp target/release/image_viewer ImageViewer.app/Contents/MacOS/

   # Copy the icon
   cp assets/icon.png ImageViewer.app/Contents/Resources/
   ```

3. **Create Info.plist file:**
   ```bash
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
   ```

4. **Install to Applications:**
   ```bash
   # Remove old version if it exists
   rm -rf /Applications/ImageViewer.app

   # Copy to Applications
   cp -r ImageViewer.app /Applications/

   # Make executable
   chmod +x /Applications/ImageViewer.app/Contents/MacOS/image_viewer
   ```

5. **Launch the application:**
   - Open Finder → Applications → ImageViewer
   - Or use Spotlight: Press Cmd+Space and type "Image Viewer"

### Method 2: Quick Install Script

Create an install script:

```bash
cat > install.sh << 'SCRIPT'
#!/bin/bash
set -e

echo "Building Image Viewer..."
cargo build --release

echo "Creating app bundle..."
rm -rf ImageViewer.app
mkdir -p ImageViewer.app/Contents/{MacOS,Resources}

cp target/release/image_viewer ImageViewer.app/Contents/MacOS/
cp assets/icon.png ImageViewer.app/Contents/Resources/ 2>/dev/null || echo "Warning: icon.png not found"

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
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Image</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.image</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
EOF

echo "Installing to /Applications..."
sudo rm -rf /Applications/ImageViewer.app
sudo cp -r ImageViewer.app /Applications/
sudo chmod +x /Applications/ImageViewer.app/Contents/MacOS/image_viewer

echo "✓ Installation complete!"
echo "You can now find 'Image Viewer' in your Applications folder"
SCRIPT

chmod +x install.sh
./install.sh
```

## Linux Installation

### System-wide Installation

1. **Build the release version:**
   ```bash
   cargo build --release
   ```

2. **Install the binary:**
   ```bash
   sudo cp target/release/image_viewer /usr/local/bin/
   sudo chmod +x /usr/local/bin/image_viewer
   ```

3. **Create desktop entry:**
   ```bash
   sudo tee /usr/share/applications/image-viewer.desktop << 'EOF'
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
   ```

4. **Install icon (optional):**
   ```bash
   sudo cp assets/icon.png /usr/share/icons/hicolor/256x256/apps/image-viewer.png
   sudo gtk-update-icon-cache /usr/share/icons/hicolor/ 2>/dev/null || true
   ```

5. **Update desktop database:**
   ```bash
   sudo update-desktop-database
   ```

### User Installation (no sudo required)

```bash
# Build
cargo build --release

# Install to user's local bin
mkdir -p ~/.local/bin
cp target/release/image_viewer ~/.local/bin/

# Create desktop entry
mkdir -p ~/.local/share/applications
cat > ~/.local/share/applications/image-viewer.desktop << 'EOF'
[Desktop Entry]
Type=Application
Name=Image Viewer
Exec=$HOME/.local/bin/image_viewer %f
Icon=image-viewer
Terminal=false
Categories=Graphics;Viewer;
EOF

# Update desktop database
update-desktop-database ~/.local/share/applications/
```

## Windows Installation

### Method 1: Manual Installation

1. **Build the release version:**
   ```powershell
   cargo build --release
   ```

2. **Create installation directory:**
   ```powershell
   mkdir "C:\Program Files\ImageViewer"
   copy target\release\image_viewer.exe "C:\Program Files\ImageViewer\"
   ```

3. **Add to PATH (optional):**
   - Right-click "This PC" → Properties → Advanced system settings
   - Environment Variables → System Variables → Path → Edit
   - Add: `C:\Program Files\ImageViewer`

4. **Create shortcut:**
   - Right-click on Desktop → New → Shortcut
   - Location: `C:\Program Files\ImageViewer\image_viewer.exe`
   - Name: "Image Viewer"

### Method 2: Create Installer with cargo-wix

```bash
# Install cargo-wix
cargo install cargo-wix

# Create WiX configuration
cargo wix init

# Build installer
cargo wix

# The installer will be in target/wix/
```

## Uninstallation

### macOS
```bash
sudo rm -rf /Applications/ImageViewer.app
```

### Linux (system-wide)
```bash
sudo rm /usr/local/bin/image_viewer
sudo rm /usr/share/applications/image-viewer.desktop
sudo rm /usr/share/icons/hicolor/256x256/apps/image-viewer.png
```

### Linux (user installation)
```bash
rm ~/.local/bin/image_viewer
rm ~/.local/share/applications/image-viewer.desktop
```

### Windows
```powershell
rmdir /s "C:\Program Files\ImageViewer"
```

## Development Mode

To run without installing:

```bash
cargo run --release
```

## Troubleshooting

### macOS: "App is damaged and can't be opened"

This happens because the app isn't signed. To fix:

```bash
xattr -cr /Applications/ImageViewer.app
```

Or allow it in System Preferences → Security & Privacy.

### Linux: Command not found

Make sure `~/.local/bin` is in your PATH:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### General: Missing dependencies

If you get compilation errors, install system dependencies:

**macOS:**
```bash
# Usually no additional dependencies needed
```

**Linux (Debian/Ubuntu):**
```bash
sudo apt install build-essential libgtk-3-dev
```

**Linux (Fedora):**
```bash
sudo dnf install gtk3-devel
```

## Updates

To update to a newer version:

1. Pull the latest code:
   ```bash
   git pull
   ```

2. Rebuild and reinstall:
   ```bash
   cargo build --release
   # Then follow the installation steps for your platform
   ```

## Features

After installation, you can:

- Open images from the application
- Drag and drop images onto the window
- Navigate through images in a folder with arrow keys
- Use drawing tools to annotate images
- Save annotations as separate JSON files
- Export annotated images

For more information, see README.md
