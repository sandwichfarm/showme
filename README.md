# showme

[![Crates.io](https://img.shields.io/crates/v/showme.svg)](https://crates.io/crates/showme)
[![Documentation](https://docs.rs/showme/badge.svg)](https://docs.rs/showme)
[![License: GPL-2.0](https://img.shields.io/badge/License-GPL--2.0-blue.svg)](https://github.com/sandwichfarm/showme/blob/master/LICENSE)
[![Build Status](https://github.com/sandwichfarm/showme/workflows/Release/badge.svg)](https://github.com/sandwichfarm/showme/actions)
[![Rust Version](https://img.shields.io/badge/rust-1.92%2B-orange.svg)](https://www.rust-lang.org)

`showme` is a powerful terminal image and video viewer written in Rust. View images, videos, PDFs, and SVGs directly in your terminal without leaving your workflow.

## Table of Contents

- [Platform Compatibility](#platform-compatibility)
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

## Platform Compatibility

The release workflows build these artifact feature sets. Historical published releases may differ; consult their release notes:

| Platform | Video Support | Sixel | All Other Features* |
|----------|---------------|-------|---------------------|
| **Linux x86_64** | ✅ Yes | ❌ No | ✅ Yes |
| **Linux ARM64** | ❌ No† | ❌ No | ✅ Yes |
| **macOS x86_64 (Intel)** | ✅ Yes | ❌ No | ✅ Yes |
| **macOS ARM64 (Apple Silicon)** | ✅ Yes | ❌ No | ✅ Yes |
| **Windows x86_64** | ❌ No‡ | ❌ No | ✅ Yes |

\* All other features include: Unicode rendering, Kitty graphics, iTerm2 images, QOI format, PDF rendering, and SVG support
† Linux ARM64: These archives omit video. Build from source with FFmpeg for video support.
‡ Windows: Complex ffmpeg setup prevents video support in pre-built binaries. Build from source with vcpkg for video.

**Building from source** allows you to enable all features on any platform if dependencies are available. See [Building from Source](#building-from-source).

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
  - Upscaling support (`-U/--upscale on`, `-U i` for integer scaling)
  - Width stretch correction (`--width-stretch`)
  - EXIF orientation support (automatic rotation for phone photos)
  - Auto-crop to remove uniform borders (`--auto-crop`)
  - Fixed border cropping (`--crop-border N`)
  - Antialiasing control (`-a/--no-antialias`)
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
  - Decode and render inputs in order, releasing media before the next input
  - Stop decoding bounded animation/video previews early
  - Configurable Rayon decoder workers (`--threads N`)
  - Compression level control (`--compress`)

## Building from source

The declared minimum Rust version is **1.92**. Development formatting and linting use the pinned **1.98.1** toolchain in `rust-toolchain.toml`. Once Rust and the native libraries below are installed:

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
- `sixel`: Enable the Sixel backend. Builds bundled static [`libsixel`](https://github.com/libsixel/libsixel) using Autotools. Activate with `cargo build --features sixel`.

To build with minimal features:
```bash
cargo build --no-default-features --features unicode,kitty,iterm2
```

### Native libraries and runtime setup

Video builds require FFmpeg development libraries and `pkg-config`, plus Clang
for generated bindings. FFmpeg 8 is verified on the local macOS development
host. The `ffmpeg-next` 9 wrapper uses only demuxing, decoding, and scaling;
installing an `ffmpeg` executable alone does not supply headers or link libraries.

Sixel builds use `sixel-sys` 0.5's bundled static libsixel. Install Autoconf,
Automake, libtool, make, and a C compiler (`brew install autoconf automake libtool`
on macOS). `.cargo/config.toml` disables Autoconf's C23 probe because the bundled
generated C code uses older function definitions. A system libsixel installation
is not used to satisfy this bundled build.

PDF support loads **PDFium 7881** at runtime, matching the explicit
`pdfium_7881` binding feature. Fetch the reviewed library and its licenses with:

```sh
# Choose linux-x64, linux-arm64, mac-x64, mac-arm64, or win-x64.
python3 scripts/fetch_pdfium.py --platform mac-arm64 --directory .native/pdfium
SHOWME_PDFIUM_DIR="$PWD/.native/pdfium" cargo run --locked -- document.pdf
```

The fetcher verifies the pinned SHA-256 before extracting. `SHOWME_PDFIUM_PATH`
selects an exact library file; `SHOWME_PDFIUM_DIR` selects a directory containing
`libpdfium.so`, `libpdfium.dylib`, or `pdfium.dll`. Discovery also checks beside
the executable and its `lib` directory, the working directory, common native
library locations, and the system loader path. Use the matching CPU architecture
and ABI. Compiling the PDF feature does not install the runtime library.

### Output, playback, and decode limits

`-o FILE` works without a terminal. Automatic backend selection chooses Unicode
for files; output contains its ANSI color sequences. The default file geometry
is 80×24 cells, with `--width`, `--height`, or `--geometry` overrides. File output
skips terminal probing, cursor hiding, alternate-screen setup, and tmux setup on
stdout. Ordinary stdout rendering requires a terminal. Grid and scrolling layouts
currently require the Unicode backend. Explicitly requesting a backend that was
not compiled in reports an error; automatic selection can fall back to Unicode.

`--force-image` bypasses FFmpeg probing, including for misnamed image files.
`--force-video` selects FFmpeg for a file regardless of its extension and requires
a build with video support. Video stdin is unsupported; `-` accepts image data.
Fixed-border cropping runs before auto-crop for both file and stdin images/GIFs.
Ctrl-C cancels playback delays and pending Unix stdin reads, restores terminal
state, and exits with status 130.

`--frames 0`, `--loops 0`, and a zero duration produce no frames. A frame offset
beyond the decoded sequence produces no sequential playback. Frame selection
applies per playback loop; scrolling uses `--frames` as a step limit per loop.
Duration limits cover playback and cap frame/scroll sleeps. Single static images
are displayed once; loop controls apply to animation and scrolling.

Image/stdin input and decompressed SVGZ data are limited to 256 MiB, dimensions to 16384 per side and 64
megapixels, retained RGBA media to 512 MiB and 1024 frames, and PDF decoding to 128
pages. Decoder and protocol working buffers are additional. Streaming video input has no
256 MiB file-size cap. Use `--frames` or `--duration` for short previews of longer
media; frame offsets count toward the retained decode budget. Inputs are decoded
one at a time, and grids retain rendered output for one row. Requests exceeding
the limits fail with an explicit error.

## Using as a Library

`showme` can be used as a Rust crate to add terminal image/video rendering capabilities to your own applications.

### Adding the Dependency

These examples describe the current checkout's API. Add the local checkout to
your `Cargo.toml` (`clap` is needed by examples that call `Cli::parse()`):

```toml
[dependencies]
showme = { path = "../showme" }
clap = "4"
```

### Basic Usage

```rust,no_run
use clap::Parser;
use showme::{Cli, Renderer};

fn main() -> showme::Result<()> {
    // Parse CLI arguments and render
    let cli = Cli::parse();
    let config = cli.into_config()?;
    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

### Programmatic Configuration

Bypass CLI parsing and configure programmatically:

```rust,no_run
use std::path::PathBuf;
use showme::{BackendKind, Config, RenderSizing, Renderer};

fn main() -> showme::Result<()> {
    let config = Config {
        inputs: vec![PathBuf::from("photo.jpg")],
        backend: BackendKind::Unicode,
        sizing: RenderSizing {
            width_cells: Some(80),
            fit_width: true,
            ..RenderSizing::unconstrained()
        },
        ..Config::default()
    };
    Renderer::build(config)?.run()
}
```

### Image Loading

Load and process images directly:

```rust,no_run
use std::path::Path;
use showme::image::load_image;
use showme::config::RotationMode;

fn main() -> showme::Result<()> {
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
    Ok(())
}
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
showme photo.jpg

# Use quarter-block mode for better detail (default)
showme -p quarter image.png

# Use half-block mode for better color accuracy
showme -p half image.png

# View images in a grid
showme --grid 3 screenshots/*.png

# View video files
showme video.mp4

# Play video for 10 seconds
showme -t 10s video.mp4

# Loop animation 3 times
showme --loops 3 animation.gif

# Show only first 50 frames
showme --frames 50 animation.gif

# Skip first 100 frames, show next 50
showme --frame-offset 100 --frames 50 video.mp4

# View PDF documents (shows all pages as frames)
showme document.pdf

# View SVG files
showme logo.svg

# Scroll through large images
showme --scroll --delta-move 5:2 large-image.jpg

# Read from stdin
cat image.jpg | showme -

# Read list of images from file
showme -f images.txt

# Read list with paths relative to file location
showme -F /path/to/gallery/images.txt

# Combine file list with other images
showme -f batch1.txt extra-image.png -f batch2.txt

# Constrain dimensions
showme --width 120 --height 40 gallery/*.gif

# Fit to width (may overflow terminal height)
showme --fit-width wide-panorama.jpg

# Fit to height (may overflow terminal width)
showme --fit-height tall-portrait.jpg

# Upscale small images
showme --upscale on small-icon.png

# Integer upscaling (no antialiasing)
showme -U i -a pixel-art.png

# View phone photos with automatic rotation
showme phone-photos/*.jpg

# Disable EXIF rotation if needed
showme --rotate off rotated-image.jpg

# Auto-crop screenshots to remove borders
showme --auto-crop screenshot.png

# Crop fixed 10px border, then auto-crop the rest
showme --crop-border 10 --auto-crop scanned-document.jpg

# Slideshow with titles
showme --wait 2 --title "%n/%f" vacation/*.png

# Use alternate screen (clear on exit)
showme --alternate-screen animation.gif

# Center images with custom background
showme --center --background "#1e1e1e" logo.png

# Use 8-bit color mode for older terminals
showme --color8 image.png

# Output to file instead of stdout
showme -o output.txt image.png

# Verbose mode with terminal info
showme --verbose photo.jpg

# Use specific number of threads
showme --threads 4 gallery/*.jpg
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
showme -f my-photos.txt

# Or use -F if you move the file list
showme -F ~/gallery/images.txt
```

## Testing

The suite exercises configuration, terminal backend selection, rendering output,
frame ordering and timing limits, grid spacing, cleanup on errors, and media
pixels and dimensions. CLI tests run the actual executable with captured output
and bounded timeouts. Small generated fixtures cover PNG, JPEG, GIF, QOI, SVG,
PDF, and video; optional-feature tests follow the selected build features.

The 2026-10-21 macOS all-feature run measured 82.94% line coverage: renderer
87.38%, image loading 81.11%, and video 86.42%, compared with the historical
52.63% overall baseline. This includes in-source test bodies and excludes
doctests and native dependency internals; branch coverage was not measured.
CI checks overall and renderer/media coverage independently. See
[TODO.md](TODO.md) for verification evidence and platform checks. Real
terminal compatibility still requires the terminal smoke checks; encoded payload
and PTY tests cannot establish how every terminal displays an image.

PDF decoding tests are explicitly ignored when no compatible native runtime is
provisioned. To include them after fetching PDFium 7881:

```sh
SHOWME_PDFIUM_DIR="$PWD/.native/pdfium" cargo test --locked --all-features -- --include-ignored
```

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
cargo test --locked --test config_tests
cargo test --locked --test rendering_tests
cargo test --locked --test capabilities_tests
cargo test --locked --test integration_tests
```

## Documentation

Additional documentation is available:

- **[Extended Examples](docs/examples.md)** - Comprehensive tutorials and use cases for both CLI and library usage
- **[API Documentation](https://docs.rs/showme)** - Full API reference (after crates.io publication)
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

For questions or bug reports, please [open an issue](https://github.com/sandwichfarm/showme/issues) on GitHub.

## Licensing

`showme` is distributed under the terms of the GPL-2.0 license. See `LICENSE` for details.


For byte-level protocol and lifecycle checks, see
[the terminal smoke checklist](docs/terminal-smoke.md). Kitty transfers use
bounded APC chunks; iTerm2 uses complete `File` commands and, above the transport
limit, the documented `MultipartFile`/`FilePart`/`FileEnd` protocol (iTerm2 3.5+
or another terminal implementing that extension). Large multipart transfers in
WezTerm and actual terminal display still need the native smoke checks. GNU
screen uses its own bounded DCS passthrough instead of tmux's escaping.
