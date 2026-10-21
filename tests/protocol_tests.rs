#![cfg(any(feature = "kitty", feature = "iterm2"))]

use base64::Engine;
use image::{Rgba, RgbaImage};

mod common;
use common::TestDirectory;

fn render(backend: &str, width: u32, height: u32) -> (String, RgbaImage) {
    let directory = TestDirectory::new();
    let input = directory.path().join("noise.png");
    let output = directory.path().join("protocol.txt");
    let mut state = 0x12345678_u32;
    let pixels = RgbaImage::from_fn(width, height, |_, _| {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        Rgba(state.to_le_bytes())
    });
    pixels.save(&input).unwrap();
    assert_cmd::Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .env_remove("TMUX")
        .env_remove("STY")
        .env("TERM", "xterm-256color")
        .args([
            "--quiet",
            "--backend",
            backend,
            "--width",
            "8",
            "--height",
            "4",
            "-o",
        ])
        .arg(&output)
        .arg(input)
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success()
        .stdout("");
    (std::fs::read_to_string(output).unwrap(), pixels)
}

fn assert_png(payload: &str, expected: &RgbaImage) -> usize {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .unwrap();
    assert_eq!(
        image::guess_format(&bytes).unwrap(),
        image::ImageFormat::Png
    );
    let decoded = image::load_from_memory(&bytes).unwrap().into_rgba8();
    assert_eq!(decoded.dimensions(), expected.dimensions());
    assert!(
        decoded == *expected,
        "protocol PNG changed source pixels or alpha"
    );
    bytes.len()
}

#[cfg(feature = "kitty")]
#[test]
fn kitty_chunks_reassemble_to_the_original_png_without_intervening_newlines() {
    let (stream, pixels) = render("kitty", 128, 128);
    assert_eq!(stream.matches('\n').count(), 1);
    let commands: Vec<_> = stream
        .strip_suffix('\n')
        .unwrap()
        .split("\x1b\\")
        .filter(|command| !command.is_empty())
        .collect();
    assert!(commands.len() > 1);
    let mut payload = String::new();
    for (index, command) in commands.iter().enumerate() {
        let command = command.strip_prefix("\x1b_G").unwrap();
        let (arguments, chunk) = command.split_once(';').unwrap();
        if index == 0 {
            for required in ["a=T", "f=100", "s=128", "v=128", "c=8", "r=4"] {
                assert!(arguments.split(',').any(|argument| argument == required));
            }
        }
        assert_eq!(
            arguments.split(',').any(|argument| argument == "m=1"),
            index + 1 < commands.len()
        );
        assert!(chunk.len() <= 4096);
        payload.push_str(chunk);
    }
    assert_png(&payload, &pixels);
}

#[cfg(feature = "iterm2")]
#[test]
fn iterm_file_contains_one_complete_png_and_correct_size() {
    let (stream, pixels) = render("iterm2", 32, 32);
    let command = stream.strip_suffix("\x07\n").unwrap();
    let (header, payload) = command.split_once(':').unwrap();
    assert!(header.starts_with("\x1b]1337;File=inline=1;"));
    assert!(!header.contains(";m="));
    assert_eq!(stream.matches("\x1b]1337;").count(), 1);
    let size = header
        .split(';')
        .find_map(|argument| argument.strip_prefix("size="))
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert_eq!(size, assert_png(payload, &pixels));
}

#[cfg(feature = "iterm2")]
#[test]
fn large_iterm_images_use_documented_multipart_file_framing() {
    let (stream, pixels) = render("iterm2", 1024, 512);
    assert_eq!(stream.matches('\n').count(), 1);
    let commands: Vec<_> = stream
        .strip_suffix('\n')
        .unwrap()
        .split('\x07')
        .filter(|command| !command.is_empty())
        .collect();
    assert!(
        commands.len() > 3,
        "fixture must exercise multipart transfer"
    );
    assert!(commands[0].starts_with("\x1b]1337;MultipartFile=inline=1;"));
    assert_eq!(commands.last().copied(), Some("\x1b]1337;FileEnd"));
    let mut payload = String::new();
    for command in &commands[1..commands.len() - 1] {
        let chunk = command.strip_prefix("\x1b]1337;FilePart=").unwrap();
        assert!(chunk.len() + 32 <= 1024 * 1024);
        payload.push_str(chunk);
    }
    assert_png(&payload, &pixels);
}
