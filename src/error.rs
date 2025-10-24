use std::path::PathBuf;

use thiserror::Error;

/// Convenient result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, RimgError>;

/// Errors that can surface while running the terminal image viewer.
#[derive(Debug, Error)]
pub enum RimgError {
    #[error("standard output is not a tty; refusing to emit escape sequences")]
    StdoutNotTty,

    #[error("no input paths provided")]
    MissingInput,

    #[error("failed to open {path}: {source}")]
    ImageOpen {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error("failed to decode frames in {path}: {source}")]
    FrameDecode {
        path: PathBuf,
        #[source]
        source: image::ImageError,
    },

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl RimgError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
