use image::{DynamicImage, Rgba, RgbaImage};
use timg_rust::RotationMode;

#[test]
fn test_rotation_mode_default() {
    let mode = RotationMode::default();
    assert_eq!(mode, RotationMode::Exif);
}

#[test]
fn test_rotation_mode_from_str() {
    use std::str::FromStr;

    assert_eq!(RotationMode::from_str("exif").unwrap(), RotationMode::Exif);
    assert_eq!(RotationMode::from_str("off").unwrap(), RotationMode::Off);
    assert_eq!(RotationMode::from_str("EXIF").unwrap(), RotationMode::Exif);
    assert!(RotationMode::from_str("invalid").is_err());
}

// Helper function to create a test image with a distinct pattern
fn create_test_image() -> DynamicImage {
    let mut img = RgbaImage::new(4, 4);

    // Create a pattern: red in top-left, green in top-right, blue in bottom-left, yellow in bottom-right
    for y in 0..4 {
        for x in 0..4 {
            let pixel = if x < 2 && y < 2 {
                Rgba([255, 0, 0, 255]) // Red
            } else if x >= 2 && y < 2 {
                Rgba([0, 255, 0, 255]) // Green
            } else if x < 2 && y >= 2 {
                Rgba([0, 0, 255, 255]) // Blue
            } else {
                Rgba([255, 255, 0, 255]) // Yellow
            };
            img.put_pixel(x, y, pixel);
        }
    }

    DynamicImage::ImageRgba8(img)
}

#[test]
fn test_image_orientation_normal() {
    let img = create_test_image();
    let rgba = img.to_rgba8();

    // Top-left should be red
    assert_eq!(rgba.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
    // Top-right should be green
    assert_eq!(rgba.get_pixel(3, 0), &Rgba([0, 255, 0, 255]));
}

#[test]
fn test_rotation_mode_parsing() {
    use std::str::FromStr;

    let modes = vec![
        ("exif", RotationMode::Exif),
        ("off", RotationMode::Off),
        ("EXIF", RotationMode::Exif),
        ("OFF", RotationMode::Off),
    ];

    for (input, expected) in modes {
        let parsed = RotationMode::from_str(input).unwrap();
        assert_eq!(parsed, expected, "Failed to parse '{}'", input);
    }
}
