# Extended Examples and Tutorials

This guide provides comprehensive examples for using showme both as a CLI tool and as a library in your Rust projects.

## Table of Contents

- [CLI Examples](#cli-examples)
  - [Basic Image Viewing](#basic-image-viewing)
  - [Working with Videos](#working-with-videos)
  - [Grid Layouts](#grid-layouts)
  - [Slideshows](#slideshows)
  - [Image Processing](#image-processing)
- [Library Examples](#library-examples)
  - [Simple Integration](#simple-integration)
  - [Custom Configuration](#custom-configuration)
  - [Backend Selection](#backend-selection)
  - [Direct Image Processing](#direct-image-processing)
  - [Advanced Rendering](#advanced-rendering)
- [Use Cases](#use-cases)
  - [Terminal File Manager](#terminal-file-manager)
  - [Image Gallery Browser](#image-gallery-browser)
  - [Documentation Screenshots](#documentation-screenshots)

## CLI Examples

### Basic Image Viewing

**View a single image:**
```bash
showme photo.jpg
```

**View with specific size constraints:**
```bash
# Constrain to 80 columns wide, 40 rows tall
showme -w 80 -H 40 image.png

# Fit to terminal width (may overflow height)
showme --fit-width panorama.jpg

# Fit to terminal height (may overflow width)
showme --fit-height portrait.jpg
```

**Center image with custom background:**
```bash
showme --center --background "#1e1e1e" logo.png
```

**Use alternate screen (clears on exit):**
```bash
showme --alternate-screen presentation.png
```

### Working with Videos

**Play a video:**
```bash
showme video.mp4
```

**Play for specific duration:**
```bash
# Play for 10 seconds
showme -t 10s video.mp4

# Play for 500 milliseconds
showme -t 500ms clip.mp4
```

**Control frame playback:**
```bash
# Show first 100 frames
showme --frames 100 video.mp4

# Skip first 50 frames, show next 100
showme --frame-offset 50 --frames 100 video.mp4
```

**Loop control:**
```bash
# Loop 3 times
showme --loops 3 animation.gif

# Loop forever
showme --loop animation.gif
```

### Grid Layouts

**Simple grid:**
```bash
# 3 columns
showme --grid 3 photos/*.jpg

# 2x3 grid (2 rows, 3 columns)
showme --grid 2x3 photos/*.jpg
```

**Grid with spacing:**
```bash
# Grid with custom spacing between images
showme --grid 3 --pattern-size 2 photos/*.jpg
```

### Slideshows

**Automatic slideshow:**
```bash
# Wait 2 seconds between images
showme --wait 2 vacation/*.png

# Progressive rendering with row delay
showme --wait-rows 0.01 animation.gif
```

**Slideshow with titles:**
```bash
# Show filename and dimensions
showme --wait 2 --title "%f - %wx%h" photos/*.jpg

# Show image number out of total
showme --wait 2 --title "Image %n of %N" photos/*.jpg
```

**Title format variables:**
- `%f` - Filename
- `%w` - Width in pixels
- `%h` - Height in pixels
- `%n` - Current image number
- `%N` - Total number of images

### Image Processing

**EXIF rotation:**
```bash
# Auto-rotate based on EXIF data (default)
showme phone-photos/*.jpg

# Disable rotation
showme --rotate off rotated.jpg
```

**Cropping:**
```bash
# Auto-crop uniform borders
showme --auto-crop screenshot.png

# Remove 10px border, then auto-crop
showme --crop-border 10 --auto-crop scan.jpg

# Just remove 10px border
showme --crop-border 10 photo.jpg
```

**Upscaling:**
```bash
# Upscale small images
showme --upscale icon.png

# Integer upscaling (good for pixel art)
showme -U i pixel-art.png
```

**Backend selection:**
```bash
# Auto-detect best backend (default)
showme photo.jpg

# Force specific backend
showme --backend kitty photo.jpg
showme --backend iterm2 photo.jpg
showme --backend unicode photo.jpg
showme --backend sixel photo.jpg  # requires --features sixel
```

**Rendering modes:**
```bash
# Quarter-block (default, best detail)
showme -p quarter photo.jpg

# Half-block (better color accuracy)
showme -p half photo.jpg
```

**Color modes:**
```bash
# 24-bit color (default)
showme photo.jpg

# 8-bit color for older terminals
showme --color8 photo.jpg
```

### Advanced CLI Usage

**Batch processing with file lists:**
```bash
# Create file list
find ~/Pictures -name "*.jpg" > photos.txt

# Process from file list
showme -f photos.txt

# Relative paths from file location
showme -F /path/to/gallery/images.txt
```

**Scrolling through large images:**
```bash
# Scroll with default delta (1x1)
showme --scroll large-map.png

# Scroll with custom movement speed
showme --scroll --delta-move 5,2 image.png

# Scroll with custom delay
showme --scroll --scroll-delay 100 image.png
```

**Output to file:**
```bash
# Save rendered output to file
showme -o output.txt image.png

# Process and save in pipeline
showme image.png -o rendered.ans
```

**Performance tuning:**
```bash
# Use 8 threads for parallel loading
showme --threads 8 gallery/*.jpg

# Adjust compression level (1-9, higher = more compression)
showme --compress 4 image.png
```

## Library Examples

### Simple Integration

The simplest way to use showme as a library:

```rust
use terminal_media::{Cli, Renderer};
use clap::Parser;

fn main() -> terminal_media::Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Convert to config
    let config = cli.into_config()?;

    // Build and run renderer
    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

### Custom Configuration

Build configuration programmatically without CLI parsing:

```rust
use std::path::PathBuf;
use std::time::Duration;
use terminal_media::{Config, Renderer, BackendKind, RenderSizing};
use terminal_media::config::{PixelationMode, RotationMode, BackgroundColor};

fn display_image(path: &str) -> terminal_media::Result<()> {
    let config = Config {
        inputs: vec![PathBuf::from(path)],

        // Backend and rendering
        backend: BackendKind::Auto,
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Exif,
        background: BackgroundColor::Auto,

        // Sizing
        sizing: RenderSizing {
            width_cells: Some(100),
            height_cells: Some(40),
            fit_width: false,
            fit_height: false,
            upscale: true,
            upscale_integer: false,
            antialias: true,
            width_stretch: 2.0, // Auto-detected by CLI, manual for library (2.0 = typical terminal)
        },

        // Display options
        center: true,
        alternate_screen: false,
        hide_cursor: true,

        // Animation
        loop_forever: false,
        loops: Some(1),
        max_frames: None,
        frame_offset: 0,
        max_duration: None,
        wait_between_images: None,
        wait_between_rows: None,

        // Other options with defaults
        quiet: false,
        verbose: false,
        clear_once: false,
        clear_between: false,
        title_format: None,
        pattern_color: None,
        pattern_size: 1,
        grid: None,
        auto_crop: false,
        crop_border: 0,
        use_8bit_color: false,
        output_file: None,
        threads: None,
        compress_level: 1,
        force_video: false,
        force_image: false,
        scroll_animation: false,
        scroll_delay: Duration::from_millis(50),
        scroll_dx: 1,
        scroll_dy: 1,
    };

    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

### Backend Selection

Choose specific rendering backends:

```rust
use terminal_media::{Config, Renderer, BackendKind};

fn render_with_backend(path: &str, backend: BackendKind) -> terminal_media::Result<()> {
    let mut config = Config::default();
    config.inputs = vec![PathBuf::from(path)];
    config.backend = backend;
    config.verbose = true;  // Show backend info

    let renderer = Renderer::build(config)?;
    renderer.run()
}

// Usage
render_with_backend("photo.jpg", BackendKind::Kitty)?;
render_with_backend("photo.jpg", BackendKind::Iterm2)?;
render_with_backend("photo.jpg", BackendKind::Unicode)?;
```

### Direct Image Processing

Load and process images without the renderer:

```rust
use std::path::Path;
use terminal_media::image::load_image;
use terminal_media::config::RotationMode;

fn analyze_image(path: &str) -> terminal_media::Result<()> {
    let sequence = load_image(
        Path::new(path),
        RotationMode::Exif,
        false,  // auto_crop
        0       // crop_border
    )?;

    println!("Loaded: {}", sequence.path.display());
    println!("Frames: {}", sequence.frames.len());

    if let Some(frame) = sequence.first_frame() {
        let img = &frame.pixels;
        println!("Dimensions: {}x{}", img.width(), img.height());
        println!("Delay: {:?}", frame.delay);

        // Access pixel data
        let pixel = img.get_pixel(0, 0);
        println!("Top-left pixel: RGB({}, {}, {})",
            pixel[0], pixel[1], pixel[2]);

        // Check for transparency
        let has_alpha = img.pixels().any(|p| p[3] < 255);
        println!("Has transparency: {}", has_alpha);
    }

    Ok(())
}
```

### Advanced Rendering

Custom rendering with progress callbacks:

```rust
use std::io::{stdout, Write};
use terminal_media::{Config, Renderer};
use terminal_media::config::PixelationMode;

fn render_with_progress(images: Vec<PathBuf>) -> terminal_media::Result<()> {
    let mut config = Config::default();
    config.inputs = images;
    config.pixelation = PixelationMode::Quarter;

    println!("Loading {} images...", config.inputs.len());

    let renderer = Renderer::build(config)?;

    // Render to stdout
    let mut out = stdout();
    renderer.run()
}
```

## Use Cases

### Terminal File Manager

Integrate image preview in a file manager:

```rust
use std::path::Path;
use terminal_media::{Config, Renderer, BackendKind, RenderSizing};

fn preview_file(path: &Path, max_width: u16, max_height: u16) -> terminal_media::Result<()> {
    // Quick preview with constraints
    let config = Config {
        inputs: vec![path.to_path_buf()],
        backend: BackendKind::Auto,
        sizing: RenderSizing {
            width_cells: Some(max_width),
            height_cells: Some(max_height),
            fit_width: true,
            ..RenderSizing::unconstrained()
        },
        quiet: true,
        hide_cursor: true,
        max_frames: Some(1),  // Only first frame for preview
        ..Config::default()
    };

    Renderer::build(config)?.run()
}
```

### Image Gallery Browser

Build a terminal image gallery:

```rust
use std::path::PathBuf;
use std::time::Duration;
use terminal_media::{Config, Renderer};

fn gallery_slideshow(images: Vec<PathBuf>, delay: u64) -> terminal_media::Result<()> {
    let config = Config {
        inputs: images,
        alternate_screen: true,
        center: true,
        wait_between_images: Some(Duration::from_secs(delay)),
        title_format: Some("Image %n/%N - %f".to_string()),
        loop_forever: true,
        clear_between: true,
        hide_cursor: true,
        ..Config::default()
    };

    Renderer::build(config)?.run()
}
```

### Documentation Screenshots

Process screenshots for documentation:

```rust
use std::path::PathBuf;
use terminal_media::{Config, Renderer};

fn process_screenshot(input: &str, output: &str) -> terminal_media::Result<()> {
    let config = Config {
        inputs: vec![PathBuf::from(input)],
        auto_crop: true,        // Remove borders
        crop_border: 5,          // Remove 5px border first
        output_file: Some(PathBuf::from(output)),
        quiet: true,
        ..Config::default()
    };

    Renderer::build(config)?.run()
}
```

## Tips and Best Practices

### Performance

- Use `--threads` to control parallelism for large batches
- Set `max_frames` when only previewing animations/videos
- Use `quiet: true` in library mode to suppress output
- Consider `compress_level` for large images in graphics protocols

### Terminal Compatibility

- Use `BackendKind::Auto` for automatic detection
- Test with `--backend unicode` for maximum compatibility
- Check terminal support with `--verbose`
- Be aware of multiplexer limitations (tmux/screen)

### Image Quality

- Use `PixelationMode::Quarter` for best spatial detail
- Use `PixelationMode::Half` for better color accuracy
- Enable `antialias: true` for smoother scaling
- Consider `upscale_integer` for pixel art

### Error Handling

```rust
use terminal_media::{Config, Renderer};

fn safe_render(config: Config) {
    match Renderer::build(config) {
        Ok(renderer) => {
            if let Err(e) = renderer.run() {
                eprintln!("Rendering failed: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to build renderer: {}", e);
        }
    }
}
```

## Further Reading

- [README.md](../README.md) - Main documentation
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Development guide
- [examples/](../examples/) - Complete example programs
- API Documentation - Run `cargo doc --open`
