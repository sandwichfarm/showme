use image::ImageEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::{FilterType, resize};

use super::BackgroundStyle;
use crate::backend::RenderOptions;
use crate::error::{Result, RimgError};
use crate::image::Frame;

/// Scale frame for graphics protocols (Kitty, iTerm2, Sixel)
/// These can display high-resolution images, so we keep original resolution
pub(crate) fn scale_frame_for_graphics(frame: &Frame, options: RenderOptions) -> (image::RgbaImage, u32, u32) {
    let pixels = &frame.pixels;

    // Calculate cell allocation (how much terminal space to use)
    let max_width_cells = options
        .sizing
        .width_cells
        .unwrap_or_else(|| pixels.width().min(options.terminal.columns as u32))
        .max(1)
        .min(options.terminal.columns as u32);

    let max_height_cells = options
        .sizing
        .height_cells
        .unwrap_or_else(|| pixels.height().min(options.terminal.rows as u32))
        .max(1)
        .min(options.terminal.rows as u32);

    // For graphics protocols: keep original image resolution
    // The terminal will display it crisply within the allocated cells
    // Only downscale if user explicitly set width/height limits
    (pixels.clone(), max_width_cells, max_height_cells)
}

/// Scale frame for Unicode backend - needs aggressive downscaling to match character blocks
pub(crate) fn scale_frame(frame: &Frame, options: RenderOptions) -> (image::RgbaImage, u32, u32) {
    let pixels = &frame.pixels;

    // For graphics protocols, use full terminal width
    // width_stretch is handled differently for unicode backends
    let effective_terminal_width = options.terminal.columns as u32;

    let max_width_cells = options
        .sizing
        .width_cells
        .unwrap_or_else(|| pixels.width().min(effective_terminal_width))
        .max(1)
        .min(effective_terminal_width);

    let max_height_cells = options
        .sizing
        .height_cells
        .unwrap_or_else(|| pixels.height().min(options.terminal.rows as u32))
        .max(1)
        .min(options.terminal.rows as u32);

    // Calculate scale factors
    let scale_width = max_width_cells as f32 / pixels.width() as f32;
    let scale_height = max_height_cells as f32 / pixels.height() as f32;

    // Choose scale based on fit mode
    let scale = if options.sizing.fit_width {
        scale_width // Allow height overflow
    } else if options.sizing.fit_height {
        scale_height // Allow width overflow
    } else {
        scale_width.min(scale_height) // Fit both dimensions
    };

    // Don't upscale unless explicitly requested
    let scale = if scale > 1.0 && !options.sizing.upscale {
        1.0
    } else {
        scale
    };

    // Apply integer upscaling constraint
    let scale = if options.sizing.upscale_integer && scale > 1.0 {
        scale.floor().max(1.0)
    } else {
        scale
    };

    let mut target_width = (pixels.width() as f32 * scale).round() as u32;
    let mut target_height = (pixels.height() as f32 * scale).round() as u32;

    if target_width == 0 {
        target_width = 1;
    }
    if target_height == 0 {
        target_height = 1;
    }

    let width_cells = target_width.max(1).min(max_width_cells);
    let height_cells = target_height.max(1).min(max_height_cells);

    let scaled = if target_width == pixels.width() && target_height == pixels.height() {
        pixels.clone()
    } else {
        // Choose filter based on antialias setting
        let filter = if options.sizing.antialias {
            FilterType::Lanczos3 // High quality antialiasing
        } else {
            FilterType::Nearest // Fast, no antialiasing
        };
        resize(pixels, target_width, target_height, filter)
    };

    (scaled, width_cells, height_cells)
}

pub(crate) fn encode_png(image: &image::RgbaImage, backend_name: &str) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    PngEncoder::new(&mut encoded)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ColorType::Rgba8.into(),
        )
        .map_err(|err| {
            RimgError::other(format!(
                "failed to encode PNG for {backend_name} backend: {err}",
            ))
        })?;
    Ok(encoded)
}

pub(crate) fn blend_transparency(image: &mut image::RgbaImage, background: BackgroundStyle) {
    if background.color.is_none() && background.pattern.is_none() {
        return;
    }

    let width = image.width();
    for (idx, pixel) in image.pixels_mut().enumerate() {
        let alpha = pixel[3] as f32 / 255.0;
        if alpha >= 1.0 {
            continue;
        }

        let x = (idx as u32) % width;
        let y = (idx as u32) / width;

        if let Some(bg) = background_rgb(x, y, background) {
            let inv = 1.0 - alpha;
            for channel in 0..3 {
                let fg = pixel[channel] as f32;
                let blended = fg * alpha + (bg[channel] as f32) * inv;
                pixel[channel] = blended.round().clamp(0.0, 255.0) as u8;
            }
            pixel[3] = 255;
        }
    }
}

pub(crate) fn background_rgb(x: u32, y: u32, background: BackgroundStyle) -> Option<[u8; 3]> {
    match (background.color, background.pattern) {
        (None, None) => None,
        (Some(color), None) => Some([color.r, color.g, color.b]),
        (None, Some(pattern)) => Some(pattern_rgb(pattern)),
        (Some(color), Some(pattern)) => {
            if checkerboard(x, y, background.pattern_size) {
                Some(pattern_rgb(pattern))
            } else {
                Some([color.r, color.g, color.b])
            }
        }
    }
}

pub(crate) fn checkerboard(x: u32, y: u32, pattern_size: u16) -> bool {
    let size = pattern_size.max(1) as u32;
    let tile = size * 4;
    let tile = tile.max(1);
    ((x / tile) + (y / tile)) % 2 == 0
}

fn pattern_rgb(color: crate::config::RgbColor) -> [u8; 3] {
    [color.r, color.g, color.b]
}
