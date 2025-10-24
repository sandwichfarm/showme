pub mod autocrop;
pub mod backend;
pub mod capabilities;
pub mod cli;
pub mod color_quantize;
pub mod config;
pub mod error;
pub mod image;
pub mod renderer;
pub mod tmux;
#[cfg(feature = "video")]
pub mod video;

pub use backend::BackendFactory;
pub use capabilities::{TerminalBackendGuess, TerminalSize, detect_terminal_backend, detect_terminal_name, is_in_multiplexer};
pub use cli::Cli;
pub use config::{BackendKind, Config, GridOptions, PixelationMode, RenderSizing, RotationMode};
pub use error::{Result, RimgError};
pub use renderer::Renderer;
#[cfg(feature = "video")]
pub use video::VideoLoader;
