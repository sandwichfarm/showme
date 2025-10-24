use super::chunk_util::Base64Chunks;
use super::image_util::{blend_transparency, encode_png, scale_frame};
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
        let total = chunks.len();
        let mut lines = Vec::with_capacity(total.max(1));
        let avg_chunk = super::chunk_util::average_chunk_len(&chunks);
        let in_tmux = tmux::in_multiplexer();

        for (idx, chunk) in (&chunks).into_iter().enumerate() {
            let more = idx + 1 < total;

            let mut line = String::with_capacity(24 + avg_chunk);
            line.push_str("\x1b]1337;File=");
            if idx == 0 {
                line.push_str(&format!(
                    "inline=1;size={};width={};height={};preserveAspectRatio=1",
                    data.len(),
                    width_cells.max(1),
                    height_cells.max(1)
                ));
            } else {
                line.push_str("inline=1");
            }

            if more {
                line.push_str(";m=1");
            }

            line.push(':');
            line.push_str(chunk);
            line.push('\x07');

            // Wrap in tmux DCS passthrough if needed
            if in_tmux {
                line = tmux::wrap_for_tmux(&line);
            }

            lines.push(line);
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
        let (mut image, width_cells, height_cells) = scale_frame(frame, options);
        blend_transparency(&mut image, options.background);
        let png = encode_png(&image, "iterm2")?;
        let lines = self.build_chunks(&png, width_cells, height_cells);

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
                    },
                    background: BackgroundStyle {
                        color: None,
                        pattern: None,
                        pattern_size: 1,
                    },
                    pixelation: crate::config::PixelationMode::Quarter,
                    use_8bit_color: false,
                    compress_level: 1,
                },
            )
            .expect("render succeeds");

        assert_eq!(rendered.lines.len(), 1);
        assert!(rendered.lines[0].starts_with("\x1b]1337;File="));
        assert!(rendered.lines[0].ends_with('\x07'));
        assert_eq!(rendered.width_cells, 1);
        assert_eq!(rendered.height_cells, 0);
    }
}
