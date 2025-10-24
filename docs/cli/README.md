# showme CLI Guide

This guide documents the `showme` executable in detail: how to install it, every command-line flag, and how the program behaves in different environments.

## 1. Installation

### Build from source

```bash
# clone the repository
git clone https://github.com/sandwichfarm/showme.git
cd showme

# build the binary with the default feature set
cargo build --release

# the executable will be written to target/release/showme
```

### Optional features

The viewer supports additional formats and backends that can be compiled in or out:

| Feature flag | Description | Extra requirements |
|--------------|-------------|--------------------|
| `unicode` (default) | Unicode block renderer (half/quarter blocks). | none. |
| `kitty` (default) | Enables the Kitty Graphics Protocol backend. | none. |
| `iterm2` (default) | Enables iTerm2 OSC 1337 inline images. | none. |
| `video` (default) | Video playback support via ffmpeg. | Requires ffmpeg libraries. |
| `qoi` (default) | QOI image format support. | none. |
| `pdf` (default) | PDF rendering support. | none. |
| `svg` (default) | SVG/SVGZ rendering support. | none. |
| `sixel` (opt-in) | Streams sixel data via `libsixel`. | Requires `libsixel` headers and library. |

Disable features with `--no-default-features`, e.g.:
```bash
cargo build --no-default-features --features unicode,kitty,iterm2
```

## 2. Usage synopsis

```
showme [OPTIONS] <PATH>...
```

- At least one input path is required.
- Supported formats: PNG, JPEG, GIF, BMP, WebP, TIFF, EXR, TGA, DDS, HDR, ICO, PNM, QOI, PDF, SVG, SVGZ, and video formats (MP4, MKV, MOV, AVI, WebM, etc.)
- Standard output must be attached to a TTY by default (override with `-o`).
- When `--backend auto` (the default) is used, the viewer inspects environment variables to pick the best backend.

## 3. Option reference

### Input options
| Flag | Description |
|------|-------------|
| `<PATH>...` | One or more files or globbed paths to render. |
| `-` | Read from standard input. |
| `-f, --filelist <FILE>` | Read paths from file (relative to current directory). |
| `-F, --filelist-from <FILE>` | Read paths from file (relative to file location). |

### Backend and rendering
| Flag | Description |
|------|-------------|
| `--backend <BACKEND>` | Force a renderer backend. Values: `auto`, `unicode`, `kitty`, `iterm2`, `sixel`. |
| `-p, --pixelation <MODE>` | Unicode block mode: `half`, `quarter` (default). |

### Sizing and scaling
| Flag | Description |
|------|-------------|
| `-w, --width <CELLS>` | Maximum width in terminal cells. |
| `-H, --height <CELLS>` | Maximum height in terminal cells. |
| `-W, --fit-width` | Fit to terminal width (may overflow height). |
| `--fit-height` | Fit to terminal height (may overflow width). |
| `-U, --upscale [MODE]` | Allow upscaling. Use `-U i` for integer scaling (pixel art). |
| `--width-stretch <FLOAT>` | Aspect ratio correction factor (default 1.0). |
| `-a, --antialias` | Enable antialiasing (Lanczos3 filter). |
| `--rotate <MODE>` | EXIF rotation mode: `exif` (default), `off`. |
| `--auto-crop` | Remove uniform borders automatically. |
| `--crop-border <PIXELS>` | Crop fixed border before auto-crop. |

### Layout
| Flag | Description |
|------|-------------|
| `--grid <COLS[xROWS]>` | Arrange images in a grid (unicode backend only). |
| `--grid-gap <CELLS>` | Horizontal spacing between columns (default 2). |
| `--center` | Center images horizontally. |
| `--scroll` | Enable scrolling animation for large images. |
| `--delta-move <X,Y>` | Scroll delta per frame (default 1,1). |

### Animation and timing
| Flag | Description |
|------|-------------|
| `--loop` | Loop animations indefinitely. |
| `--loops <N>` | Loop animations N times. |
| `-t, --duration <TIME>` | Stop after duration (e.g., "10s", "500ms"). |
| `--frames <N>` | Limit to N frames. |
| `--frame-offset <N>` | Skip first N frames. |
| `--wait <DURATION>` | Pause between images. |
| `--wait-rows <DURATION>` | Pause between grid rows. |

### Display features
| Flag | Description |
|------|-------------|
| `-q, --quiet` | Suppress headers and warnings. |
| `--verbose` | Print terminal info and statistics. |
| `--title <FORMAT>` | Title format string (tokens: %f, %b, %w, %h, %n, %%). |
| `--clear [once\|between]` | Clear screen once or between images. |
| `--alternate-screen` | Use alternate screen buffer. |
| `--hide-cursor` | Hide cursor during rendering (default true). |

### Colors and background
| Flag | Description |
|------|-------------|
| `-b, --background <COLOR>` | Background color for transparency. |
| `-B, --pattern <COLOR>` | Checkerboard pattern color. |
| `--pattern-size <INT>` | Pattern scale factor (default 1). |
| `--color8` | Use 8-bit color mode (256 colors). |

### Output and performance
| Flag | Description |
|------|-------------|
| `-o, --output <FILE>` | Write to file instead of stdout. |
| `--threads <N>` | Number of threads for parallel loading. |
| `--compress <LEVEL>` | PNG compression level (0-9, default 1). |
| `-I, --force-image` | Force image interpretation (disable video). |

## 4. Duration syntax

- Plain numbers are seconds (`1.5` → 1.5 seconds).
- Values ending in `ms` specify milliseconds (`250ms`).
- Durations must be non-negative.

## 5. Colour syntax

- Named colours: W3C HTML color names (case-insensitive).
- Hex colours: `#rrggbb` (6-digit form).
- RGB notation: `rgb(r,g,b)` with decimal or hex components (0–255).
- Special values: `auto` (sample terminal), `none` (transparent).

## 6. Title formatting

Format tokens:
- `%f` - Full file path
- `%b` - Basename only
- `%w` - Image width in pixels
- `%h` - Image height in pixels
- `%n` - Image number (1-based)
- `%%` - Literal `%`

Examples:
- `--title "%n/%w×%h %b"` → `1/1920×1080 photo.jpg`
- `--title "%f"` → full path

## 7. Behaviour notes

- **Grid mode** requires the unicode backend.
- **Centering** is ignored with `--grid`.
- **Scrolling** (`--scroll`) only works with unicode backend.
- **Background blending**: Transparent pixels are composited against the specified background color.
- **Animation playback**: Multi-frame sequences update in-place using cursor positioning.
- **PDF/SVG**: Each page/image is treated as a separate frame.
- **8-bit color mode**: When using `--color8`, RGB colors are quantized to xterm-256 palette.

## 8. Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success. |
| `1` | Any failure (I/O error, unsupported feature, decode error, etc.). |

## 9. Examples

### Basic usage

Render a single photo:
```bash
showme beach.png
```

View video for 10 seconds:
```bash
showme -t 10s video.mp4
```

View PDF document:
```bash
showme document.pdf
```

View SVG file:
```bash
showme logo.svg
```

### Grid layouts

Show screenshots in a 3×2 grid:
```bash
showme --grid 3x2 --grid-gap 1 screenshots/*.png
```

### Animation control

Loop GIF forever:
```bash
showme --loop animation.gif
```

Loop 3 times:
```bash
showme --loops 3 animation.gif
```

Show first 50 frames:
```bash
showme --frames 50 video.mp4
```

Skip first 100 frames:
```bash
showme --frame-offset 100 video.mp4
```

### Sizing and scaling

Fit to width:
```bash
showme --fit-width panorama.jpg
```

Upscale small images:
```bash
showme --upscale icon.png
```

Integer upscaling for pixel art:
```bash
showme -U i pixel-art.png
```

Constrain dimensions:
```bash
showme --width 80 --height 24 image.jpg
```

### Image processing

Auto-crop borders:
```bash
showme --auto-crop screenshot.png
```

Crop fixed border then auto-crop:
```bash
showme --crop-border 10 --auto-crop scan.jpg
```

Disable EXIF rotation:
```bash
showme --rotate off photo.jpg
```

### Display customization

Center with custom background:
```bash
showme --center --background "#1e1e1e" logo.png
```

Slideshow with titles:
```bash
showme --wait 2 --title "%n/%f" vacation/*.png
```

Use alternate screen:
```bash
showme --alternate-screen animation.gif
```

Use 8-bit color mode:
```bash
showme --color8 image.png
```

Verbose output:
```bash
showme --verbose photo.jpg
```

### File lists

Create and use file lists:
```bash
find ~/Pictures -name "*.jpg" > photos.txt
showme -f photos.txt
```

### Scrolling large images

Scroll through panorama:
```bash
showme --scroll --delta-move 5,2 large-panorama.jpg
```

### Output to file

Save output to file:
```bash
showme -o output.txt image.png
```

### Performance tuning

Use specific thread count:
```bash
showme --threads 4 gallery/*.jpg
```

Higher compression:
```bash
showme --compress 9 --backend kitty image.png
```

With these options you can customize showme for any workflow.
