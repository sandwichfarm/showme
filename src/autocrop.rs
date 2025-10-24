/// Auto-crop functionality for removing uniform borders from images
///
/// This module implements automatic border detection and cropping,
/// useful for screenshots, scanned documents, and images with padding.

use image::{DynamicImage, GenericImageView, Rgba};

/// Threshold for color similarity (0-255 per channel)
/// Pixels within this threshold of the border color are considered background
const COLOR_THRESHOLD: u8 = 10;

/// Determine if two colors are similar within threshold
fn colors_similar(a: Rgba<u8>, b: Rgba<u8>, threshold: u8) -> bool {
    let threshold = threshold as i32;
    (a[0] as i32 - b[0] as i32).abs() <= threshold
        && (a[1] as i32 - b[1] as i32).abs() <= threshold
        && (a[2] as i32 - b[2] as i32).abs() <= threshold
        && (a[3] as i32 - b[3] as i32).abs() <= threshold
}

/// Detect the background color by sampling edge pixels
///
/// Samples the four corners and edges to determine the most common
/// edge color, which is assumed to be the background.
fn detect_background_color(img: &DynamicImage) -> Rgba<u8> {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return Rgba([255, 255, 255, 255]);
    }

    // Sample corner pixel (most likely to be background)
    // Use top-left corner as the background color
    // (most common convention for padding)
    img.get_pixel(0, 0)
}

/// Find the crop boundaries by scanning inward from edges
///
/// Returns (left, top, right, bottom) boundaries
fn find_crop_bounds(img: &DynamicImage, bg_color: Rgba<u8>) -> Option<(u32, u32, u32, u32)> {
    let (width, height) = img.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    // Scan from left
    let mut left = 0;
    'left_scan: for x in 0..width {
        for y in 0..height {
            if !colors_similar(img.get_pixel(x, y), bg_color, COLOR_THRESHOLD) {
                left = x;
                break 'left_scan;
            }
        }
    }

    // Scan from right
    let mut right = width - 1;
    'right_scan: for x in (0..width).rev() {
        for y in 0..height {
            if !colors_similar(img.get_pixel(x, y), bg_color, COLOR_THRESHOLD) {
                right = x;
                break 'right_scan;
            }
        }
    }

    // Scan from top
    let mut top = 0;
    'top_scan: for y in 0..height {
        for x in 0..width {
            if !colors_similar(img.get_pixel(x, y), bg_color, COLOR_THRESHOLD) {
                top = y;
                break 'top_scan;
            }
        }
    }

    // Scan from bottom
    let mut bottom = height - 1;
    'bottom_scan: for y in (0..height).rev() {
        for x in 0..width {
            if !colors_similar(img.get_pixel(x, y), bg_color, COLOR_THRESHOLD) {
                bottom = y;
                break 'bottom_scan;
            }
        }
    }

    // If all pixels are background, return None (don't crop)
    if left >= right || top >= bottom {
        return None;
    }

    Some((left, top, right, bottom))
}

/// Auto-crop an image by removing uniform borders
///
/// Detects the background color from edge pixels and removes any
/// continuous border of that color.
///
/// Returns the cropped image, or the original if no cropping is needed.
pub fn auto_crop(img: DynamicImage) -> DynamicImage {
    let bg_color = detect_background_color(&img);

    if let Some((left, top, right, bottom)) = find_crop_bounds(&img, bg_color) {
        let crop_width = right - left + 1;
        let crop_height = bottom - top + 1;

        // Only crop if we're actually removing some border
        let (orig_width, orig_height) = img.dimensions();
        if crop_width < orig_width || crop_height < orig_height {
            return img.crop_imm(left, top, crop_width, crop_height);
        }
    }

    // No cropping needed
    img
}

/// Crop a fixed border of pixels from all sides
///
/// This is useful for removing known padding or borders before auto-crop.
pub fn crop_border(img: DynamicImage, border: u32) -> DynamicImage {
    let (width, height) = img.dimensions();

    if border == 0 || border * 2 >= width || border * 2 >= height {
        return img;
    }

    let new_width = width - 2 * border;
    let new_height = height - 2 * border;

    img.crop_imm(border, border, new_width, new_height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn create_test_image_with_border(
        content_width: u32,
        content_height: u32,
        border: u32,
    ) -> DynamicImage {
        let total_width = content_width + 2 * border;
        let total_height = content_height + 2 * border;
        let mut img = ImageBuffer::new(total_width, total_height);

        // Fill with white background
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }

        // Fill content area with red
        for y in border..(border + content_height) {
            for x in border..(border + content_width) {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }

        DynamicImage::ImageRgba8(img)
    }

    #[test]
    fn test_colors_similar() {
        let c1 = Rgba([100, 100, 100, 255]);
        let c2 = Rgba([105, 95, 102, 255]);
        let c3 = Rgba([150, 150, 150, 255]);

        assert!(colors_similar(c1, c2, 10));
        assert!(!colors_similar(c1, c3, 10));
    }

    #[test]
    fn test_detect_background_color() {
        let img = create_test_image_with_border(10, 10, 5);
        let bg = detect_background_color(&img);
        assert_eq!(bg, Rgba([255, 255, 255, 255]));
    }

    #[test]
    fn test_auto_crop_with_border() {
        let img = create_test_image_with_border(10, 10, 5);
        let cropped = auto_crop(img);

        assert_eq!(cropped.dimensions(), (10, 10));
    }

    #[test]
    fn test_auto_crop_no_border() {
        let mut img = ImageBuffer::new(10, 10);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]);
        }
        let img = DynamicImage::ImageRgba8(img);
        let cropped = auto_crop(img.clone());

        assert_eq!(cropped.dimensions(), img.dimensions());
    }

    #[test]
    fn test_crop_border() {
        let img = create_test_image_with_border(20, 20, 10);
        let cropped = crop_border(img, 10);

        assert_eq!(cropped.dimensions(), (20, 20));
    }

    #[test]
    fn test_crop_border_too_large() {
        let img = create_test_image_with_border(10, 10, 5);
        let cropped = crop_border(img.clone(), 50);

        // Should not crop if border is too large
        assert_eq!(cropped.dimensions(), img.dimensions());
    }

    #[test]
    fn test_auto_crop_asymmetric_border() {
        // Create image with asymmetric border
        let mut img = ImageBuffer::new(30, 20);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 255, 255, 255]);
        }
        // Content: 10x10 starting at (10, 5)
        for y in 5..15 {
            for x in 10..20 {
                img.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            }
        }

        let img = DynamicImage::ImageRgba8(img);
        let cropped = auto_crop(img);

        assert_eq!(cropped.dimensions(), (10, 10));
    }
}
