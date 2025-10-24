mod unicode;

#[cfg(feature = "kitty")]
mod kitty;

#[cfg(feature = "iterm2")]
mod iterm2;

#[cfg(feature = "sixel")]
mod sixel;

mod chunk_util;
mod image_util;

use std::time::Duration;

pub use unicode::UnicodeBackend;

#[cfg(feature = "kitty")]
pub use kitty::KittyBackend;

#[cfg(feature = "iterm2")]
pub use iterm2::ITerm2Backend;

#[cfg(feature = "sixel")]
pub use sixel::SixelBackend;

use crate::config::{BackendKind, PixelationMode, RenderSizing, RgbColor};
use crate::error::Result;
#[cfg(any(
    not(feature = "kitty"),
    not(feature = "iterm2"),
    not(feature = "sixel")
))]
use crate::error::RimgError;
use crate::image::Frame;
use crate::{capabilities::TerminalSize, config};

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub sizing: RenderSizing,
    pub terminal: TerminalSize,
    pub background: BackgroundStyle,
    pub pixelation: PixelationMode,
    pub use_8bit_color: bool,
    pub compress_level: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct BackgroundStyle {
    pub color: Option<RgbColor>,
    pub pattern: Option<RgbColor>,
    pub pattern_size: u16,
}

#[derive(Debug, Clone)]
pub struct RenderedFrame {
    pub lines: Vec<String>,
    pub width_cells: u32,
    pub height_cells: u32,
    pub delay: Duration,
}

pub trait Backend {
    fn name(&self) -> &'static str;
    fn supported_kind(&self) -> BackendKind;
    fn render(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame>;
}

pub struct BackendFactory;

impl BackendFactory {
    pub fn build(kind: BackendKind) -> Result<Box<dyn Backend + Send + Sync>> {
        match kind {
            BackendKind::Unicode | BackendKind::Auto => Ok(Box::new(UnicodeBackend::default())),
            BackendKind::Kitty => {
                #[cfg(feature = "kitty")]
                {
                    Ok(Box::new(KittyBackend::default()))
                }
                #[cfg(not(feature = "kitty"))]
                {
                    Err(RimgError::other(
                        "kitty backend requested but support was not compiled in",
                    ))
                }
            }
            BackendKind::Iterm2 => {
                #[cfg(feature = "iterm2")]
                {
                    Ok(Box::new(ITerm2Backend::default()))
                }
                #[cfg(not(feature = "iterm2"))]
                {
                    Err(RimgError::other(
                        "iterm2 backend requested but support was not compiled in",
                    ))
                }
            }
            BackendKind::Sixel => {
                #[cfg(feature = "sixel")]
                {
                    Ok(Box::new(SixelBackend::default()))
                }
                #[cfg(not(feature = "sixel"))]
                {
                    Err(RimgError::other(
                        "sixel backend requested but support was not compiled in",
                    ))
                }
            }
        }
    }

    pub fn auto_from_guess(guess: config::BackendKind) -> Box<dyn Backend + Send + Sync> {
        match Self::build(guess) {
            Ok(backend) => backend,
            Err(_) => Box::new(UnicodeBackend::default()),
        }
    }
}
