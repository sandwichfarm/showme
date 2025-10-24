use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::OnceLock;

use clap::{ArgAction, Parser};

use crate::config::{BackendKind, BackgroundColor, Config, GridOptions, PixelationMode, RenderSizing, RotationMode, RgbColor};
use crate::error::{Result, RimgError};

#[derive(Debug, Parser)]
#[command(author, version, about = "Terminal image viewer rewritten in Rust", long_about = None)]
pub struct Cli {
    /// Image or animated media files to show (use "-" for stdin)
    #[arg(value_name = "PATH", num_args = 0..)]
    pub inputs: Vec<PathBuf>,

    /// Read list of images from file (relative paths from current directory)
    #[arg(short = 'f', long = "filelist", value_name = "FILE")]
    filelist: Vec<PathBuf>,

    /// Read list of images from file (relative paths from filelist directory)
    #[arg(short = 'F', long = "filelist-relative", value_name = "FILE")]
    filelist_relative: Vec<PathBuf>,

    /// Renderer backend to use
    #[arg(
        long = "backend",
        value_name = "BACKEND",
        default_value = "auto",
        value_parser = parse_backend,
    )]
    backend: BackendKind,

    /// Pixelation mode for Unicode backend (half or quarter blocks)
    #[arg(
        short = 'p',
        long = "pixelation",
        value_name = "MODE",
        default_value = "quarter",
        value_parser = parse_pixelation,
    )]
    pixelation: PixelationMode,

    /// Image rotation mode (exif or off)
    #[arg(
        long = "rotate",
        value_name = "MODE",
        default_value = "exif",
        value_parser = parse_rotation,
    )]
    rotation: RotationMode,

    /// Output geometry in character cells (WIDTHxHEIGHT, WIDTHx, or xHEIGHT)
    #[arg(short = 'g', long = "geometry", value_name = "SPEC")]
    geometry: Option<String>,

    /// Maximum width in terminal cells
    #[arg(short = 'w', long = "width", value_name = "CELLS", conflicts_with = "geometry")]
    width: Option<u32>,

    /// Maximum height in terminal cells
    #[arg(short = 'H', long = "height", value_name = "CELLS", conflicts_with = "geometry")]
    height: Option<u32>,

    /// Scale to fit width (may overflow height)
    #[arg(short = 'W', long = "fit-width", action = ArgAction::SetTrue)]
    fit_width: bool,

    /// Scale to fit height (may overflow width)
    #[arg(long = "fit-height", action = ArgAction::SetTrue)]
    fit_height: bool,

    /// Allow upscaling images smaller than terminal size. Optional 'i' for integer scaling
    #[arg(short = 'U', long = "upscale", value_name = "MODE")]
    upscale: Option<String>,

    /// Width stretch factor for aspect ratio correction (auto-detected if not specified)
    #[arg(long = "width-stretch", value_name = "FACTOR")]
    width_stretch: Option<f32>,

    /// Disable antialiasing when scaling images
    #[arg(short = 'a', long = "no-antialias", action = ArgAction::SetTrue)]
    no_antialias: bool,

    /// Arrange images in a grid of COLS or COLSxROWS
    #[arg(long = "grid", value_name = "COLS[xROWS]")]
    grid: Option<String>,

    /// Horizontal gap between grid columns
    #[arg(long = "grid-gap", default_value_t = 2, value_name = "CELLS")]
    grid_gap: u16,

    /// Loop animated images indefinitely instead of respecting embedded duration
    #[arg(long = "loop", action = ArgAction::SetTrue)]
    loop_forever: bool,

    /// Number of times to loop animation/video (-1 for infinite)
    #[arg(long = "loops", value_name = "COUNT", conflicts_with = "loop_forever")]
    loops: Option<i32>,

    /// Only render the first N frames of animation/video
    #[arg(long = "frames", value_name = "COUNT")]
    max_frames: Option<usize>,

    /// Start rendering at this frame offset
    #[arg(long = "frame-offset", value_name = "OFFSET", default_value_t = 0)]
    frame_offset: usize,

    /// Stop animation/video after this duration (e.g., "10s", "500ms")
    #[arg(short = 't', long = "duration", value_name = "DURATION")]
    max_duration: Option<String>,

    /// Reduce informational output
    #[arg(long = "quiet", short = 'q', action = ArgAction::SetTrue)]
    quiet: bool,

    /// Print verbose terminal and performance information
    #[arg(long = "verbose", short = 'v', action = ArgAction::SetTrue, conflicts_with = "quiet")]
    verbose: bool,

    /// Clear the terminal before rendering. Accepts optional value `every`.
    #[arg(long = "clear", value_name = "MODE", num_args = 0..=1, default_missing_value = "once")]
    clear: Option<String>,

    /// Wait duration between images (e.g. `0.5s`, `150ms`).
    #[arg(long = "wait", value_name = "DURATION")]
    wait: Option<String>,

    /// Wait between output rows when using --grid.
    #[arg(long = "wait-rows", value_name = "DURATION")]
    wait_rows: Option<String>,

    /// Title format to print above each image (supports `%f`, `%b`).
    #[arg(long = "title", value_name = "FORMAT")]
    title: Option<String>,

    /// Center image horizontally within terminal width (non-grid mode).
    #[arg(long = "center", action = ArgAction::SetTrue)]
    center: bool,

    /// Render inside the terminal's alternate screen buffer and restore on exit.
    #[arg(long = "alternate-screen", action = ArgAction::SetTrue)]
    alternate_screen: bool,

    /// Don't hide cursor while displaying images
    #[arg(short = 'E', long = "show-cursor", action = ArgAction::SetTrue)]
    show_cursor: bool,

    /// Background color to blend transparent pixels (`auto`, `none`, `#rrggbb`, `rgb()`).
    #[arg(
        short = 'b',
        long = "background",
        value_name = "COLOR",
        default_value = "auto"
    )]
    background: String,

    /// Checkerboard pattern color for transparency.
    #[arg(short = 'B', long = "pattern", value_name = "COLOR")]
    pattern: Option<String>,

    /// Scale factor for transparency checkerboard pattern.
    #[arg(long = "pattern-size", value_name = "INT", default_value_t = 1)]
    pattern_size: u16,

    /// Auto-crop image by removing same-color borders.
    #[arg(long = "auto-crop", action = ArgAction::SetTrue)]
    auto_crop: bool,

    /// Crop fixed border of pixels before auto-crop (e.g., `--crop-border 10`).
    #[arg(long = "crop-border", value_name = "PIXELS", default_value_t = 0)]
    crop_border: u32,

    /// Write output to file instead of stdout
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    output_file: Option<PathBuf>,

    /// Use 8-bit color mode (256 colors) for Unicode renderer
    #[arg(long = "color8", action = ArgAction::SetTrue)]
    color8: bool,

    /// Number of threads for parallel image decoding
    #[arg(long = "threads", value_name = "N")]
    threads: Option<usize>,

    /// Compression level for graphics protocols (0-9, default: 1)
    #[arg(long = "compress", value_name = "LEVEL", default_value_t = 1)]
    compress: u8,

    /// Force video decoding (skip image probe)
    #[arg(long = "force-video", action = ArgAction::SetTrue, conflicts_with = "force_image")]
    force_video: bool,

    /// Force image decoding (skip video probe)
    #[arg(short = 'I', long = "force-image", action = ArgAction::SetTrue)]
    force_image: bool,

    /// Enable horizontal scrolling animation. Optional delay in ms (default: 60)
    #[arg(long = "scroll", value_name = "MS")]
    scroll: Option<String>,

    /// Delta x and y when scrolling (default: 1:0)
    #[arg(long = "delta-move", value_name = "DX:DY")]
    delta_move: Option<String>,
}

impl Cli {
    pub fn into_config(self) -> Result<Config> {
        // Collect all inputs from command line and file lists
        let mut all_inputs = self.inputs.clone();

        // Process -f file lists (resolve relative to cwd)
        for filelist_path in &self.filelist {
            let images = read_filelist(filelist_path, false)?;
            all_inputs.extend(images);
        }

        // Process -F file lists (resolve relative to filelist directory)
        for filelist_path in &self.filelist_relative {
            let images = read_filelist(filelist_path, true)?;
            all_inputs.extend(images);
        }

        if all_inputs.is_empty() {
            return Err(RimgError::MissingInput);
        }

        let grid = match self.grid {
            Some(spec) => Some(parse_grid(&spec, self.grid_gap)?),
            None => None,
        };

        let wait_between_images = parse_optional_duration(self.wait.as_deref())?;
        let wait_between_rows = parse_optional_duration(self.wait_rows.as_deref())?;

        let (clear_once, clear_between) = parse_clear(self.clear.as_deref())?;

        if self.pattern_size == 0 {
            return Err(RimgError::other("pattern-size must be greater than zero"));
        }

        let background = parse_background_color(&self.background)?;
        let pattern_color = match self.pattern.as_deref() {
            Some(raw) => parse_optional_color(raw)?,
            None => None,
        };

        let max_duration = parse_optional_duration(self.max_duration.as_deref())?;

        // Parse geometry or use individual width/height
        let (width_cells, height_cells) = if let Some(ref geom) = self.geometry {
            parse_geometry(geom)?
        } else {
            (self.width, self.height)
        };

        // Parse upscale settings
        let (upscale, upscale_integer) = parse_upscale(self.upscale.as_deref());

        // Parse scrolling settings
        let (scroll_animation, scroll_delay, scroll_dx, scroll_dy) =
            parse_scroll_settings(self.scroll.as_deref(), self.delta_move.as_deref())?;

        // Validate compress level
        if self.compress > 9 {
            return Err(RimgError::other("compress level must be between 0 and 9"));
        }

        // Validate width stretch if provided
        if let Some(stretch) = self.width_stretch {
            if stretch <= 0.0 {
                return Err(RimgError::other("width-stretch must be positive"));
            }
        }

        // Use provided width_stretch or auto-detect from terminal
        let width_stretch = self.width_stretch.unwrap_or_else(|| {
            use crate::capabilities::current_terminal_size;
            current_terminal_size().recommended_width_stretch()
        });

        Ok(Config {
            inputs: all_inputs,
            backend: self.backend,
            pixelation: self.pixelation,
            rotation: self.rotation,
            sizing: RenderSizing {
                width_cells,
                height_cells,
                fit_width: self.fit_width,
                fit_height: self.fit_height,
                upscale,
                upscale_integer,
                width_stretch,
                antialias: !self.no_antialias,
            },
            grid,
            loop_forever: self.loop_forever,
            loops: self.loops,
            max_frames: self.max_frames,
            frame_offset: self.frame_offset,
            max_duration,
            quiet: self.quiet,
            verbose: self.verbose,
            clear_between,
            clear_once,
            wait_between_images,
            wait_between_rows,
            title_format: self.title,
            center: self.center,
            alternate_screen: self.alternate_screen,
            hide_cursor: !self.show_cursor,
            background,
            pattern_color,
            pattern_size: self.pattern_size,
            auto_crop: self.auto_crop,
            crop_border: self.crop_border,
            output_file: self.output_file,
            use_8bit_color: self.color8,
            threads: self.threads,
            compress_level: self.compress,
            force_video: self.force_video,
            force_image: self.force_image,
            scroll_animation,
            scroll_delay,
            scroll_dx,
            scroll_dy,
        })
    }
}

fn parse_backend(value: &str) -> std::result::Result<BackendKind, String> {
    BackendKind::from_str(value)
}

fn parse_pixelation(value: &str) -> std::result::Result<PixelationMode, String> {
    PixelationMode::from_str(value)
}

fn parse_rotation(value: &str) -> std::result::Result<RotationMode, String> {
    RotationMode::from_str(value)
}

fn parse_geometry(spec: &str) -> Result<(Option<u32>, Option<u32>)> {

    // Parse WIDTHxHEIGHT, WIDTHx, or xHEIGHT
    if spec.starts_with('x') {
        // xHEIGHT
        let height = spec[1..].parse::<u32>().map_err(|_| {
            RimgError::other(format!("invalid height in geometry '{}'", spec))
        })?;
        Ok((None, Some(height)))
    } else if spec.ends_with('x') {
        // WIDTHx
        let width = spec[..spec.len()-1].parse::<u32>().map_err(|_| {
            RimgError::other(format!("invalid width in geometry '{}'", spec))
        })?;
        Ok((Some(width), None))
    } else if let Some(pos) = spec.find('x') {
        // WIDTHxHEIGHT
        let width = spec[..pos].parse::<u32>().map_err(|_| {
            RimgError::other(format!("invalid width in geometry '{}'", spec))
        })?;
        let height = spec[pos+1..].parse::<u32>().map_err(|_| {
            RimgError::other(format!("invalid height in geometry '{}'", spec))
        })?;
        Ok((Some(width), Some(height)))
    } else {
        Err(RimgError::other(format!("invalid geometry '{}'. Expected WIDTHxHEIGHT, WIDTHx, or xHEIGHT", spec)))
    }
}

fn parse_upscale(spec: Option<&str>) -> (bool, bool) {
    match spec {
        None => (false, false),
        Some(s) if s.is_empty() => (true, false),
        Some(s) if s.eq_ignore_ascii_case("i") => (true, true),
        Some(_) => (true, false),
    }
}

fn parse_scroll_settings(scroll_ms: Option<&str>, delta: Option<&str>) -> Result<(bool, std::time::Duration, i32, i32)> {
    let scroll_enabled = scroll_ms.is_some();
    let scroll_delay = if let Some(ms_str) = scroll_ms {
        if ms_str.is_empty() {
            std::time::Duration::from_millis(60) // default
        } else {
            let ms = ms_str.parse::<u64>().map_err(|_| {
                RimgError::other(format!("invalid scroll delay '{}'", ms_str))
            })?;
            std::time::Duration::from_millis(ms)
        }
    } else {
        std::time::Duration::from_millis(60)
    };

    let (dx, dy) = if let Some(delta_str) = delta {
        let parts: Vec<&str> = delta_str.split(':').collect();
        if parts.is_empty() {
            return Err(RimgError::other("delta-move requires at least dx value"));
        }
        let dx = parts[0].parse::<i32>().map_err(|_| {
            RimgError::other(format!("invalid dx in delta-move '{}'", parts[0]))
        })?;
        let dy = if parts.len() > 1 {
            parts[1].parse::<i32>().map_err(|_| {
                RimgError::other(format!("invalid dy in delta-move '{}'", parts[1]))
            })?
        } else {
            0
        };
        (dx, dy)
    } else {
        (1, 0) // default: scroll right
    };

    Ok((scroll_enabled, scroll_delay, dx, dy))
}

fn parse_grid(spec: &str, spacing: u16) -> Result<GridOptions> {
    let parts: Vec<&str> = spec.split('x').collect();
    if parts.is_empty() {
        return Err(RimgError::other("grid value must be COLS or COLSxROWS"));
    }

    let columns = NonZeroUsize::from_str(parts[0]).map_err(|_| {
        RimgError::other("grid columns must be a positive integer greater than zero")
    })?;

    let rows = if parts.len() > 1 {
        Some(NonZeroUsize::from_str(parts[1]).map_err(|_| {
            RimgError::other("grid rows must be a positive integer greater than zero")
        })?)
    } else {
        None
    };

    Ok(GridOptions {
        columns,
        spacing,
        rows,
    })
}

fn parse_optional_duration(input: Option<&str>) -> Result<Option<std::time::Duration>> {
    match input {
        Some(raw) => Ok(Some(parse_duration(raw)?)),
        None => Ok(None),
    }
}

fn parse_clear(spec: Option<&str>) -> Result<(bool, bool)> {
    match spec {
        None => Ok((false, false)),
        Some(mode) => match mode {
            "once" => Ok((true, false)),
            "every" => Ok((false, true)),
            other => Err(RimgError::other(format!(
                "unsupported clear mode '{other}'. use 'every' or omit value"
            ))),
        },
    }
}

fn parse_duration(raw: &str) -> Result<std::time::Duration> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(RimgError::other("duration cannot be empty"));
    }

    if let Some(value) = trimmed.strip_suffix("ms") {
        let ms: f64 = value.trim().parse().map_err(|_| {
            RimgError::other(format!(
                "invalid millisecond duration '{raw}': expected number"
            ))
        })?;
        if ms < 0.0 {
            return Err(RimgError::other("duration must be non-negative"));
        }
        return Ok(std::time::Duration::from_millis(ms.round() as u64));
    }

    let seconds_str = trimmed.strip_suffix('s').unwrap_or(trimmed);
    let seconds: f64 = seconds_str
        .trim()
        .parse()
        .map_err(|_| RimgError::other(format!("invalid duration '{raw}': expected number")))?;
    if seconds < 0.0 {
        return Err(RimgError::other("duration must be non-negative"));
    }
    Ok(std::time::Duration::from_secs_f64(seconds))
}

fn parse_background_color(raw: &str) -> Result<BackgroundColor> {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "auto" {
        return Ok(BackgroundColor::Auto);
    }
    if lowered == "none" {
        return Ok(BackgroundColor::None);
    }
    let color = parse_rgb_color(raw)?;
    Ok(BackgroundColor::Color(color))
}

fn parse_optional_color(raw: &str) -> Result<Option<RgbColor>> {
    let lowered = raw.trim().to_ascii_lowercase();
    if lowered == "none" || lowered.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse_rgb_color(raw)?))
}

fn parse_rgb_color(raw: &str) -> Result<RgbColor> {
    let trimmed = raw.trim();
    if let Some(color) = parse_hex_color(trimmed) {
        return Ok(color);
    }
    if let Some(color) = parse_rgb_function(trimmed) {
        return Ok(color);
    }
    if let Some(color) = parse_named_color(trimmed) {
        return Ok(color);
    }
    Err(RimgError::other(format!("unrecognized color '{raw}'")))
}

fn parse_hex_color(input: &str) -> Option<RgbColor> {
    if input.len() == 7 && input.starts_with('#') {
        let r = u8::from_str_radix(&input[1..3], 16).ok()?;
        let g = u8::from_str_radix(&input[3..5], 16).ok()?;
        let b = u8::from_str_radix(&input[5..7], 16).ok()?;
        Some(RgbColor { r, g, b })
    } else {
        None
    }
}

fn parse_rgb_function(input: &str) -> Option<RgbColor> {
    let lower = input.to_ascii_lowercase();
    let body = lower.strip_prefix("rgb(")?.strip_suffix(')')?;
    let mut parts = body.split(',');
    let r = parse_component(parts.next()?)?;
    let g = parse_component(parts.next()?)?;
    let b = parse_component(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(RgbColor { r, g, b })
}

fn parse_component(component: &str) -> Option<u8> {
    let trimmed = component.trim();
    if let Some(hex) = trimmed.strip_prefix("0x") {
        u8::from_str_radix(hex, 16).ok()
    } else {
        let value: f64 = trimmed.parse().ok()?;
        if !(0.0..=255.0).contains(&value) {
            return None;
        }
        Some(value.round().clamp(0.0, 255.0) as u8)
    }
}

fn parse_named_color(name: &str) -> Option<RgbColor> {
    let map = html_color_map();
    map.get(&name.to_ascii_lowercase()).copied()
}

fn html_color_map() -> &'static HashMap<String, RgbColor> {
    static CACHE: OnceLock<HashMap<String, RgbColor>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut map = HashMap::new();
        for line in HTML_COLOR_DATA.lines() {
            let trimmed = line.trim();
            if !trimmed.starts_with('{') {
                continue;
            }
            let mut parts = trimmed.split('"');
            let _open = parts.next();
            let name = match parts.next() {
                Some(name) => name,
                None => continue,
            };
            let _middle = parts.next();
            let value = match parts.next() {
                Some(value) => value,
                None => continue,
            };
            if let Some(rgb) = parse_hex_color(value) {
                map.insert(name.to_ascii_lowercase(), rgb);
            }
        }
        map
    })
}

const HTML_COLOR_DATA: &str = include_str!("data/html-colors.inc");

/// Read a file list and return a vector of image paths.
/// If `relative_to_filelist` is true, relative paths are resolved relative to the
/// directory containing the file list. Otherwise, they are resolved relative to the
/// current working directory.
fn read_filelist(filelist_path: &Path, relative_to_filelist: bool) -> Result<Vec<PathBuf>> {
    let file = File::open(filelist_path).map_err(|err| {
        RimgError::other(format!(
            "failed to open file list '{}': {}",
            filelist_path.display(),
            err
        ))
    })?;

    let reader = BufReader::new(file);
    let mut images = Vec::new();

    let base_dir = if relative_to_filelist {
        filelist_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        std::env::current_dir().map_err(|err| {
            RimgError::other(format!("failed to get current directory: {}", err))
        })?
    };

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.map_err(|err| {
            RimgError::other(format!(
                "failed to read line {} from '{}': {}",
                line_num + 1,
                filelist_path.display(),
                err
            ))
        })?;

        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let image_path = PathBuf::from(trimmed);
        let resolved_path = if image_path.is_absolute() {
            image_path
        } else {
            base_dir.join(image_path)
        };

        images.push(resolved_path);
    }

    Ok(images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_grid_with_rows() {
        let grid = parse_grid("3x2", 1).expect("grid parse");
        assert_eq!(grid.columns.get(), 3);
        assert_eq!(grid.rows.unwrap().get(), 2);
    }

    #[test]
    fn parses_seconds_duration() {
        let dur = parse_duration("1.5").expect("duration");
        assert_eq!(dur.as_millis(), 1500);
    }

    #[test]
    fn parses_millis_duration() {
        let dur = parse_duration("250ms").expect("duration");
        assert_eq!(dur.as_millis(), 250);
    }

    #[test]
    fn parses_named_color() {
        let color = parse_rgb_color("AliceBlue").expect("color");
        assert_eq!((color.r, color.g, color.b), (0xF0, 0xF8, 0xFF));
    }

    #[test]
    fn parses_background_auto() {
        assert!(matches!(
            parse_background_color("auto").expect("background"),
            BackgroundColor::Auto
        ));
    }

    #[test]
    fn parses_pixelation_quarter() {
        let mode = parse_pixelation("quarter").expect("pixelation");
        assert_eq!(mode, PixelationMode::Quarter);
    }

    #[test]
    fn parses_pixelation_half() {
        let mode = parse_pixelation("half").expect("pixelation");
        assert_eq!(mode, PixelationMode::Half);
    }

    #[test]
    fn parses_rotation_exif() {
        let mode = parse_rotation("exif").expect("rotation");
        assert_eq!(mode, RotationMode::Exif);
    }

    #[test]
    fn parses_rotation_off() {
        let mode = parse_rotation("off").expect("rotation");
        assert_eq!(mode, RotationMode::Off);
    }

    #[test]
    fn reads_filelist_with_comments() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let filelist_path = temp_dir.join("test_filelist.txt");

        {
            let mut file = File::create(&filelist_path).expect("create file");
            writeln!(file, "# This is a comment").unwrap();
            writeln!(file, "image1.jpg").unwrap();
            writeln!(file, "").unwrap();
            writeln!(file, "image2.png").unwrap();
            writeln!(file, "# Another comment").unwrap();
            writeln!(file, "subdir/image3.gif").unwrap();
        }

        let images = read_filelist(&filelist_path, false).expect("read filelist");

        assert_eq!(images.len(), 3);
        assert!(images[0].to_string_lossy().ends_with("image1.jpg"));
        assert!(images[1].to_string_lossy().ends_with("image2.png"));
        assert!(images[2].to_string_lossy().ends_with("image3.gif"));

        std::fs::remove_file(&filelist_path).ok();
    }

    #[test]
    fn reads_filelist_relative_to_cwd() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let filelist_path = temp_dir.join("test_filelist_cwd.txt");

        {
            let mut file = File::create(&filelist_path).expect("create file");
            writeln!(file, "relative/image.jpg").unwrap();
        }

        let images = read_filelist(&filelist_path, false).expect("read filelist");

        assert_eq!(images.len(), 1);
        // Should be resolved from current directory, not temp_dir
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(images[0], cwd.join("relative/image.jpg"));

        std::fs::remove_file(&filelist_path).ok();
    }

    #[test]
    fn reads_filelist_relative_to_file() {
        use std::io::Write;
        let temp_dir = std::env::temp_dir();
        let filelist_path = temp_dir.join("test_filelist_relative.txt");

        {
            let mut file = File::create(&filelist_path).expect("create file");
            writeln!(file, "relative/image.jpg").unwrap();
        }

        let images = read_filelist(&filelist_path, true).expect("read filelist");

        assert_eq!(images.len(), 1);
        // Should be resolved from temp_dir
        assert_eq!(images[0], temp_dir.join("relative/image.jpg"));

        std::fs::remove_file(&filelist_path).ok();
    }

    #[test]
    fn parses_alternate_screen_flag() {
        let cli = Cli::parse_from(["showme", "--alternate-screen", "img.png"]);
        let config = cli.into_config().expect("config");
        assert!(config.alternate_screen);
    }
}
