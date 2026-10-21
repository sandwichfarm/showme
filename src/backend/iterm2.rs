use super::chunk_util::Base64Chunks;
use super::image_util::{blend_transparency, encode_png};
use crate::backend::{Backend, RenderOptions, RenderedFrame};
use crate::config::BackendKind;
use crate::error::Result;
use crate::image::Frame;
use crate::tmux;

const BASE64_CHUNK: usize = 4096;

#[derive(Debug, Default)]
pub struct ITerm2Backend;

impl ITerm2Backend {
    fn build_chunks(&self, data: &[u8], width_cells: u32, height_cells: u32) -> Vec<String> {
        let chunks = Base64Chunks::new(data, BASE64_CHUNK);
        let arguments = format!(
            "inline=1;size={};width={};height={};preserveAspectRatio=1",
            data.len(),
            width_cells.max(1),
            height_cells.max(1)
        );
        // A File command contains an entire file. Kitty's m=1 continuation flag
        // is not part of OSC 1337. Use iTerm2 3.5's documented multipart framing
        // only when a single sequence would exceed the 1 MiB transport limit.
        let mut lines = if chunks.encoded_len() + arguments.len() + 32 <= 1024 * 1024 {
            let mut command = format!("\x1b]1337;File={arguments}:");
            for chunk in &chunks {
                command.push_str(chunk);
            }
            command.push('\x07');
            vec![command]
        } else {
            let mut commands = Vec::with_capacity(chunks.len() + 2);
            commands.push(format!("\x1b]1337;MultipartFile={arguments}\x07"));
            for chunk in &chunks {
                commands.push(format!("\x1b]1337;FilePart={chunk}\x07"));
            }
            commands.push("\x1b]1337;FileEnd\x07".into());
            commands
        };
        for line in &mut lines {
            *line = tmux::wrap_if_needed(line);
        }

        lines
    }
}

impl Backend for ITerm2Backend {
    fn name(&self) -> &'static str {
        "iterm2"
    }

    fn supported_kind(&self) -> BackendKind {
        BackendKind::Iterm2
    }

    fn render(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        // For iTerm2 graphics, don't downscale - keep original resolution
        // Just calculate cell allocation based on aspect ratio
        let pixels = &frame.pixels;

        let max_width_cells = options
            .sizing
            .width_cells
            .unwrap_or(options.terminal.columns as u32)
            .min(options.terminal.columns as u32);

        let max_height_cells = options
            .sizing
            .height_cells
            .unwrap_or(options.terminal.rows as u32)
            .min(options.terminal.rows as u32);

        // Calculate aspect-ratio-preserving cell allocation
        let cell_aspect = 0.5;
        let img_aspect = pixels.width() as f64 / pixels.height() as f64;

        let (width_cells, height_cells) =
            if options.sizing.width_cells.is_some() && options.sizing.height_cells.is_some() {
                (max_width_cells, max_height_cells)
            } else {
                let width_if_height_limited =
                    (max_height_cells as f64 * img_aspect / cell_aspect) as u32;
                let height_if_width_limited =
                    (max_width_cells as f64 * cell_aspect / img_aspect) as u32;

                if width_if_height_limited <= max_width_cells {
                    (width_if_height_limited.max(1), max_height_cells)
                } else {
                    (max_width_cells, height_if_width_limited.max(1))
                }
            };

        let mut image = pixels.clone();
        blend_transparency(&mut image, options.background);
        let png = encode_png(&image, "iterm2", options.compress_level)?;
        let lines = self.build_chunks(&png, width_cells, height_cells);

        if options.verbose {
            eprintln!(
                "  [iTerm2] Rendering {}x{} pixels in {}x{} cells",
                image.width(),
                image.height(),
                width_cells,
                height_cells
            );
        }

        Ok(RenderedFrame {
            lines,
            width_cells,
            height_cells: 0,
            delay: frame.delay,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{BackgroundStyle, RenderOptions};
    use crate::capabilities::TerminalSize;
    use crate::config::RenderSizing;
    use crate::image::Frame as ImageFrame;
    use image::{ImageBuffer, Rgba};
    use std::time::Duration;

    #[test]
    fn emits_iterm_sequence() {
        let buffer = ImageBuffer::from_pixel(1, 1, Rgba([128, 64, 32, 255]));
        let frame = ImageFrame {
            pixels: buffer,
            delay: Duration::ZERO,
        };

        let backend = ITerm2Backend;
        let rendered = backend
            .render(
                &frame,
                RenderOptions {
                    sizing: RenderSizing::unconstrained(),
                    terminal: TerminalSize {
                        columns: 80,
                        rows: 24,
                        width_pixels: None,
                        height_pixels: None,
                    },
                    background: BackgroundStyle {
                        color: None,
                        pattern: None,
                        pattern_size: 1,
                    },
                    pixelation: crate::config::PixelationMode::Quarter,
                    use_8bit_color: false,
                    compress_level: 1,
                    verbose: false,
                },
            )
            .expect("render succeeds");

        assert_eq!(rendered.lines.len(), 1);
        assert!(rendered.lines[0].starts_with("\x1b]1337;File="));
        assert!(rendered.lines[0].ends_with('\x07'));
        assert_eq!(rendered.width_cells, 48);
        assert_eq!(rendered.height_cells, 0);
    }
}
