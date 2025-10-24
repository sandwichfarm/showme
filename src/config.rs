use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Auto,
    Unicode,
    Kitty,
    Iterm2,
    Sixel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelationMode {
    Half,
    Quarter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationMode {
    Off,
    Exif,
}

impl Default for RotationMode {
    fn default() -> Self {
        Self::Exif
    }
}

impl FromStr for RotationMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "exif" => Ok(Self::Exif),
            other => Err(format!(
                "unsupported rotation mode '{}'. valid choices: exif, off",
                other
            )),
        }
    }
}

impl Default for PixelationMode {
    fn default() -> Self {
        Self::Quarter
    }
}

impl FromStr for PixelationMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "half" | "h" => Ok(Self::Half),
            "quarter" | "q" => Ok(Self::Quarter),
            other => Err(format!(
                "unsupported pixelation mode '{}'. valid choices: half (h), quarter (q)",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl BackendKind {
    pub const fn variants() -> &'static [&'static str] {
        &["auto", "unicode", "kitty", "iterm2", "sixel"]
    }
}

impl FromStr for BackendKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "unicode" | "block" | "blocks" => Ok(Self::Unicode),
            "kitty" => Ok(Self::Kitty),
            "iterm2" | "iterm" => Ok(Self::Iterm2),
            "sixel" => Ok(Self::Sixel),
            other => Err(format!(
                "unsupported backend '{}'. valid choices: {:?}",
                other,
                Self::variants()
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometrySpec {
    Full(u32, u32),
    WidthOnly(u32),
    HeightOnly(u32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderSizing {
    pub width_cells: Option<u32>,
    pub height_cells: Option<u32>,
    pub fit_width: bool,
    pub fit_height: bool,
    pub upscale: bool,
    pub upscale_integer: bool,
    pub width_stretch: f32,
    pub antialias: bool,
}

impl RenderSizing {
    pub const fn unconstrained() -> Self {
        Self {
            width_cells: None,
            height_cells: None,
            fit_width: false,
            fit_height: false,
            upscale: false,
            upscale_integer: false,
            width_stretch: 1.0,
            antialias: true,
        }
    }
}

impl Default for RenderSizing {
    fn default() -> Self {
        Self::unconstrained()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridOptions {
    pub columns: NonZeroUsize,
    pub spacing: u16,
    pub rows: Option<NonZeroUsize>,
}

impl GridOptions {
    pub fn rows_for_total(&self, total: usize) -> usize {
        let cols = self.columns.get();
        let rows_needed = (total + cols - 1) / cols;
        if let Some(limit) = self.rows {
            rows_needed.min(limit.get())
        } else {
            rows_needed
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub inputs: Vec<PathBuf>,
    pub backend: BackendKind,
    pub pixelation: PixelationMode,
    pub rotation: RotationMode,
    pub sizing: RenderSizing,
    pub grid: Option<GridOptions>,
    pub loop_forever: bool,
    pub loops: Option<i32>, // -1 for infinite, None for default behavior
    pub max_frames: Option<usize>,
    pub frame_offset: usize,
    pub max_duration: Option<std::time::Duration>,
    pub quiet: bool,
    pub verbose: bool,
    pub clear_between: bool,
    pub clear_once: bool,
    pub wait_between_images: Option<std::time::Duration>,
    pub wait_between_rows: Option<std::time::Duration>,
    pub title_format: Option<String>,
    pub center: bool,
    pub alternate_screen: bool,
    pub hide_cursor: bool,
    pub background: BackgroundColor,
    pub pattern_color: Option<RgbColor>,
    pub pattern_size: u16,
    pub auto_crop: bool,
    pub crop_border: u32,
    pub output_file: Option<PathBuf>,
    pub use_8bit_color: bool,
    pub threads: Option<usize>,
    pub compress_level: u8,
    pub force_video: bool,
    pub force_image: bool,
    pub scroll_animation: bool,
    pub scroll_delay: std::time::Duration,
    pub scroll_dx: i32,
    pub scroll_dy: i32,
}

impl Config {
    pub fn validate(&self) -> bool {
        !self.inputs.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundColor {
    Auto,
    None,
    Color(RgbColor),
}
