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
/// use showme::tmux::wrap_for_tmux;
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

/// Wrap an escape sequence for GNU screen's raw DCS passthrough.
///
/// Screen does not use tmux's prefix or ESC doubling. Bound complete DCS
/// strings to 256 bytes for older Screen versions and split inner ST sequences
/// between DCS strings so Screen forwards the original terminator intact.
pub fn wrap_for_screen(sequence: &str) -> String {
    let mut result = String::new();
    let mut chunk = String::new();
    for character in sequence.chars() {
        if chunk.len() + character.len_utf8() > 252
            || (character == '\\' && chunk.ends_with('\x1b'))
        {
            result.push_str("\x1bP");
            result.push_str(&chunk);
            result.push_str("\x1b\\");
            chunk.clear();
        }
        chunk.push(character);
    }
    if !chunk.is_empty() || sequence.is_empty() {
        result.push_str("\x1bP");
        result.push_str(&chunk);
        result.push_str("\x1b\\");
    }
    result
}

/// Wrap a sequence for the current multiplexer, or return it unchanged.
pub fn wrap_if_needed(sequence: &str) -> String {
    let terminal = env::var("TERM").unwrap_or_default();
    if in_tmux() || terminal.starts_with("tmux") {
        wrap_for_tmux(sequence)
    } else if env::var("STY").is_ok() || terminal.starts_with("screen") {
        wrap_for_screen(sequence)
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
        assert_eq!(
            wrapped,
            "\x1bPtmux;\x1b\x1b[1m\x1b\x1b[31m\x1b\x1b[0m\x1b\\"
        );
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
    fn screen_uses_raw_dcs_instead_of_tmux_escaping() {
        let seq = "\x1b[31mtext\x1b[0m";
        assert_eq!(wrap_for_screen(seq), format!("\x1bP{seq}\x1b\\"));
    }

    #[test]
    fn screen_preserves_long_apc_payloads_and_utf8_across_bounded_dcs_strings() {
        let sequence = format!("\x1b_Ga=T;{}é\x1b\\", "abcdefgh".repeat(800));
        let wrapped = wrap_for_screen(&sequence);
        let mut remaining = wrapped.as_str();
        let mut forwarded = String::new();
        let mut count = 0;
        while !remaining.is_empty() {
            let body = remaining.strip_prefix("\x1bP").unwrap();
            let end = body.find("\x1b\\").unwrap();
            assert!(end + 4 <= 256);
            forwarded.push_str(&body[..end]);
            remaining = &body[end + 2..];
            count += 1;
        }
        assert!(count > 1);
        assert_eq!(forwarded, sequence);
    }
}
