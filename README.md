# terminal-media

[![Crates.io](https://img.shields.io/crates/v/terminal-media.svg)](https://crates.io/crates/terminal-media)
[![Documentation](https://docs.rs/terminal-media/badge.svg)](https://docs.rs/terminal-media)
[![License: GPL-2.0](https://img.shields.io/badge/License-GPL--2.0-blue.svg)](LICENSE)
[![Build Status](https://github.com/sandwichfarm/terminal-media/workflows/Release/badge.svg)](https://github.com/sandwichfarm/terminal-media/actions)
[![Rust Version](https://img.shields.io/badge/rust-2024%2B-orange.svg)](https://www.rust-lang.org)

`terminal-media` is a powerful terminal image and video viewer written in Rust. View images, videos, PDFs, and SVGs directly in your terminal without leaving your workflow.

## Table of Contents

- [Features](#features)
- [Building from Source](#building-from-source)
- [Using as a Library](#using-as-a-library)
  - [Adding the Dependency](#adding-the-dependency)
  - [Basic Usage](#basic-usage)
  - [Programmatic Configuration](#programmatic-configuration)
  - [Image Loading](#image-loading)
  - [Available Modules](#available-modules)
- [CLI Usage](#cli-usage)
- [File Lists](#file-lists)
- [Testing](#testing)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [Licensing](#licensing)

## Features

- **Multiple rendering modes:**
  - Unicode half-block renderer with 24-bit colour output (`-p half`)
  - Unicode quarter-block renderer for higher spatial resolution (`-p quarter`, default)
  - Kitty graphics protocol backend (`--backend kitty`)
  - iTerm2 inline image backend using OSC 1337 protocol (`--backend iterm2`)
  - Optional Sixel graphics emission (`--features sixel` + `--backend sixel`)
  - Automatic backend detection (`--backend auto`, default) with support for:
    - Kitty terminal and Ghostty (Kitty graphics protocol)
    - iTerm2, VSCode terminal, WezTerm (iTerm2 inline images)
    - mlterm, Windows Terminal (Sixel graphics)
    - Fallback to Unicode blocks for other terminals
  - Multiplexer support:
    - Automatic detection of tmux/screen
    - DCS passthrough wrapping for graphics protocols
    - Automatic tmux `allow-passthrough` enablement (tmux >= 3.3)
- **Wide format support:**
  - **Images:** PNG, JPEG, GIF, BMP, WebP, TIFF, EXR, TGA, DDS, HDR, ICO, PNM, QOI
  - **Videos:** MP4, MKV, MOV, AVI, WebM, and other ffmpeg-supported formats
  - **Documents:** PDF (multi-page support)
  - **Vector:** SVG, SVGZ (compressed SVG)
- **Flexible sizing:**
  - Automatic sizing against the active terminal
  - Manual width/height constraints (`-w`, `-H`)
  - Fit-to-width mode (`-W/--fit-width`)
  - Fit-to-height mode (`--fit-height`)
  - Upscaling support (`-U/--upscale`, `-U i` for integer scaling)
  - Width stretch correction (`--width-stretch`)
  - EXIF orientation support (automatic rotation for phone photos)
  - Auto-crop to remove uniform borders (`--auto-crop`)
  - Fixed border cropping (`--crop-border N`)
  - Antialiasing control (`-a/--antialias`)
- **Input sources:**
  - File paths (command line arguments)
  - Standard input (`-` for piping images)
  - File lists (`-f` and `-F` for batch processing)
- **Layout options:**
  - Grid layout (`--grid N` or `--grid NxM`) with configurable spacing
  - Image centering (`--center`)
  - Image scrolling animation (`--scroll` with `--delta-move`)
- **Display features:**
  - Timed slideshows (`--wait`, `--wait-rows`)
  - Per-image titles with format strings (`--title`)
  - Background/alpha controls (`--background`, `--pattern`, `--pattern-size`)
  - Alternate screen buffer support (`--alternate-screen`)
  - Screen clearing options (`--clear`, `--clear-between`)
  - Cursor hiding control (`--hide-cursor`)
  - 8-bit color mode for older terminals (`--color8`)
  - Output to file (`-o`)
  - Verbose mode (`--verbose`)
- **Animation support:**
  - Animated GIF playback with full timing control
  - Video playback with frame-accurate controls
  - Loop control: `--loop` (infinite), `--loops N` (specific count)
  - Frame selection: `--frames N` (limit), `--frame-offset N` (skip initial)
  - Time-based stopping: `-t/--duration` (e.g., "10s", "500ms")
- **Performance:**
  - Parallel image loading using Rayon
  - Configurable thread pool (`--threads N`)
  - Compression level control (`--compress`)

## Building from source

You need a recent Rust toolchain (edition 2024). Once Rust is installed:

```bash
cargo build --release
```

Run the viewer directly with:

```bash
cargo run --release -- <files>
```

Or install it into your `$CARGO_HOME/bin` for regular use:

```bash
cargo install --path .
```

### Features

The following features are enabled by default:
- `unicode`: Unicode block renderer (always recommended)
- `kitty`: Kitty graphics protocol backend
- `iterm2`: iTerm2 inline images backend
- `video`: Video playback support via ffmpeg
- `qoi`: QOI image format support
- `pdf`: PDF rendering support
- `svg`: SVG/SVGZ rendering support

**Optional features:**
- `sixel`: Enable the Sixel backend. Requires [`libsixel`](https://github.com/libsixel/libsixel). Activate with `cargo build --features sixel`.

To build with minimal features:
```bash
cargo build --no-default-features --features unicode,kitty,iterm2
```

## Using as a Library

`terminal-media` can be used as a Rust crate to add terminal image/video rendering capabilities to your own applications.

### Adding the Dependency

Add to your `Cargo.toml`:

```toml
[dependencies]
terminal-media = { git = "https://github.com/sandwichfarm/terminal-media" }
# Or when published to crates.io:
# terminal-media = "0.1"
```

### Basic Usage

```rust
use terminal_media::{Cli, Renderer};

fn main() -> terminal_media::Result<()> {
    // Parse CLI arguments and render
    let cli = Cli::parse();
    let config = cli.into_config()?;
    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

### Programmatic Configuration

Bypass CLI parsing and configure programmatically:

```rust
use std::path::PathBuf;
use terminal_media::{
    Config, Renderer, BackendKind, RenderSizing,
    config::{PixelationMode, RotationMode, BackgroundColor}
};

fn main() -> terminal_media::Result<()> {
    let mut config = Config {
        inputs: vec![PathBuf::from("photo.jpg")],
        backend: BackendKind::Auto,
        sizing: RenderSizing {
            width_cells: Some(80),
            height_cells: None,
            fit_width: true,
            ..RenderSizing::unconstrained()
        },
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Exif,
        background: BackgroundColor::Auto,
        quiet: false,
        verbose: false,
        // ... other Config fields with sensible defaults
    };

    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

### Image Loading

Load and process images directly:

```rust
use std::path::Path;
use terminal_media::image::load_image;
use terminal_media::config::RotationMode;

let sequence = load_image(
    Path::new("photo.jpg"),
    RotationMode::Exif,
    true,  // auto_crop
    10     // crop_border
)?;

println!("Loaded {} frames", sequence.frames.len());
println!("First frame: {}x{}",
    sequence.frames[0].pixels.width(),
    sequence.frames[0].pixels.height()
);
```

### Available Modules

- `config`: Configuration types and builders
- `backend`: Rendering backends (Unicode, Kitty, iTerm2, Sixel)
- `image`: Image/video loading and decoding
- `capabilities`: Terminal detection and feature probing
- `error`: Error types and Result alias

For detailed API documentation, see [`docs/library/README.md`](docs/library/README.md).

## CLI Usage

```bash
# View a single image
terminal-media photo.jpg

# Use quarter-block mode for better detail (default)
terminal-media -p quarter image.png

# Use half-block mode for better color accuracy
terminal-media -p half image.png

# View images in a grid
terminal-media --grid 3 screenshots/*.png

# View video files
terminal-media video.mp4

# Play video for 10 seconds
terminal-media -t 10s video.mp4

# Loop animation 3 times
terminal-media --loops 3 animation.gif

# Show only first 50 frames
terminal-media --frames 50 animation.gif

# Skip first 100 frames, show next 50
terminal-media --frame-offset 100 --frames 50 video.mp4

# View PDF documents (shows all pages as frames)
terminal-media document.pdf

# View SVG files
terminal-media logo.svg

# Scroll through large images
terminal-media --scroll --delta-move 5,2 large-image.jpg

# Read from stdin
cat image.jpg | terminal-media -

# Read list of images from file
terminal-media -f images.txt

# Read list with paths relative to file location
terminal-media -F /path/to/gallery/images.txt

# Combine file list with other images
terminal-media -f batch1.txt extra-image.png -f batch2.txt

# Constrain dimensions
terminal-media --width 120 --height 40 gallery/*.gif

# Fit to width (may overflow terminal height)
terminal-media --fit-width wide-panorama.jpg

# Fit to height (may overflow terminal width)
terminal-media --fit-height tall-portrait.jpg

# Upscale small images
terminal-media --upscale small-icon.png

# Integer upscaling (no antialiasing)
terminal-media -U i pixel-art.png

# View phone photos with automatic rotation
terminal-media phone-photos/*.jpg

# Disable EXIF rotation if needed
terminal-media --rotate off rotated-image.jpg

# Auto-crop screenshots to remove borders
terminal-media --auto-crop screenshot.png

# Crop fixed 10px border, then auto-crop the rest
terminal-media --crop-border 10 --auto-crop scanned-document.jpg

# Slideshow with titles
terminal-media --wait 2 --title "%n/%f" vacation/*.png

# Use alternate screen (clear on exit)
terminal-media --alternate-screen animation.gif

# Center images with custom background
terminal-media --center --background "#1e1e1e" logo.png

# Use 8-bit color mode for older terminals
terminal-media --color8 image.png

# Output to file instead of stdout
terminal-media -o output.txt image.png

# Verbose mode with terminal info
terminal-media --verbose photo.jpg

# Use specific number of threads
terminal-media --threads 4 gallery/*.jpg
```

Pass `--help` to see the complete option list.

### File Lists

File lists allow you to batch process many images without shell glob limits. Create a text file with one image path per line:

```text
# images.txt - Example file list
photo1.jpg
photo2.png
vacation/beach.jpg

# Lines starting with # are comments
# Empty lines are ignored

/absolute/path/to/image.png
relative/path/to/image.gif
```

**Usage:**
- `-f FILE`: Relative paths resolved from current directory
- `-F FILE`: Relative paths resolved from file list's directory

Example workflow:
```bash
# Create a file list
find ~/Pictures -name "*.jpg" > my-photos.txt

# View all photos
terminal-media -f my-photos.txt

# Or use -F if you move the file list
terminal-media -F ~/gallery/images.txt
```

## Testing

The project has comprehensive test coverage with 34+ library tests and integration tests covering:

- **Configuration parsing**: Backend, pixelation, and rotation mode parsing
- **File list handling**: Comment filtering, path resolution (-f vs -F)
- **EXIF rotation**: All 8 orientation transformations
- **Rendering modes**: Quarter-block and half-block pixelation
- **Sizing options**: Upscale, fit-width, dimension constraints
- **Animation controls**: Loops, frame offset, duration limits
- **Terminal detection**: Automatic backend selection for 10+ terminal emulators
- **Tmux/screen support**: DCS passthrough wrapping for multiplexers
- **Auto-crop**: Border detection, fixed cropping, asymmetric borders
- **Color quantization**: 8-bit color mode support
- **Integration tests**: Full configuration workflows

Run the test suite:
```bash
cargo test
```

Run tests with output:
```bash
cargo test -- --nocapture
```

Run specific test module:
```bash
cargo test config_tests
cargo test rendering_tests
cargo test capabilities_tests
cargo test integration_tests
```

## Documentation

Additional documentation is available:

- **[Extended Examples](docs/examples.md)** - Comprehensive tutorials and use cases for both CLI and library usage
- **[API Documentation](https://docs.rs/terminal-media)** - Full API reference (after crates.io publication)
- **[Library Examples](examples/)** - Working code examples in the `examples/` directory:
  - `basic_viewer.rs` - Simple image viewer
  - `custom_backend.rs` - Backend selection
  - `programmatic_config.rs` - Advanced configuration
  - `image_processing.rs` - Direct image processing

Run examples with:
```bash
cargo run --example basic_viewer path/to/image.jpg
```

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Development setup and build instructions
- Code style guidelines and testing requirements
- Pull request process
- Areas looking for contributions

Before submitting a pull request:

```bash
# Run tests
cargo test

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

For questions or bug reports, please [open an issue](https://github.com/sandwichfarm/terminal-media/issues) on GitHub.

## Licensing

`terminal-media` is distributed under the terms of the GPL-2.0 license. See `LICENSE` for details.
