use image::{Delay, DynamicImage, Rgba, RgbaImage, codecs::gif::GifEncoder};
use showme::{RotationMode, image::load_image};
use std::path::{Path, PathBuf};
use std::time::Duration;

mod common;
use common::TestDirectory;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn png_preserves_dimensions_pixels_and_alpha() {
    let directory = TestDirectory::new();
    let path = directory.path().join("alpha.png");
    let pixels = RgbaImage::from_fn(3, 2, |x, y| {
        Rgba([(x * 60) as u8, (y * 90) as u8, 200, (x * 100) as u8])
    });
    pixels.save(&path).unwrap();
    let loaded = load_image(&path, RotationMode::Off, false, 0).unwrap();
    assert_eq!(loaded.frames.len(), 1);
    assert_eq!(loaded.frames[0].pixels, pixels);
}

#[test]
fn jpeg_decodes_dimensions_and_color() {
    let directory = TestDirectory::new();
    let path = directory.path().join("red.jpg");
    DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 2, Rgba([255, 0, 0, 255])))
        .to_rgb8()
        .save(&path)
        .unwrap();
    let loaded = load_image(&path, RotationMode::Exif, false, 0).unwrap();
    assert_eq!(loaded.frames[0].pixels.dimensions(), (4, 2));
    for pixel in loaded.frames[0].pixels.pixels() {
        assert!(pixel[0] >= 250 && pixel[1] <= 5 && pixel[2] <= 5 && pixel[3] == 255);
    }
}

#[test]
fn gif_decodes_frame_order_pixels_and_timing() {
    let directory = TestDirectory::new();
    let path = directory.path().join("colors.gif");
    let mut encoder = GifEncoder::new(std::fs::File::create(&path).unwrap());
    for (color, delay) in [([255, 0, 0, 255], 20), ([0, 0, 255, 255], 40)] {
        encoder
            .encode_frame(image::Frame::from_parts(
                RgbaImage::from_pixel(2, 2, Rgba(color)),
                0,
                0,
                Delay::from_numer_denom_ms(delay, 1),
            ))
            .unwrap();
    }
    drop(encoder);
    let loaded = load_image(&path, RotationMode::Off, false, 0).unwrap();
    assert_eq!(loaded.frames.len(), 2);
    assert_eq!(loaded.frames[0].pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(loaded.frames[1].pixels.get_pixel(0, 0).0, [0, 0, 255, 255]);
    assert_eq!(loaded.frames[0].delay, Duration::from_millis(20));
    assert_eq!(loaded.frames[1].delay, Duration::from_millis(40));
}

#[test]
fn corrupt_and_truncated_images_return_errors() {
    let directory = TestDirectory::new();
    for (name, data) in [
        ("bad.png", b"not a PNG".as_slice()),
        ("truncated.gif", b"GIF89a\x02\x00".as_slice()),
        ("empty.jpg", b"".as_slice()),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, data).unwrap();
        assert!(
            load_image(&path, RotationMode::Off, false, 0).is_err(),
            "{name}"
        );
    }
}

#[cfg(feature = "qoi")]
#[test]
fn qoi_decodes_pixels_and_alpha() {
    let loaded = load_image(&fixture("colors.qoi"), RotationMode::Off, false, 0).unwrap();
    assert_eq!(loaded.frames[0].pixels.dimensions(), (2, 1));
    assert_eq!(loaded.frames[0].pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(loaded.frames[0].pixels.get_pixel(1, 0).0, [0, 0, 255, 128]);
}

#[cfg(feature = "qoi")]
#[test]
fn qoi_dimensions_are_rejected_before_pixel_allocation() {
    let directory = TestDirectory::new();
    let path = directory.path().join("oversized.qoi");
    let mut bytes = std::fs::read(fixture("colors.qoi")).unwrap();
    bytes[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!(
        load_image(&path, RotationMode::Off, false, 0)
            .unwrap_err()
            .to_string()
            .contains("dimensions exceed")
    );
}

#[cfg(feature = "svg")]
#[test]
fn svg_and_svgz_render_matching_pixels() {
    let svg = load_image(&fixture("colors.svg"), RotationMode::Off, false, 0).unwrap();
    let svgz = load_image(&fixture("colors.svgz"), RotationMode::Off, false, 0).unwrap();
    let pixels = &svg.frames[0].pixels;
    assert_eq!(pixels.dimensions(), (800, 400));
    assert_eq!(pixels.get_pixel(100, 100).0, [255, 0, 0, 255]);
    assert_eq!(pixels.get_pixel(700, 100).0, [0, 0, 255, 255]);
    assert_eq!(pixels, &svgz.frames[0].pixels);
}

#[cfg(feature = "video")]
#[test]
fn video_decodes_real_frames_pixels_and_timing() {
    let loaded = load_image(&fixture("short.mkv"), RotationMode::Off, false, 0).unwrap();
    assert_eq!(loaded.frames.len(), 3);
    for frame in loaded.frames {
        assert_eq!(frame.pixels.dimensions(), (4, 2));
        let pixel = frame.pixels.get_pixel(0, 0);
        assert!(pixel[0] > 240 && pixel[1] < 10 && pixel[2] < 10 && pixel[3] == 255);
        assert_eq!(frame.delay, Duration::from_millis(200));
    }
}

#[cfg(feature = "pdf")]
#[test]
#[ignore = "requires the checksum-verified PDFium 7881 runtime; see scripts/fetch_pdfium.py"]
fn pdf_decodes_page_order_dimensions_and_colors() {
    let loaded = load_image(&fixture("two-pages.pdf"), RotationMode::Off, false, 0).unwrap();
    assert_eq!(loaded.frames.len(), 2);
    let first = &loaded.frames[0].pixels;
    let second = &loaded.frames[1].pixels;
    assert_eq!(first.width(), 2000);
    assert!((1333..=1334).contains(&first.height()));
    assert_eq!(first.get_pixel(100, 100).0, [255, 0, 0, 255]);
    assert_eq!(second.get_pixel(100, 100).0, [0, 0, 255, 255]);
    let repeated = load_image(&fixture("two-pages.pdf"), RotationMode::Off, false, 0).unwrap();
    assert_eq!(repeated.frames.len(), 2);
    assert_eq!(
        repeated.frames[1].pixels.get_pixel(100, 100).0,
        [0, 0, 255, 255]
    );
    let directory = TestDirectory::new();
    let corrupt = directory.path().join("corrupt.pdf");
    std::fs::write(&corrupt, b"%PDF-1.4\ntruncated").unwrap();
    let error = load_image(&corrupt, RotationMode::Off, false, 0)
        .unwrap_err()
        .to_string();
    assert!(error.contains("Failed to load PDF"), "{error}");
}

#[test]
fn disabled_media_features_have_explicit_errors() {
    for (enabled, name, feature) in [
        (cfg!(feature = "qoi"), "colors.qoi", "qoi"),
        (cfg!(feature = "svg"), "colors.svg", "svg"),
        (cfg!(feature = "pdf"), "two-pages.pdf", "pdf"),
        (cfg!(feature = "video"), "short.mkv", "video"),
    ] {
        if !enabled {
            let error = load_image(&fixture(name), RotationMode::Off, false, 0).unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("{feature} feature is disabled"))
            );
        }
    }
}

#[test]
fn corrupt_optional_formats_return_errors() {
    let directory = TestDirectory::new();
    for (enabled, name, bytes) in [
        (cfg!(feature = "qoi"), "bad.qoi", b"qoif".as_slice()),
        (cfg!(feature = "svg"), "bad.svg", b"<svg>".as_slice()),
        (
            cfg!(feature = "svg"),
            "bad.svgz",
            b"\x1f\x8btruncated".as_slice(),
        ),
        (
            cfg!(feature = "video"),
            "bad.mkv",
            b"invalid Matroska".as_slice(),
        ),
    ] {
        if enabled {
            let path = directory.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            assert!(
                load_image(&path, RotationMode::Off, false, 0).is_err(),
                "{name}"
            );
        }
    }
}

#[test]
fn malformed_dds_pitch_and_dimensions_fail_before_pixel_indexing() {
    let directory = TestDirectory::new();
    let path = directory.path().join("bad.dds");
    let mut bytes = vec![0_u8; 136];
    bytes[..4].copy_from_slice(b"DDS ");
    for (offset, value) in [
        (4, 124_u32),
        (8, 0x100f),
        (12, 2),
        (16, 2),
        (20, 4),
        (76, 32),
        (80, 0x41),
        (88, 32),
        (92, 0xff0000),
        (96, 0xff00),
        (100, 0xff),
        (104, 0xff000000),
        (108, 0x1000),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    std::fs::write(&path, &bytes).unwrap();
    assert!(load_image(&path, RotationMode::Off, false, 0).is_err());
    bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();
    assert!(
        load_image(&path, RotationMode::Off, false, 0)
            .unwrap_err()
            .to_string()
            .contains("dimensions exceed")
    );
}
