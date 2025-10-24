use image::{Rgba, RgbaImage};
use timg_rust::{PixelationMode, RenderSizing, RotationMode};

#[test]
fn test_pixelation_mode_default() {
    let mode = PixelationMode::default();
    assert_eq!(mode, PixelationMode::Quarter);
}

#[test]
fn test_pixelation_mode_from_str() {
    use std::str::FromStr;

    assert_eq!(PixelationMode::from_str("quarter").unwrap(), PixelationMode::Quarter);
    assert_eq!(PixelationMode::from_str("half").unwrap(), PixelationMode::Half);
    assert_eq!(PixelationMode::from_str("q").unwrap(), PixelationMode::Quarter);
    assert_eq!(PixelationMode::from_str("h").unwrap(), PixelationMode::Half);
    assert!(PixelationMode::from_str("invalid").is_err());
}

#[test]
fn test_render_sizing_default() {
    let sizing = RenderSizing::default();
    assert_eq!(sizing.width_cells, None);
    assert_eq!(sizing.height_cells, None);
    assert_eq!(sizing.fit_width, false);
    assert_eq!(sizing.upscale, false);
}

#[test]
fn test_render_sizing_fit_width() {
    let sizing = RenderSizing {
        width_cells: Some(100),
        height_cells: None,
        fit_width: true,
        upscale: false,
    };
    assert!(sizing.fit_width);
    assert_eq!(sizing.width_cells, Some(100));
}

#[test]
fn test_render_sizing_upscale() {
    let sizing = RenderSizing {
        width_cells: Some(200),
        height_cells: Some(100),
        fit_width: false,
        upscale: true,
    };
    assert!(sizing.upscale);
}

// Helper to create a small test image
fn create_small_image(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, color);
        }
    }
    img
}

#[test]
fn test_small_image_dimensions() {
    let img = create_small_image(10, 10, Rgba([255, 0, 0, 255]));
    assert_eq!(img.width(), 10);
    assert_eq!(img.height(), 10);
    assert_eq!(img.get_pixel(0, 0), &Rgba([255, 0, 0, 255]));
}

#[test]
fn test_pixelation_modes_are_different() {
    assert_ne!(PixelationMode::Half, PixelationMode::Quarter);
}

#[test]
fn test_rotation_and_pixelation_independent() {
    // These should be orthogonal concerns
    let rotation = RotationMode::Exif;
    let pixelation = PixelationMode::Quarter;

    // Just verify they can coexist without compilation errors
    assert_eq!(rotation, RotationMode::Exif);
    assert_eq!(pixelation, PixelationMode::Quarter);
}
