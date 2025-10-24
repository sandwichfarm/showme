/// Utilities for tmux/screen multiplexer passthrough support
///
/// When running inside tmux or screen, graphics escape sequences need to be
/// wrapped in DCS (Device Control String) sequences so they can pass through
/// to the underlying terminal emulator.
///
/// References:
/// - https://github.com/tmux/tmux/wiki/FAQ#what-is-the-passthrough-escape-sequence-and-how-do-i-use-it
/// - https://sw.kovidgoyal.net/kitty/graphics-protocol/#tmux-support

use std::env;

/// Check if we're running inside a multiplexer (tmux or screen)
pub fn in_multiplexer() -> bool {
    crate::capabilities::is_in_multiplexer()
}

/// Check if we're in tmux specifically
pub fn in_tmux() -> bool {
    env::var("TMUX").is_ok()
}

/// Wrap an escape sequence for tmux passthrough
///
/// Tmux requires escape sequences to be wrapped in DCS passthrough:
/// - Prefix: `\x1bPtmux;`
/// - All `\x1b` (ESC) characters must be escaped as `\x1b\x1b`
/// - Suffix: `\x1b\\`
///
/// Example:
/// ```
/// use timg_rust::tmux::wrap_for_tmux;
///
/// let seq = "\x1b_Ga=T;...\x1b\\";
/// let wrapped = wrap_for_tmux(seq);
/// // Result: "\x1bPtmux;\x1b\x1b_Ga=T;...\x1b\x1b\\\x1b\\"
/// ```
pub fn wrap_for_tmux(sequence: &str) -> String {
    let mut result = String::with_capacity(sequence.len() * 2 + 10);

    // Start DCS passthrough
    result.push_str("\x1bPtmux;");

    // Escape all ESC characters
    for ch in sequence.chars() {
        if ch == '\x1b' {
            result.push('\x1b');
            result.push('\x1b');
        } else {
            result.push(ch);
        }
    }

    // End DCS passthrough
    result.push_str("\x1b\\");

    result
}

/// Wrap an escape sequence for screen passthrough
///
/// Screen uses a similar DCS wrapping mechanism but may have slight
/// differences in implementation. Currently uses the same format as tmux.
pub fn wrap_for_screen(sequence: &str) -> String {
    // Screen uses the same DCS wrapping format as tmux
    wrap_for_tmux(sequence)
}

/// Wrap a sequence for the current multiplexer if detected
///
/// Returns the original sequence unchanged if not in a multiplexer.
pub fn wrap_if_needed(sequence: &str) -> String {
    if in_multiplexer() {
        wrap_for_tmux(sequence)
    } else {
        sequence.to_string()
    }
}

/// Enable tmux passthrough mode by setting allow-passthrough
///
/// This is required for tmux >= 3.3 to allow graphics protocols.
/// Returns true if successfully enabled, false otherwise.
pub fn enable_tmux_passthrough() -> bool {
    if !in_tmux() {
        return false;
    }

    // Try to enable tmux passthrough
    use std::process::Command;

    match Command::new("tmux")
        .args(["set", "-p", "allow-passthrough", "on"])
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                return true;
            } else if let Some(1) = output.status.code() {
                // Exit code 1 means tmux command worked but option not recognized
                eprintln!("Warning: tmux allow-passthrough not supported. Need tmux >= 3.3");
                return false;
            }
            false
        }
        Err(_) => {
            // Command failed - tmux might not be available or we're in remote session
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_simple_sequence() {
        let seq = "\x1b[31mred\x1b[0m";
        let wrapped = wrap_for_tmux(seq);
        assert_eq!(wrapped, "\x1bPtmux;\x1b\x1b[31mred\x1b\x1b[0m\x1b\\");
    }

    #[test]
    fn test_wrap_kitty_sequence() {
        let seq = "\x1b_Ga=T\x1b\\";
        let wrapped = wrap_for_tmux(seq);
        // All ESC chars should be doubled
        assert_eq!(wrapped, "\x1bPtmux;\x1b\x1b_Ga=T\x1b\x1b\\\x1b\\");
    }

    #[test]
    fn test_wrap_multiple_escapes() {
        let seq = "\x1b[1m\x1b[31m\x1b[0m";
        let wrapped = wrap_for_tmux(seq);
        assert_eq!(wrapped, "\x1bPtmux;\x1b\x1b[1m\x1b\x1b[31m\x1b\x1b[0m\x1b\\");
    }

    #[test]
    fn test_wrap_empty_sequence() {
        let seq = "";
        let wrapped = wrap_for_tmux(seq);
        assert_eq!(wrapped, "\x1bPtmux;\x1b\\");
    }

    #[test]
    fn test_wrap_no_escapes() {
        let seq = "hello";
        let wrapped = wrap_for_tmux(seq);
        assert_eq!(wrapped, "\x1bPtmux;hello\x1b\\");
    }

    #[test]
    fn test_screen_wrap_same_as_tmux() {
        let seq = "\x1b[31mtext\x1b[0m";
        assert_eq!(wrap_for_screen(seq), wrap_for_tmux(seq));
    }
}
