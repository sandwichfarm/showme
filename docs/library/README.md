# showme Library Guide

This document explains how the `showme` crate is structured, how to embed it in other Rust applications, and the responsibilities of the major modules.

## 1. Getting started

Add the crate to a Cargo project directly from the local checkout:

```toml
[dependencies]
showme = { path = "../showme", features = ["kitty", "iterm2"] }
# Or from git:
# showme = { git = "https://github.com/sandwichfarm/showme", features = ["kitty", "iterm2"] }
```

Feature flags:

| Feature | Default? | Purpose |
|---------|----------|---------|
| `unicode` | ✅ | Enables the unicode block renderer (half/quarter). Required for grid output. |
| `kitty` | ✅ | Enables the Kitty Graphics Protocol backend. |
| `iterm2` | ✅ | Enables the iTerm2 OSC 1337 backend. |
| `video` | ✅ | Enables video playback via ffmpeg. |
| `qoi` | ✅ | Enables QOI image format support. |
| `pdf` | ✅ | Enables PDF rendering support. |
| `svg` | ✅ | Enables SVG/SVGZ rendering support. |
| `sixel` | ⛔️ | Optional Sixel backend implemented via `libsixel`. Requires the native library. |

Disable default features with `--no-default-features` or by enumerating only the features you want.

## 2. High-level architecture

The crate is split into cooperating modules:

- `cli`: Clap-based argument parser. Produces a `Config` struct that the renderer understands.
- `config`: Types describing user intent (backend, sizing, grid layout, timing, transparency handling).
- `renderer`: Orchestrates backend selection, terminal capability probing, image loading (using Rayon for parallelism), and output pacing.
- `backend`: Trait (`Backend`) plus concrete implementations (`UnicodeBackend`, `KittyBackend`, `ITerm2Backend`, optional `SixelBackend`). Helpers for chunking, scaling, and background blending.
- `image`: Loader supporting multiple formats (static images, GIF, QOI, PDF, SVG, video). Produces `ImageSequence` values containing decoded frames and playback delays.
- `video`: Video decoder using ffmpeg (feature-gated).
- `capabilities`: Terminal detection and verification (TTY checks, window size lookup, backend guessing).
- `autocrop`: Border detection and auto-cropping logic.
- `color_quantize`: 8-bit color quantization to xterm-256 palette.
- `tmux`: Terminal multiplexer detection and DCS passthrough wrapping.
- `error`: Shared `RimgError` enum with context-rich variants for reporting to users.

The typical flow is `Cli::parse() → Cli::into_config() → Renderer::build(config) → Renderer::run()`.

## 3. Example: embedding the renderer

```rust
use showme::{Cli, Renderer};

fn main() -> showme::Result<()> {
    // When you already have CLI arguments you can reuse the clap struct.
    let cli = Cli::parse();
    let config = cli.into_config()?;
    let renderer = Renderer::build(config)?;
    renderer.run()
}
```

When integrating into a larger app you can bypass `Cli` entirely:

```rust
use std::num::NonZeroUsize;
use showme::{BackendKind, Config, GridOptions, RenderSizing, Renderer};

let config = Config {
    inputs: vec!["demo.png".into(), "demo.gif".into()],
    backend: BackendKind::Unicode,
    sizing: RenderSizing {
        width_cells: Some(80),
        height_cells: None,
        fit_width: false,
        fit_height: false,
        upscale: false,
        upscale_integer: false,
        width_stretch: 1.0,
        antialias: true,
    },
    pixelation: showme::config::PixelationMode::Quarter,
    rotation: showme::config::RotationMode::Exif,
    auto_crop: false,
    crop_border: 0,
    grid: Some(GridOptions {
        columns: NonZeroUsize::new(2).unwrap(),
        rows: None,
        spacing: 1
    }),
    loop_forever: false,
    loops: None,
    max_frames: None,
    frame_offset: 0,
    max_duration: None,
    quiet: false,
    verbose: false,
    clear_between: false,
    clear_once: true,
    wait_between_images: None,
    wait_between_rows: None,
    title_format: Some("%n/%w×%h %b".into()),
    center: false,
    alternate_screen: false,
    hide_cursor: true,
    background: showme::config::BackgroundColor::Auto,
    pattern_color: None,
    pattern_size: 1,
    use_8bit_color: false,
    output_file: None,
    threads: None,
    compress_level: 1,
    force_video: false,
    force_image: false,
    scroll_animation: false,
    scroll_delay: std::time::Duration::from_millis(50),
    scroll_dx: 1,
    scroll_dy: 1,
};

Renderer::build(config)?.run()?;
```

> **TTY requirement**: `Renderer::build` calls `ensure_tty_stdout()`. If your application wants to render into an off-screen buffer, you must provide an alternative backend implementation that skips this guard.

## 4. Module details

### 4.1 `cli`

- `Cli` derives `clap::Parser`, exposing each CLI flag as a field.
- `Cli::into_config` performs additional parsing (durations, colors, grid spec, file lists). It validates inputs (`pattern-size > 0`, at least one path, etc.) and returns a `Config` or `RimgError`.
- Duration parsing accepts seconds (default unit) or `ms` suffix. Colors accept HTML names, `#rrggbb`, or `rgb()` with decimal / hex components.
- File list parsing supports `-f` (relative to CWD) and `-F` (relative to file location).

### 4.2 `config`

Key types:

- `BackendKind`: `Auto`, `Unicode`, `Kitty`, `Iterm2`, `Sixel`. `Auto` uses terminal detection heuristics.
- `PixelationMode`: `Half`, `Quarter` (default). Controls Unicode block rendering resolution.
- `RotationMode`: `Exif`, `Off`. Controls EXIF-based auto-rotation.
- `RenderSizing`: Width/height limits in cells, fit modes, upscaling, width stretch correction, antialiasing.
- `GridOptions`: Number of columns (required), optional maximum rows, and column spacing.
- `Config`: Canonical view of user intent consumed by the renderer.
- `BackgroundColor`, `RgbColor`: Support alpha blending and checkerboard patterns. `BackgroundColor::Auto` encourages backends to maintain transparency; `Color` triggers composition.

### 4.3 `renderer`

Responsibilities:

- Picks a backend (respecting feature availability) and falls back to unicode if the requested backend isn't compiled in.
- Records the terminal size at startup. Unicode and grid rendering rely on this to scale correctly.
- Loads images in parallel with Rayon. Thread pool can be configured via `Config::threads`.
- `image::load_image` handles multiple formats and returns `ImageSequence` with decoded frames.
- Handles playback pacing: `--loop`, `--loops`, `--wait`, frame delays, duration limits.
- Provides title formatting (`%f`, `%b`, `%w`, `%h`, `%n`, `%%`).
- Grid rendering is restricted to the unicode backend.
- Scrolling animation support for large images (unicode backend only).
- Centering logic calculates indentation based on rendered width vs. terminal width.
- Cursor hiding/showing via RAII guard pattern.
- Alternate screen buffer support via RAII guard pattern.

### 4.4 `backend`

- `Backend` trait has a single `render(&Frame, RenderOptions)` method returning `RenderedFrame`.
- `RenderOptions` bundles sizing, terminal data, background style, pixelation mode, 8-bit color flag, and compression level.
- `BackgroundStyle` is computed from the config: optional solid color plus optional checkerboard color/size.
- `UnicodeBackend`: Rasterizes using half-block or quarter-block characters with 24-bit or 8-bit ANSI colors. Supports checkerboard transparency.
- `KittyBackend`: PNG-encodes the frame, emits Kitty escape sequences in base64 chunks with DCS passthrough for tmux.
- `ITerm2Backend`: Uses OSC 1337 protocol, streaming base64 PNG chunks with inline metadata.
- `SixelBackend` *(feature-gated)*: Uses `libsixel` to encode frames with dithering.
- `chunk_util`: Encodes data once and provides reusable base64 chunks to reduce allocations.
- `image_util`: Handles resizing (Lanczos3/Nearest filter), background blending, PNG encoding.

### 4.5 `image`

- Uses `image::ImageReader` for format detection.
- Supported formats: PNG, JPEG, GIF, BMP, WebP, TIFF, EXR, TGA, DDS, HDR, ICO, PNM, QOI, PDF, SVG, SVGZ.
- Video support via `ffmpeg-next` crate (feature-gated).
- Animated GIFs use the `gif` decoder and convert frame delays to `Duration`.
- PDF rendering via `pdfium-render` crate (feature-gated). Each page becomes a frame.
- SVG rendering via `resvg` crate (feature-gated). Rasterizes vector graphics to RGBA.
- EXIF orientation detection and application.
- Auto-crop and fixed border cropping support.
- Returns `ImageSequence { path, frames }` where each `Frame` contains RGBA pixels and playback delay.

### 4.6 `video`

- Feature-gated video decoder using ffmpeg.
- Probes video metadata (codec, dimensions, duration, frame rate).
- Decodes video frames on-demand.
- Returns frames as `ImageSequence` for consistent rendering pipeline.

### 4.7 `capabilities`

- `ensure_tty_stdout()` → error if STDOUT isn't a TTY.
- `current_terminal_size()` → query terminal dimensions, fallback to 80×24.
- `detect_terminal_backend()` → heuristics for Kitty (`KITTY_WINDOW_ID`), iTerm2 (`TERM_PROGRAM`), and Sixel (`TERM`). Defaults to unicode.
- `is_in_multiplexer()` → detects tmux/screen from environment variables.

### 4.8 `autocrop`

- `detect_background_color()` → samples corner pixels to determine uniform background.
- `crop_border()` → removes fixed-width border from all sides.
- `auto_crop()` → detects and removes uniform background borders.
- Supports asymmetric border detection.

### 4.9 `color_quantize`

- `rgb_to_256()` → quantizes 24-bit RGB to xterm-256 color palette.
- Handles grayscale mapping (colors 232-255).
- Maps RGB to 6×6×6 color cube (colors 16-231).
- Used when `--color8` flag is enabled.

### 4.10 `tmux`

- `in_tmux()` → detects tmux from `TMUX` environment variable.
- `in_multiplexer()` → detects tmux or screen.
- `wrap_for_tmux()` → wraps escape sequences in DCS passthrough for graphics protocols.
- `enable_tmux_passthrough()` → automatically enables `allow-passthrough` in tmux >= 3.3.

### 4.11 `error`

`RimgError` includes variants for missing input, I/O, image decode failures, frame decode errors, and generic `Other(String)` for user-facing diagnostics. The `Result<T>` alias streamlines error propagation across modules.

## 5. Background handling

- When `BackgroundColor::Auto`, unicode output shows a checkerboard (if `pattern_color` is set) but preserves original RGBA values for protocol backends.
- `BackgroundColor::None` leaves transparent pixels untouched.
- `pattern_color` + `pattern_size` define the alternating squares used to visualize alpha (unicode backend only).
- Background blending is applied during rendering in `image_util::blend_transparency()`.

## 6. Performance considerations

- **Parallel loading**: Images are loaded in parallel using Rayon. Configure thread count with `Config::threads`.
- **Compression level**: PNG compression for Kitty/iTerm2 backends can be adjusted (0-9) via `Config::compress_level`.
- **Antialiasing**: Lanczos3 filter for high-quality scaling, or Nearest for pixel-perfect integer upscaling.
- **8-bit color mode**: Reduces output size by using xterm-256 palette instead of 24-bit RGB.

## 7. Testing

- `cargo test` exercises parsing helpers, chunk utilities, backend smoke tests, color quantization, auto-crop logic, and tmux wrapping.
- `tests/` directory contains integration tests for configuration parsing, rendering workflows, and capabilities detection.
- 34+ library tests ensure code quality and feature coverage.
- When adding new public APIs or behavior, please extend these tests or add integration tests under `tests/`.

## 8. Extending the library

### Adding a new backend

1. Implement the `Backend` trait in `src/backend/`
2. Add feature flag to `Cargo.toml`
3. Update `BackendFactory::build()` to instantiate your backend
4. Add terminal detection logic to `capabilities.rs` if needed
5. Write unit tests for your backend

### Adding a new image format

1. Add decoder dependency to `Cargo.toml` with optional feature
2. Update `load_image()` in `src/image.rs` to detect file extension
3. Implement `load_<format>()` function following existing patterns
4. Apply rotation and cropping transformations
5. Return `ImageSequence` with decoded frames

Armed with the above understanding you can treat `showme` as a library component—construct configs programmatically, feed it data, and reuse individual modules (e.g. the unicode backend) inside your own rendering pipelines.
