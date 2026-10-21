use assert_cmd::Command;
use predicates::prelude::*;

mod common;
use common::TestDirectory;

fn fixture() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red.ppm")
}

#[test]
fn help_displays_usage() {
    let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("showme"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Terminal image viewer"));
}

#[test]
fn file_output_works_without_a_terminal_and_keeps_stdout_empty() {
    let directory = TestDirectory::new();
    let output = directory.path().join("output.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .env("TERM", "xterm-kitty")
        .args(["--quiet", "--alternate-screen", "--output"])
        .arg(&output)
        .arg(fixture())
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    let rendered = std::fs::read_to_string(output).unwrap();
    assert!(rendered.contains('▀') || rendered.contains('█'));
    for control in [
        "\x1b[?25l",
        "\x1b[?25h",
        "\x1b[?1049h",
        "\x1b[?1049l",
        "\x1b[16t",
    ] {
        assert!(
            !rendered.contains(control),
            "unexpected terminal control: {control:?}"
        );
    }
}

#[test]
fn unwritable_destination_reports_an_error_without_stdout_controls() {
    let directory = TestDirectory::new();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--alternate-screen", "--output"])
        .arg(directory.path())
        .arg(fixture())
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("failed to create output file"));
}

#[test]
fn captured_stdout_still_requires_explicit_file_output() {
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .arg(fixture())
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("standard output is not a tty"));
}

#[test]
fn force_image_decodes_a_misnamed_file() {
    let directory = TestDirectory::new();
    let input = directory.path().join("photo.mp4");
    std::fs::copy(fixture(), &input).unwrap();
    let output = directory.path().join("rendered.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--force-image", "-o"])
        .arg(&output)
        .arg(input)
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(!std::fs::read(output).unwrap().is_empty());
}

#[cfg(not(feature = "video"))]
#[test]
fn force_video_reports_a_disabled_feature() {
    let directory = TestDirectory::new();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--force-video", "-o"])
        .arg(directory.path().join("output.txt"))
        .arg(fixture())
        .assert()
        .failure()
        .stderr(predicate::str::contains("video feature is disabled"));
}

#[test]
fn stdin_gif_and_file_gif_apply_the_same_cropping() {
    use image::{Delay, Frame, Rgba, RgbaImage, codecs::gif::GifEncoder};
    let directory = TestDirectory::new();
    let pixels = RgbaImage::from_fn(6, 6, |x, y| {
        if (2..4).contains(&x) && (2..4).contains(&y) {
            Rgba([255, 0, 0, 255])
        } else {
            Rgba([0, 0, 0, 255])
        }
    });
    let mut gif = Vec::new();
    GifEncoder::new(&mut gif)
        .encode_frame(Frame::from_parts(
            pixels,
            0,
            0,
            Delay::from_numer_denom_ms(20, 1),
        ))
        .unwrap();
    let input = directory.path().join("border.gif");
    std::fs::write(&input, &gif).unwrap();
    let file_output = directory.path().join("file.txt");
    let stdin_output = directory.path().join("stdin.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--crop-border", "1", "--auto-crop", "-o"])
        .arg(&file_output)
        .arg(input)
        .assert()
        .success();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--crop-border", "1", "--auto-crop", "-o"])
        .arg(&stdin_output)
        .arg("-")
        .write_stdin(gif)
        .timeout(std::time::Duration::from_secs(5))
        .assert()
        .success();
    assert_eq!(
        std::fs::read(file_output).unwrap(),
        std::fs::read(stdin_output).unwrap()
    );
}

#[test]
fn scrolling_with_infinite_loop_obeys_duration_in_a_real_process() {
    let directory = TestDirectory::new();
    let input = directory.path().join("tall.ppm");
    let mut ppm = String::from("P3\n2 20\n255\n");
    for _ in 0..40 {
        ppm.push_str("255 0 0 ");
    }
    std::fs::write(&input, ppm).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args([
            "--quiet",
            "--loop",
            "--scroll",
            "--delta-move",
            "-1:1",
            "--duration",
            "20ms",
            "--fit-width",
            "-w",
            "2",
            "-H",
            "1",
            "-o",
        ])
        .arg(directory.path().join("scroll.txt"))
        .arg(input)
        .timeout(std::time::Duration::from_secs(3))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn frame_limited_gif_preview_does_not_decode_a_corrupt_later_frame() {
    let directory = TestDirectory::new();
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder
            .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                2,
                2,
                image::Rgba([255, 0, 0, 255]),
            )))
            .unwrap();
    }
    assert_eq!(bytes.pop(), Some(0x3b)); // Replace the trailer with a truncated second image.
    bytes.extend_from_slice(b"\x2c\x00\x00");
    let input = directory.path().join("partial.gif");
    std::fs::write(&input, bytes).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--frames", "1", "-o"])
        .arg(directory.path().join("preview.txt"))
        .arg(&input)
        .timeout(std::time::Duration::from_secs(3))
        .assert()
        .success();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "-o"])
        .arg(directory.path().join("full.txt"))
        .arg(&input)
        .timeout(std::time::Duration::from_secs(3))
        .assert()
        .failure();
}

#[cfg(feature = "video")]
#[test]
fn forced_video_decodes_an_extensionless_preview() {
    let directory = TestDirectory::new();
    let input = directory.path().join("video-without-extension");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/short.mkv"),
        &input,
    )
    .unwrap();
    let output = directory.path().join("preview.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--force-video", "--frames", "1", "-o"])
        .arg(&output)
        .arg(input)
        .timeout(std::time::Duration::from_secs(3))
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
    assert!(!std::fs::read(output).unwrap().is_empty());
}

#[test]
fn invalid_numeric_and_color_values_report_errors_instead_of_panicking() {
    let directory = TestDirectory::new();
    for (option, value) in [
        ("--duration", "NaN"),
        ("--duration", "inf"),
        ("--duration", "1e100"),
        ("--duration", "NaNms"),
        ("--duration", "1e100ms"),
        ("--width-stretch", "NaN"),
        ("--width-stretch", "inf"),
        ("--width", "0"),
        ("--height", "0"),
        ("--threads", "0"),
        ("--loops", "-2"),
        ("--grid", "2x2x2"),
        ("--delta-move", "1:2:3"),
        ("--background", "#0é000"),
    ] {
        Command::new(assert_cmd::cargo::cargo_bin!("showme"))
            .args([option, value, "-o"])
            .arg(directory.path().join("output.txt"))
            .arg(fixture())
            .timeout(std::time::Duration::from_secs(3))
            .assert()
            .failure()
            .code(1)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("panicked").not());
    }
}

#[test]
fn zero_playback_limits_and_offset_past_eof_produce_empty_files() {
    let directory = TestDirectory::new();
    for (option, value) in [
        ("--frames", "0"),
        ("--loops", "0"),
        ("--duration", "0s"),
        ("--frame-offset", "99"),
    ] {
        let output = directory.path().join(format!("{option}.txt"));
        Command::new(assert_cmd::cargo::cargo_bin!("showme"))
            .args(["--quiet", option, value, "-o"])
            .arg(&output)
            .arg(fixture())
            .timeout(std::time::Duration::from_secs(3))
            .assert()
            .success();
        assert_eq!(std::fs::metadata(output).unwrap().len(), 0);
    }
}

#[test]
fn conflicting_layout_and_decoder_options_are_rejected() {
    for arguments in [
        ["--fit-width", "--fit-height"],
        ["--force-image", "--force-video"],
        ["--scroll", "--grid=2"],
    ] {
        Command::new(assert_cmd::cargo::cargo_bin!("showme"))
            .args(arguments)
            .arg(fixture())
            .assert()
            .failure()
            .code(2);
    }
}

#[cfg(not(feature = "sixel"))]
#[test]
fn an_explicit_disabled_backend_reports_an_error() {
    let directory = TestDirectory::new();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--backend", "sixel", "-o"])
        .arg(directory.path().join("output.txt"))
        .arg(fixture())
        .assert()
        .failure()
        .stderr(predicate::str::contains("not compiled in"));
}

#[test]
fn grid_row_limit_does_not_decode_inputs_outside_the_requested_grid() {
    let directory = TestDirectory::new();
    let output = directory.path().join("grid.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--grid", "2x1", "-o"])
        .arg(&output)
        .arg(fixture())
        .arg(fixture())
        .arg(directory.path().join("must-not-be-loaded.png"))
        .assert()
        .success();
    assert!(!std::fs::read(output).unwrap().is_empty());
}

#[test]
fn grid_offset_past_eof_emits_no_frames() {
    let directory = TestDirectory::new();
    let output = directory.path().join("grid.txt");
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--grid", "2", "--frame-offset", "99", "-o"])
        .arg(&output)
        .arg(fixture())
        .assert()
        .success();
    assert_eq!(std::fs::metadata(output).unwrap().len(), 0);
}

#[cfg(feature = "video")]
#[test]
fn force_image_does_not_fall_back_to_video_decoding() {
    let directory = TestDirectory::new();
    Command::new(assert_cmd::cargo::cargo_bin!("showme"))
        .args(["--quiet", "--force-image", "-o"])
        .arg(directory.path().join("output.txt"))
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/short.mkv"))
        .timeout(std::time::Duration::from_secs(3))
        .assert()
        .failure();
}
