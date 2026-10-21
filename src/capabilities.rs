use std::env;

use std::io::IsTerminal;

use crate::config::BackendKind;
use crate::error::{Result, RimgError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendConfidence {
    Certain,
    Likely,
    Fallback,
}

#[derive(Debug, Clone)]
pub struct TerminalBackendGuess {
    pub backend: BackendKind,
    pub confidence: BackendConfidence,
    pub rationale: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
    pub width_pixels: Option<u16>,
    pub height_pixels: Option<u16>,
}

impl TerminalSize {
    /// Calculate the aspect ratio of a character cell (width/height)
    /// Returns None if pixel dimensions are unavailable
    pub fn cell_aspect_ratio(&self) -> Option<f32> {
        match (self.width_pixels, self.height_pixels) {
            (Some(w_px), Some(h_px)) if self.columns > 0 && self.rows > 0 => {
                let cell_width = w_px as f32 / self.columns as f32;
                let cell_height = h_px as f32 / self.rows as f32;
                Some(cell_width / cell_height)
            }
            _ => None,
        }
    }

    /// Calculate the recommended width_stretch factor for proper aspect ratios
    /// Returns 2.0 if pixel dimensions are unavailable (assumes 2:1 height:width cells)
    pub fn recommended_width_stretch(&self) -> f32 {
        // If we can detect aspect ratio, use it
        // Otherwise, use a reasonable default for common terminals (2.0)
        self.cell_aspect_ratio()
            .map(|ratio| {
                // Character cells are typically 2:1 (height:width) ratio (ratio = 0.5)
                // We need to expand width to compensate: 1.0 / ratio
                // If ratio is 0.5 (cell is 2x taller than wide), stretch = 2.0
                1.0 / ratio
            })
            .unwrap_or(2.0) // Default assumption: chars are ~2x as tall as wide, so stretch by 2x
    }
}

pub fn ensure_tty_stdout() -> Result<()> {
    if std::io::stdout().is_terminal() {
        Ok(())
    } else {
        Err(RimgError::StdoutNotTty)
    }
}

/// Try to query terminal pixel dimensions using XTWINOPS (CSI 16 t)
/// Returns (width_pixels, height_pixels) if supported
#[cfg(unix)]
fn query_terminal_pixel_size() -> Option<(u16, u16)> {
    use std::io::{self, Write};
    use std::os::fd::AsRawFd;
    use std::time::Duration;

    // Only try this if stdout is a TTY
    if !io::stdout().is_terminal() {
        return None;
    }
    // If stdin is not a TTY (e.g. `cat image.png | showme -`), reading from stdin for terminal
    // replies will consume the piped image bytes and break stdin decoding.
    if !io::stdin().is_terminal() {
        return None;
    }

    // Try to query character cell size in pixels: CSI 16 t
    // Response format: CSI 6 ; height ; width t
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let stdin = stdin.lock();

    // Preserve a caller's existing raw-mode session.
    let was_raw = crossterm::terminal::is_raw_mode_enabled().ok()?;
    if !was_raw {
        crossterm::terminal::enable_raw_mode().ok()?;
    }

    let result = (|| {
        // Send query
        write!(stdout, "\x1b[16t").ok()?;
        stdout.flush().ok()?;

        // Read response with timeout.
        //
        // Note: `stdin.read()` is blocking, so we must poll for readability first.
        let mut response = Vec::new();
        let mut buf = [0u8; 1];
        let start = std::time::Instant::now();

        const MAX_RESPONSE_LEN: usize = 50;
        let timeout = Duration::from_millis(100);

        while start.elapsed() < timeout && response.len() < MAX_RESPONSE_LEN {
            // Wait up to the remaining time for stdin to become readable.
            #[cfg(unix)]
            {
                let remaining = timeout.saturating_sub(start.elapsed());
                let timeout_ms: i32 = remaining.as_millis().min(i32::MAX as u128) as i32;
                let mut fds = libc::pollfd {
                    fd: stdin.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };

                let ready = unsafe { libc::poll(&mut fds as *mut _, 1, timeout_ms) };
                if ready <= 0 {
                    break;
                }
                if (fds.revents & libc::POLLIN) == 0 {
                    break;
                }
            }

            // Read the same unbuffered descriptor that poll checked. StdinLock's
            // buffer can prefetch the whole reply, leaving poll waiting while
            // the remaining bytes sit in that user-space buffer.
            // SAFETY: the live stdin descriptor and one-byte writable buffer are valid.
            if unsafe { libc::read(stdin.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) } != 1 {
                break;
            }

            response.push(buf[0]);

            // Check if we have a complete response
            if buf[0] == b't' && response.len() > 5 {
                break;
            }
        }

        // Parse response: ESC [ 6 ; height ; width t
        let response_str = String::from_utf8_lossy(&response);
        if response_str.starts_with("\x1b[6;") && response_str.ends_with('t') {
            let parts: Vec<&str> = response_str[4..response_str.len() - 1].split(';').collect();

            if parts.len() >= 2 {
                let height = parts[0].parse::<u16>().ok()?;
                let width = parts[1].parse::<u16>().ok()?;
                return Some((width, height));
            }
        }

        None
    })();

    // Restore terminal mode
    if !was_raw {
        let _ = crossterm::terminal::disable_raw_mode();
    }

    result
}

#[cfg(not(unix))]
fn query_terminal_pixel_size() -> Option<(u16, u16)> {
    None
}

pub fn current_terminal_size() -> TerminalSize {
    let (columns, rows) = crossterm::terminal::size().unwrap_or((80, 24));

    // CSI 16 t returns one character cell's dimensions, not the whole window.
    let (width_pixels, height_pixels) = query_terminal_pixel_size()
        .and_then(|(width, height)| {
            let ratio = f32::from(width) / f32::from(height);
            if !(0.3..0.7).contains(&ratio) {
                return None;
            }
            Some((width.checked_mul(columns)?, height.checked_mul(rows)?))
        })
        .map(|(width, height)| (Some(width), Some(height)))
        .unwrap_or((None, None));

    TerminalSize {
        columns,
        rows,
        width_pixels,
        height_pixels,
    }
}

pub fn detect_terminal_backend() -> TerminalBackendGuess {
    // Check for Kitty and Ghostty (both use Kitty graphics protocol)
    // Kitty sets KITTY_WINDOW_ID, Ghostty sets TERM=xterm-ghostty
    if let Ok(term) = env::var("TERM") {
        let term_lower = term.to_ascii_lowercase();
        if term_lower.contains("kitty")
            || term_lower.contains("ghostty")
            || term == "xterm-kitty"
            || term == "xterm-ghostty"
        {
            return TerminalBackendGuess {
                backend: BackendKind::Kitty,
                confidence: BackendConfidence::Certain,
                rationale: "TERM indicates Kitty graphics protocol support",
            };
        }
    }

    if env::var("KITTY_WINDOW_ID").is_ok() {
        return TerminalBackendGuess {
            backend: BackendKind::Kitty,
            confidence: BackendConfidence::Certain,
            rationale: "detected Kitty terminal (KITTY_WINDOW_ID)",
        };
    }

    // Check for iTerm2 and terminals using iTerm2 protocol
    if env::var("ITERM_SESSION_ID").is_ok() {
        return TerminalBackendGuess {
            backend: BackendKind::Iterm2,
            confidence: BackendConfidence::Certain,
            rationale: "detected iTerm2 (ITERM_SESSION_ID)",
        };
    }

    if let Ok(program) = env::var("TERM_PROGRAM") {
        match program.as_str() {
            "iTerm.app" => {
                return TerminalBackendGuess {
                    backend: BackendKind::Iterm2,
                    confidence: BackendConfidence::Certain,
                    rationale: "detected iTerm2 (TERM_PROGRAM)",
                };
            }
            "vscode" => {
                // VSCode terminal supports iTerm2 inline images
                return TerminalBackendGuess {
                    backend: BackendKind::Iterm2,
                    confidence: BackendConfidence::Likely,
                    rationale: "VSCode terminal supports iTerm2 protocol",
                };
            }
            "WezTerm" => {
                // WezTerm supports iTerm2 protocol
                return TerminalBackendGuess {
                    backend: BackendKind::Iterm2,
                    confidence: BackendConfidence::Likely,
                    rationale: "WezTerm supports iTerm2 protocol",
                };
            }
            _ => {}
        }
    }

    // Check for Sixel support
    // Some terminals set TERM with -sixel suffix
    if let Ok(term) = env::var("TERM") {
        if term.contains("-sixel") || term.contains("sixel") {
            return TerminalBackendGuess {
                backend: BackendKind::Sixel,
                confidence: BackendConfidence::Likely,
                rationale: "TERM indicates Sixel support",
            };
        }

        // mlterm is known to support sixel
        if term.contains("mlterm") {
            return TerminalBackendGuess {
                backend: BackendKind::Sixel,
                confidence: BackendConfidence::Likely,
                rationale: "mlterm supports Sixel",
            };
        }
    }

    // Check for Windows Terminal (supports Sixel as of 1.22)
    if let Ok(program) = env::var("TERM_PROGRAM")
        && (program.contains("WindowsTerminal") || program.contains("Microsoft.WindowsTerminal"))
    {
        return TerminalBackendGuess {
            backend: BackendKind::Sixel,
            confidence: BackendConfidence::Likely,
            rationale: "Windows Terminal supports Sixel",
        };
    }

    if env::var("WT_SESSION").is_ok() || env::var("WT_PROFILE_ID").is_ok() {
        return TerminalBackendGuess {
            backend: BackendKind::Sixel,
            confidence: BackendConfidence::Likely,
            rationale: "Windows Terminal detected (WT_SESSION)",
        };
    }

    // Fallback to Unicode for unknown or basic terminals
    TerminalBackendGuess {
        backend: BackendKind::Unicode,
        confidence: BackendConfidence::Fallback,
        rationale: "no graphics protocol detected, using Unicode blocks",
    }
}

/// Check if running inside tmux or screen
pub fn is_in_multiplexer() -> bool {
    env::var("TMUX").is_ok() || env::var("STY").is_ok()
}

/// Get the name of the detected terminal emulator
pub fn detect_terminal_name() -> Option<String> {
    // Try TERM_PROGRAM first (most specific)
    if let Ok(program) = env::var("TERM_PROGRAM") {
        return Some(program);
    }

    // Try TERM
    if let Ok(term) = env::var("TERM") {
        return Some(term);
    }

    None
}
