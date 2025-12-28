# Image Viewer

A minimalistic Rust image viewer with advanced features for scientific and technical image analysis. Built with eframe/egui for a responsive GUI experience.

![Image Viewer Screenshot](assets/app.png)

## Features

### Image Format Support
- **Standard formats**: PNG, JPG, JPEG, BMP, TIF, TIFF, WebP, GIF, AVIF, HDR, EXR, Farbfeld, QOI, DDS, TGA, PNM, FF, ICO
- **Floating point TIFF**: Special support for 32-bit floating point TIFF files (Gray, RGB, RGBA)
- **Drag & drop**: Drop image files directly onto the window
- **Command line**: Load images by passing file path as argument

### Image Analysis Tools

#### Normalization Types
- **None**: Display original image data
- **Min-Max**: Normalize pixel values to 0-255 range
- **Log Min-Max**: Logarithmic normalization for better visualization of wide dynamic range
- **Standard**: Standardization using mean and standard deviation
- **FFT**: Fast Fourier Transform visualization with windowing function

#### Channel Viewing
- **RGB**: View all channels combined
- **Red**: View only the red channel
- **Green**: View only the green channel
- **Blue**: View only the blue channel

#### Histogram Analysis
- **Multi-channel histogram**: Separate histograms for Red, Green, and Blue channels
- **Hover information**: Displays bin number, count, and percentage when hovering
- **Floating point support**: Calculates histograms from original data when available

#### Pixel Information Tool
- **Coordinate display**: Shows (x, y) coordinates of clicked pixel
- **Value display**: Shows RGB values for regular images or floating point values for FP images
- **Channel-aware**: Displays appropriate format based on image type (Grayscale vs RGB)

### Drawing and Annotation Tools

#### Drawing Modes
- **Free Draw**: Freehand drawing with mouse
- **Rectangle**: Draw rectangular shapes (filled or outline)
- **Line**: Draw straight lines
- **Arrow**: Draw arrows for annotations
- **Text**: Add text labels to images

#### Drawing Features
- **Color picker**: Choose any color for drawings
- **Stroke thickness**: Adjustable line width (1-20 pixels)
- **Fill option**: Toggle filled/outline for rectangles
- **Font size**: Adjustable text size (8-72 points)
- **Undo/Redo**: Full undo/redo support (Ctrl/Cmd+Z, Ctrl/Cmd+Y)
- **Clear all**: Remove all annotations at once

#### Annotation Persistence
- **Save annotations**: Save drawings as separate `.annotations.json` files
- **Auto-load**: Annotations automatically load with images
- **Export**: Bake annotations into new image files (PNG/JPEG)
- **Non-destructive**: Original images remain untouched

## Controls

### Mouse Interaction
- **Zoom**: CTRL + Mouse wheel to zoom in/out (0.1x to 20x magnification)
- **Pan**: Left mouse button drag to pan the image (when not in drawing mode)
- **Draw**: Click and drag to draw when drawing mode is enabled
- **Pixel sampling**: Left click to sample pixel values (when pixel tool is enabled)

### Keyboard Shortcuts
- **Arrow Left/Right**: Navigate to previous/next image in folder
- **Ctrl/Cmd + Z**: Undo last drawing action
- **Ctrl/Cmd + Shift + Z** or **Ctrl/Cmd + Y**: Redo drawing action

### UI Controls
- **Open Image**: Button to open file dialog
- **Scale slider**: Manual zoom control
- **Normalization**: Radio buttons to select normalization type
- **Channel dropdown**: Select which channels to display
- **Pixel Info checkbox**: Toggle pixel inspection mode
- **Histogram button**: Toggle histogram window
- **Drawing Mode checkbox**: Enable/disable annotation tools

### Loading Images
- **File dialog**: Use "Open Image" button
- **Drag & drop**: Drop image files onto the window
- **Command line**: `./image_viewer path/to/image.jpg`

## Advanced Features

### Floating Point Image Support
- Direct TIFF decoder for 32-bit floating point formats
- Preserves original floating point values for accurate analysis
- Proper normalization handling for floating point ranges
- Shows true floating point values in pixel sampling

### Performance Optimizations
- Texture caching to avoid unnecessary regeneration
- Smart scaling that only resizes when displaying smaller than original
- Lazy histogram calculation only when window is opened
- Efficient GPU-based image rendering

## Installation

### Quick Install (macOS/Linux)

```bash
git clone https://github.com/branislavhesko/image_viewer.git
cd image_viewer
./install.sh
```

This will:
- Build the release version
- Create a system application
- Install to `/Applications` (macOS) or system/user bin (Linux)

### Manual Installation

See [INSTALL.md](INSTALL.md) for detailed installation instructions including:
- macOS app bundle creation
- Linux system-wide or user installation
- Windows installation
- Uninstallation steps

### From Source (Development)
```bash
git clone https://github.com/branislavhesko/image_viewer.git
cd image_viewer
cargo build --release
cargo run --release
```

### From Releases
Download precompiled binaries from the [Releases page](https://github.com/branislavhesko/image_viewer/releases):
- **Linux (x86_64)**: `image_viewer-linux-x86_64`
- **Windows (x86_64)**: `image_viewer-windows-x86_64.exe`
- **macOS (Apple Silicon/ARM64)**: `ImageViewer-macos-arm64.app.zip`

**Note**: macOS Intel (x86_64) is not supported. Only Apple Silicon Macs (M1, M2, M3, etc.) are supported.

## Usage Examples

```bash
# Open file dialog
./image_viewer

# Load specific image
./image_viewer path/to/image.tiff

# Load floating point TIFF for scientific analysis
./image_viewer scientific_data.tiff
```

## Requirements

### Linux
- OpenGL support
- GTK development libraries (for file dialogs)

### Windows
- Windows 7 or later
- OpenGL support

Feature requests and bug reports are welcome!
