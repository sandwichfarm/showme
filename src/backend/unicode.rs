use image::imageops::{FilterType, resize};
use image::{Pixel, Rgba};

use crate::backend::{Backend, BackgroundStyle, RenderOptions, RenderedFrame};
use crate::config::{BackendKind, PixelationMode};
use crate::error::Result;
use crate::image::Frame;

#[derive(Debug, Default)]
pub struct UnicodeBackend;

#[derive(Debug, Clone, Copy)]
enum QuarterBlock {
    Empty,
    TopLeft,     // ▘
    TopRight,    // ▝
    BotLeft,     // ▖
    BotRight,    // ▗
    Top,         // ▀
    Bottom,      // ▄
    Left,        // ▌
    Right,       // ▐
    TopLeftBotRight, // ▚
    TopRightBotLeft, // ▞
    Full,        // █
}

impl QuarterBlock {
    fn to_char(self) -> char {
        match self {
            Self::Empty => ' ',
            Self::TopLeft => '▘',
            Self::TopRight => '▝',
            Self::BotLeft => '▖',
            Self::BotRight => '▗',
            Self::Top => '▀',
            Self::Bottom => '▄',
            Self::Left => '▌',
            Self::Right => '▐',
            Self::TopLeftBotRight => '▚',
            Self::TopRightBotLeft => '▞',
            Self::Full => '█',
        }
    }
}

impl Backend for UnicodeBackend {
    fn name(&self) -> &'static str {
        "unicode"
    }

    fn supported_kind(&self) -> BackendKind {
        BackendKind::Unicode
    }

    fn render(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        match options.pixelation {
            PixelationMode::Half => self.render_half_blocks(frame, options),
            PixelationMode::Quarter => self.render_quarter_blocks(frame, options),
        }
    }
}

impl UnicodeBackend {
    fn render_half_blocks(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        let sizing = options.sizing;
        let terminal = options.terminal;

        let max_width_cells = sizing
            .width_cells
            .unwrap_or_else(|| frame.pixels.width().min(terminal.columns as u32));
        let max_width_cells = max_width_cells.max(1).min(terminal.columns as u32);

        let max_height_pixels = sizing
            .height_cells
            .map(|cells| cells.saturating_mul(2))
            .unwrap_or(
                frame
                    .pixels
                    .height()
                    .min((terminal.rows as u32).saturating_mul(2).max(1)),
            );

        let mut scale_width = max_width_cells as f32 / frame.pixels.width() as f32;
        let mut scale_height = max_height_pixels as f32 / frame.pixels.height() as f32;

        // Apply upscale logic
        if !sizing.upscale {
            scale_width = scale_width.min(1.0);
            scale_height = scale_height.min(1.0);
        }

        // Apply fit-width logic
        let scale = if sizing.fit_width {
            scale_width
        } else {
            scale_width.min(scale_height)
        };

        // Scale image, then apply width_stretch to width
        let base_width = (frame.pixels.width() as f32 * scale).round() as u32;
        let base_height = (frame.pixels.height() as f32 * scale).round() as u32;

        // Apply width_stretch but cap at max to prevent overflow
        let stretched_width = (base_width as f32 * sizing.width_stretch).round() as u32;
        let mut target_width = stretched_width.min(max_width_cells);
        let mut target_height = base_height;

        if target_width == 0 {
            target_width = 1;
        }
        if target_height == 0 {
            target_height = 1;
        }

        if target_height % 2 != 0 {
            target_height += 1;
        }

        let scaled =
            if target_width == frame.pixels.width() && target_height == frame.pixels.height() {
                frame.pixels.clone()
            } else {
                resize(
                    &frame.pixels,
                    target_width,
                    target_height,
                    FilterType::Triangle,
                )
            };

        let mut lines = Vec::with_capacity((target_height as usize + 1) / 2);
        let mut y = 0;
        while y < scaled.height() {
            let mut line = String::with_capacity((target_width as usize) * 24);
            let mut x = 0;
            while x < scaled.width() {
                let top = *scaled.get_pixel(x, y);
                let bottom = if y + 1 < scaled.height() {
                    *scaled.get_pixel(x, y + 1)
                } else {
                    Rgba([0, 0, 0, 0])
                };

                append_half_block(&mut line, top, bottom, x, y, options.background, options.use_8bit_color);
                x += 1;
            }
            line.push_str("\x1b[0m");
            lines.push(line);
            y += 2;
        }

        Ok(RenderedFrame {
            lines,
            width_cells: target_width,
            height_cells: target_height / 2,
            delay: frame.delay,
        })
    }

    fn render_quarter_blocks(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        let sizing = options.sizing;
        let terminal = options.terminal;

        let max_width_cells = sizing
            .width_cells
            .unwrap_or_else(|| frame.pixels.width().min(terminal.columns as u32));
        let max_width_cells = max_width_cells.max(1).min(terminal.columns as u32);

        // Quarter blocks: each cell shows 2x2 pixels
        let max_height_pixels = sizing
            .height_cells
            .map(|cells| cells.saturating_mul(2))
            .unwrap_or(
                frame
                    .pixels
                    .height()
                    .min((terminal.rows as u32).saturating_mul(2).max(1)),
            );

        // For quarter blocks, each cell handles 2 pixels horizontally
        let mut scale_width = (max_width_cells * 2) as f32 / frame.pixels.width() as f32;
        let mut scale_height = max_height_pixels as f32 / frame.pixels.height() as f32;

        // Apply upscale logic
        if !sizing.upscale {
            scale_width = scale_width.min(1.0);
            scale_height = scale_height.min(1.0);
        }

        // Apply fit-width logic
        let scale = if sizing.fit_width {
            scale_width
        } else {
            scale_width.min(scale_height)
        };

        // Scale image, then apply width_stretch to width
        let base_width = (frame.pixels.width() as f32 * scale).round() as u32;
        let base_height = (frame.pixels.height() as f32 * scale).round() as u32;

        // Apply width_stretch but cap at max to prevent overflow
        let stretched_width = (base_width as f32 * sizing.width_stretch).round() as u32;
        let mut target_width = stretched_width.min(max_width_cells * 2);
        let mut target_height = base_height;

        if options.verbose {
            eprintln!("  [Unicode] Aspect ratio correction: {}x{} -> {}x{} (stretch={:.1}x)",
                     base_width, base_height, target_width, target_height, sizing.width_stretch);
        }

        if target_width == 0 {
            target_width = 2;
        }
        if target_height == 0 {
            target_height = 2;
        }

        // Ensure even dimensions for 2x2 blocks
        if target_width % 2 != 0 {
            target_width += 1;
        }
        if target_height % 2 != 0 {
            target_height += 1;
        }

        let scaled =
            if target_width == frame.pixels.width() && target_height == frame.pixels.height() {
                frame.pixels.clone()
            } else {
                resize(
                    &frame.pixels,
                    target_width,
                    target_height,
                    FilterType::Triangle,
                )
            };

        let mut lines = Vec::with_capacity((target_height as usize + 1) / 2);
        let mut y = 0;
        while y < scaled.height() {
            let mut line = String::with_capacity((target_width as usize / 2) * 24);
            let mut x = 0;
            while x < scaled.width() {
                let tl = *scaled.get_pixel(x, y);
                let tr = if x + 1 < scaled.width() {
                    *scaled.get_pixel(x + 1, y)
                } else {
                    tl
                };
                let bl = if y + 1 < scaled.height() {
                    *scaled.get_pixel(x, y + 1)
                } else {
                    tl
                };
                let br = if x + 1 < scaled.width() && y + 1 < scaled.height() {
                    *scaled.get_pixel(x + 1, y + 1)
                } else {
                    tl
                };

                append_quarter_block(&mut line, tl, tr, bl, br, x, y, options.background, options.use_8bit_color);
                x += 2;
            }
            line.push_str("\x1b[0m");
            lines.push(line);
            y += 2;
        }

        Ok(RenderedFrame {
            lines,
            width_cells: target_width / 2,
            height_cells: target_height / 2,
            delay: frame.delay,
        })
    }
}

fn append_half_block(
    line: &mut String,
    top: Rgba<u8>,
    bottom: Rgba<u8>,
    x: u32,
    y: u32,
    background: BackgroundStyle,
    use_8bit: bool,
) {
    let top = resolve_color(top, x, y, background);
    let bottom = resolve_color(bottom, x, y + 1, background);

    match (top, bottom) {
        (None, None) => {
            line.push_str("\x1b[0m ");
        }
        (Some(top), Some(bottom)) => {
            push_fg(line, top, use_8bit);
            push_bg(line, bottom, use_8bit);
            line.push('▀');
        }
        (Some(top), None) => {
            push_fg(line, top, use_8bit);
            reset_bg(line);
            line.push('▀');
        }
        (None, Some(bottom)) => {
            push_fg(line, bottom, use_8bit);
            reset_bg(line);
            line.push('▄');
        }
    }
}

fn resolve_color(pixel: Rgba<u8>, x: u32, y: u32, background: BackgroundStyle) -> Option<[u8; 3]> {
    if pixel.channels()[3] >= 16 {
        Some([pixel[0], pixel[1], pixel[2]])
    } else {
        super::image_util::background_rgb(x, y, background)
    }
}

fn push_fg(buf: &mut String, rgb: [u8; 3], use_8bit: bool) {
    use std::fmt::Write as _;
    if use_8bit {
        let idx = crate::color_quantize::rgb_to_256(rgb[0], rgb[1], rgb[2]);
        let _ = write!(buf, "\x1b[38;5;{}m", idx);
    } else {
        let _ = write!(buf, "\x1b[38;2;{};{};{}m", rgb[0], rgb[1], rgb[2]);
    }
}

fn push_bg(buf: &mut String, rgb: [u8; 3], use_8bit: bool) {
    use std::fmt::Write as _;
    if use_8bit {
        let idx = crate::color_quantize::rgb_to_256(rgb[0], rgb[1], rgb[2]);
        let _ = write!(buf, "\x1b[48;5;{}m", idx);
    } else {
        let _ = write!(buf, "\x1b[48;2;{};{};{}m", rgb[0], rgb[1], rgb[2]);
    }
}

fn reset_bg(buf: &mut String) {
    buf.push_str("\x1b[49m");
}

// Quarter block rendering: chooses the best glyph and colors for 2x2 pixel block
fn append_quarter_block(
    line: &mut String,
    tl: Rgba<u8>,
    tr: Rgba<u8>,
    bl: Rgba<u8>,
    br: Rgba<u8>,
    x: u32,
    y: u32,
    background: BackgroundStyle,
    use_8bit: bool,
) {
    let tl = resolve_color(tl, x, y, background);
    let tr = resolve_color(tr, x + 1, y, background);
    let bl = resolve_color(bl, x, y + 1, background);
    let br = resolve_color(br, x + 1, y + 1, background);

    // If all transparent, just output space
    if tl.is_none() && tr.is_none() && bl.is_none() && br.is_none() {
        line.push_str("\x1b[0m ");
        return;
    }

    // Choose the best block character and colors
    let (block, fg, bg) = find_best_quarter_block(tl, tr, bl, br);

    if let Some(fg_rgb) = fg {
        push_fg(line, fg_rgb, use_8bit);
    }
    if let Some(bg_rgb) = bg {
        push_bg(line, bg_rgb, use_8bit);
    } else {
        reset_bg(line);
    }
    line.push(block.to_char());
}

fn find_best_quarter_block(
    tl: Option<[u8; 3]>,
    tr: Option<[u8; 3]>,
    bl: Option<[u8; 3]>,
    br: Option<[u8; 3]>,
) -> (QuarterBlock, Option<[u8; 3]>, Option<[u8; 3]>) {
    // Count transparent vs opaque quadrants
    let count = [tl, tr, bl, br].iter().filter(|p| p.is_some()).count();

    if count == 0 {
        return (QuarterBlock::Empty, None, None);
    }

    if count == 4 {
        // All filled - check if similar colors
        let colors = [tl.unwrap(), tr.unwrap(), bl.unwrap(), br.unwrap()];
        let avg = average_colors(&colors);
        if colors_similar(&colors, avg) {
            return (QuarterBlock::Full, Some(avg), None);
        }
    }

    // Try different block patterns and find the best one
    let mut best_block = QuarterBlock::Empty;
    let mut best_fg = None;
    let mut best_bg = None;
    let mut best_error = f32::MAX;

    // Test each possible block pattern
    let patterns = [
        (QuarterBlock::Empty, vec![], vec![tl, tr, bl, br]),
        (QuarterBlock::TopLeft, vec![tl], vec![tr, bl, br]),
        (QuarterBlock::TopRight, vec![tr], vec![tl, bl, br]),
        (QuarterBlock::BotLeft, vec![bl], vec![tl, tr, br]),
        (QuarterBlock::BotRight, vec![br], vec![tl, tr, bl]),
        (QuarterBlock::Top, vec![tl, tr], vec![bl, br]),
        (QuarterBlock::Bottom, vec![bl, br], vec![tl, tr]),
        (QuarterBlock::Left, vec![tl, bl], vec![tr, br]),
        (QuarterBlock::Right, vec![tr, br], vec![tl, bl]),
        (QuarterBlock::TopLeftBotRight, vec![tl, br], vec![tr, bl]),
        (QuarterBlock::TopRightBotLeft, vec![tr, bl], vec![tl, br]),
        (QuarterBlock::Full, vec![tl, tr, bl, br], vec![]),
    ];

    for (block, fg_pixels, bg_pixels) in patterns {
        let fg_colors: Vec<[u8; 3]> = fg_pixels.iter().filter_map(|&p| p).collect();
        let bg_colors: Vec<[u8; 3]> = bg_pixels.iter().filter_map(|&p| p).collect();

        if fg_colors.is_empty() && !matches!(block, QuarterBlock::Empty) {
            continue;
        }

        let fg_avg = if !fg_colors.is_empty() {
            Some(average_colors(&fg_colors))
        } else {
            None
        };

        let bg_avg = if !bg_colors.is_empty() {
            Some(average_colors(&bg_colors))
        } else {
            None
        };

        // Calculate error for this configuration
        let mut error = 0.0f32;
        for (pixel, should_be_fg) in [
            (tl, matches!(block, QuarterBlock::TopLeft | QuarterBlock::Top | QuarterBlock::Left | QuarterBlock::TopLeftBotRight | QuarterBlock::Full)),
            (tr, matches!(block, QuarterBlock::TopRight | QuarterBlock::Top | QuarterBlock::Right | QuarterBlock::TopRightBotLeft | QuarterBlock::Full)),
            (bl, matches!(block, QuarterBlock::BotLeft | QuarterBlock::Bottom | QuarterBlock::Left | QuarterBlock::TopRightBotLeft | QuarterBlock::Full)),
            (br, matches!(block, QuarterBlock::BotRight | QuarterBlock::Bottom | QuarterBlock::Right | QuarterBlock::TopLeftBotRight | QuarterBlock::Full)),
        ] {
            if let Some(color) = pixel {
                let target = if should_be_fg { fg_avg } else { bg_avg };
                if let Some(t) = target {
                    error += color_distance(color, t);
                } else {
                    error += 100.0; // Penalty for not matching
                }
            }
        }

        if error < best_error {
            best_error = error;
            best_block = block;
            best_fg = fg_avg;
            best_bg = bg_avg;
        }
    }

    (best_block, best_fg, best_bg)
}

fn average_colors(colors: &[[u8; 3]]) -> [u8; 3] {
    if colors.is_empty() {
        return [0, 0, 0];
    }
    let sum_r: u32 = colors.iter().map(|c| c[0] as u32).sum();
    let sum_g: u32 = colors.iter().map(|c| c[1] as u32).sum();
    let sum_b: u32 = colors.iter().map(|c| c[2] as u32).sum();
    let len = colors.len() as u32;
    [(sum_r / len) as u8, (sum_g / len) as u8, (sum_b / len) as u8]
}

fn color_distance(c1: [u8; 3], c2: [u8; 3]) -> f32 {
    let dr = c1[0] as f32 - c2[0] as f32;
    let dg = c1[1] as f32 - c2[1] as f32;
    let db = c1[2] as f32 - c2[2] as f32;
    (dr * dr + dg * dg + db * db).sqrt()
}

fn colors_similar(colors: &[[u8; 3]], avg: [u8; 3]) -> bool {
    colors.iter().all(|&c| color_distance(c, avg) < 30.0)
}
