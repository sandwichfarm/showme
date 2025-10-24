use super::chunk_util::Base64Chunks;
use super::image_util::{blend_transparency, encode_png, scale_frame};
use crate::backend::{Backend, RenderOptions, RenderedFrame};
use crate::config::BackendKind;
use crate::error::Result;
use crate::image::Frame;
use crate::tmux;

// timg uses 3072 raw bytes which encodes to 4096 base64 bytes
const BASE64_CHUNK: usize = 3072;

#[derive(Debug, Default)]
pub struct KittyBackend;

impl KittyBackend {
    fn build_chunks(
        &self,
        data: &[u8],
        width_cells: u32,
        height_cells: u32,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Vec<String> {
        let chunks = Base64Chunks::new(data, BASE64_CHUNK);
        let total = chunks.len();
        let mut lines = Vec::with_capacity(total.max(1));
        let avg_chunk = super::chunk_util::average_chunk_len(&chunks);
        let in_tmux = tmux::in_multiplexer();

        // Generate unique image ID (use timestamp + random for uniqueness)
        use std::time::{SystemTime, UNIX_EPOCH};
        let image_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u32)
            .unwrap_or(1);

        for (idx, chunk) in (&chunks).into_iter().enumerate() {
            let more = idx + 1 < total;

            let mut params = Vec::new();
            if idx == 0 {
                params.push(format!("a=T"));
                params.push(format!("f=100"));
                params.push(format!("q=2")); // Suppress terminal feedback
                params.push(format!("i={}", image_id)); // Unique image ID
                params.push(format!("s={}", pixel_width));
                params.push(format!("v={}", pixel_height));
                params.push(format!("c={}", width_cells.max(1)));
                params.push(format!("r={}", height_cells.max(1)));
            }
            if more {
                params.push("m=1".to_string());
            }

            let mut line = String::with_capacity(12 + avg_chunk);
            line.push_str("\x1b_G");
            if !params.is_empty() {
                line.push_str(&params.join(","));
            }
            line.push(';');
            line.push_str(chunk);
            line.push_str("\x1b\\");

            // Wrap in tmux DCS passthrough if needed
            if in_tmux {
                line = tmux::wrap_for_tmux(&line);
            }

            lines.push(line);
        }

        lines
    }
}

impl Backend for KittyBackend {
    fn name(&self) -> &'static str {
        "kitty"
    }

    fn supported_kind(&self) -> BackendKind {
        BackendKind::Kitty
    }

    fn render(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        // For Kitty graphics, don't downscale - keep original resolution
        // Just calculate cell allocation
        let pixels = &frame.pixels;

        let width_cells = options.sizing.width_cells
            .unwrap_or(options.terminal.columns as u32)
            .max(1)
            .min(options.terminal.columns as u32);

        let height_cells = options.sizing.height_cells
            .unwrap_or(options.terminal.rows as u32)
            .max(1)
            .min(options.terminal.rows as u32);

        let mut image = pixels.clone();
        blend_transparency(&mut image, options.background);
        let png = encode_png(&image, "kitty")?;

        let lines = self.build_chunks(
            &png,
            width_cells,
            height_cells,
            image.width(),
            image.height(),
        );

        if options.verbose {
            eprintln!("  [Kitty] Rendering {}x{} pixels in {}x{} cells",
                     image.width(), image.height(), width_cells, height_cells);
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
    use crate::config::RenderSizing;
    use crate::image::Frame as ImageFrame;
    use crate::{
        backend::{BackgroundStyle, RenderOptions},
        capabilities::TerminalSize,
    };
    use image::{ImageBuffer, Rgba};
    use std::time::Duration;

    #[test]
    fn renders_single_chunk_for_small_image() {
        let mut buffer = ImageBuffer::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
        buffer.put_pixel(1, 1, Rgba([0, 255, 0, 255]));

        let frame = ImageFrame {
            pixels: buffer,
            delay: Duration::ZERO,
        };

        let backend = KittyBackend;
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

        assert_eq!(rendered.width_cells, 2);
        assert_eq!(rendered.height_cells, 0);
        assert_eq!(rendered.lines.len(), 1);
        assert!(rendered.lines[0].starts_with("\x1b_G"));
        assert!(rendered.lines[0].ends_with("\x1b\\"));
    }
}
