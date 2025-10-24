use std::env;

use atty::Stream;

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
}

pub fn ensure_tty_stdout() -> Result<()> {
    if atty::is(Stream::Stdout) {
        Ok(())
    } else {
        Err(RimgError::StdoutNotTty)
    }
}

pub fn current_terminal_size() -> TerminalSize {
    crossterm::terminal::size()
        .map(|(columns, rows)| TerminalSize { columns, rows })
        .unwrap_or(TerminalSize {
            columns: 80,
            rows: 24,
        })
}

pub fn detect_terminal_backend() -> TerminalBackendGuess {
    // Check for Kitty and Ghostty (both use Kitty graphics protocol)
    // Kitty sets KITTY_WINDOW_ID, Ghostty sets TERM=xterm-ghostty
    if let Ok(term) = env::var("TERM") {
        let term_lower = term.to_ascii_lowercase();
        if term_lower.contains("kitty") || term_lower.contains("ghostty") || term == "xterm-kitty" || term == "xterm-ghostty" {
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
    if let Ok(program) = env::var("TERM_PROGRAM") {
        if program.contains("WindowsTerminal") || program.contains("Microsoft.WindowsTerminal") {
            return TerminalBackendGuess {
                backend: BackendKind::Sixel,
                confidence: BackendConfidence::Likely,
                rationale: "Windows Terminal supports Sixel",
            };
        }
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
