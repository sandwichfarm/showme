use std::env;
use std::sync::Mutex;
use timg_rust::{BackendKind, detect_terminal_backend, detect_terminal_name, is_in_multiplexer};
use timg_rust::capabilities::BackendConfidence;

// Mutex to serialize tests that modify environment variables
// This prevents race conditions when tests run in parallel
static ENV_LOCK: Mutex<()> = Mutex::new(());

// Helper to set environment variable for a test
struct EnvGuard {
    key: String,
    old_value: Option<String>,
}

impl EnvGuard {
    fn set(key: &str, value: &str) -> Self {
        let old_value = env::var(key).ok();
        unsafe {
            env::set_var(key, value);
        }
        Self {
            key: key.to_string(),
            old_value,
        }
    }

    fn remove(key: &str) -> Self {
        let old_value = env::var(key).ok();
        unsafe {
            env::remove_var(key);
        }
        Self {
            key: key.to_string(),
            old_value,
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(ref value) = self.old_value {
                env::set_var(&self.key, value);
            } else {
                env::remove_var(&self.key);
            }
        }
    }
}

#[test]
fn test_detect_kitty_by_term() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard2 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard3 = EnvGuard::set("TERM", "xterm-kitty");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Kitty);
    assert_eq!(guess.confidence, BackendConfidence::Certain);
}

#[test]
fn test_detect_kitty_by_window_id() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::set("KITTY_WINDOW_ID", "1");
    let _guard2 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard3 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Kitty);
    assert_eq!(guess.confidence, BackendConfidence::Certain);
}

#[test]
fn test_detect_ghostty() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard2 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard3 = EnvGuard::set("TERM", "xterm-ghostty");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Kitty);
}

#[test]
fn test_detect_iterm2() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::set("ITERM_SESSION_ID", "w0t0p0:12345678");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::remove("TERM_PROGRAM");
    let _guard4 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Iterm2);
    assert_eq!(guess.confidence, BackendConfidence::Certain);
}

#[test]
fn test_detect_iterm2_by_term_program() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::set("TERM_PROGRAM", "iTerm.app");
    let _guard4 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Iterm2);
    assert_eq!(guess.confidence, BackendConfidence::Certain);
}

#[test]
fn test_detect_vscode() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::set("TERM_PROGRAM", "vscode");
    let _guard4 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Iterm2);
    assert_eq!(guess.confidence, BackendConfidence::Likely);
}

#[test]
fn test_detect_wezterm() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::set("TERM_PROGRAM", "WezTerm");
    let _guard4 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Iterm2);
}

#[test]
fn test_detect_sixel_by_term() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::remove("TERM_PROGRAM");
    let _guard4 = EnvGuard::set("TERM", "xterm-sixel");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Sixel);
}

#[test]
fn test_detect_mlterm() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::remove("TERM_PROGRAM");
    let _guard4 = EnvGuard::set("TERM", "mlterm");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Sixel);
}

#[test]
fn test_detect_windows_terminal() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::set("WT_SESSION", "abc123");
    let _guard4 = EnvGuard::remove("TERM");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Sixel);
}

#[test]
fn test_fallback_to_unicode() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("ITERM_SESSION_ID");
    let _guard2 = EnvGuard::remove("KITTY_WINDOW_ID");
    let _guard3 = EnvGuard::remove("TERM_PROGRAM");
    let _guard4 = EnvGuard::remove("WT_SESSION");
    let _guard5 = EnvGuard::set("TERM", "xterm-256color");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Unicode);
    assert_eq!(guess.confidence, BackendConfidence::Fallback);
}

#[test]
fn test_detect_tmux() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::set("TMUX", "/tmp/tmux-1000/default,1234,0");
    assert!(is_in_multiplexer());
}

#[test]
fn test_detect_screen() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("TMUX");
    let _guard2 = EnvGuard::set("STY", "1234.pts-0.hostname");
    assert!(is_in_multiplexer());
}

#[test]
fn test_not_in_multiplexer() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard1 = EnvGuard::remove("TMUX");
    let _guard2 = EnvGuard::remove("STY");
    assert!(!is_in_multiplexer());
}

#[test]
fn test_detect_terminal_name() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::set("TERM_PROGRAM", "iTerm.app");
    let name = detect_terminal_name();
    assert_eq!(name, Some("iTerm.app".to_string()));
}

#[test]
fn test_backend_priority() {
    let _lock = ENV_LOCK.lock().unwrap();
    // Kitty should take priority over iTerm2
    let _guard1 = EnvGuard::set("KITTY_WINDOW_ID", "1");
    let _guard2 = EnvGuard::set("ITERM_SESSION_ID", "w0t0p0:12345678");

    let guess = detect_terminal_backend();
    assert_eq!(guess.backend, BackendKind::Kitty);
}
