/// Direct image processing example
///
/// This example demonstrates how to use the image loading APIs
/// directly without the Renderer abstraction.
///
/// Usage: cargo run --example image_processing <image-path>

use std::path::Path;
use showme::image::load_image;
use showme::config::RotationMode;

fn main() -> showme::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <image-path>", args[0]);
        std::process::exit(1);
    }

    let image_path = Path::new(&args[1]);

    println!("Loading image: {}", image_path.display());
    println!();

    // Load the image with EXIF rotation and no cropping
    let sequence = load_image(
        image_path,
        RotationMode::Exif,
        false,  // auto_crop
        0       // crop_border
    )?;

    // Display information about the loaded image
    println!("Image Information:");
    println!("  Path: {}", sequence.path.display());
    println!("  Total frames: {}", sequence.frames.len());
    println!();

    // Display information about each frame
    for (i, frame) in sequence.frames.iter().enumerate() {
        let image = &frame.pixels;

        println!("Frame {}:", i);
        println!("  Dimensions: {}x{}", image.width(), image.height());
        println!("  Delay: {:?}", frame.delay);

        // Sample some pixels
        if image.width() > 0 && image.height() > 0 {
            let pixel = image.get_pixel(0, 0);
            println!("  Top-left pixel (RGBA): ({}, {}, {}, {})",
                pixel[0], pixel[1], pixel[2], pixel[3]);

            // Check if image has transparency
            let has_alpha = image.pixels()
                .any(|p| p[3] < 255);
            println!("  Has transparency: {}", has_alpha);

            // Calculate average brightness
            let total_brightness: u64 = image.pixels()
                .map(|p| (p[0] as u64 + p[1] as u64 + p[2] as u64) / 3)
                .sum();
            let avg_brightness = total_brightness / (image.width() as u64 * image.height() as u64);
            println!("  Average brightness: {}/255", avg_brightness);
        }

        println!();
    }

    // Example of processing with different settings
    println!("Testing different rotation modes:");

    let modes = [
        (RotationMode::Off, "Rotation off"),
        (RotationMode::Exif, "EXIF-based rotation"),
    ];

    for (mode, description) in &modes {
        match load_image(image_path, *mode, false, 0) {
            Ok(seq) => {
                if let Some(first) = seq.first_frame() {
                    println!("  {}: {}x{}",
                        description,
                        first.pixels.width(),
                        first.pixels.height()
                    );
                }
            }
            Err(e) => {
                println!("  {}: Error - {}", description, e);
            }
        }
    }

    println!();
    println!("Testing auto-crop:");

    let cropped = load_image(
        image_path,
        RotationMode::Exif,
        true,   // auto_crop enabled
        10      // remove 10px border first
    )?;

    let original_frame = sequence.first_frame()
        .ok_or_else(|| showme::RimgError::other("No frames in original image"))?;
    let cropped_frame = cropped.first_frame()
        .ok_or_else(|| showme::RimgError::other("No frames in cropped image"))?;

    println!("  Original: {}x{}",
        original_frame.pixels.width(),
        original_frame.pixels.height()
    );
    println!("  After crop: {}x{}",
        cropped_frame.pixels.width(),
        cropped_frame.pixels.height()
    );

    let pixels_removed = (original_frame.pixels.width() * original_frame.pixels.height())
        .saturating_sub(cropped_frame.pixels.width() * cropped_frame.pixels.height());
    println!("  Pixels removed: {}", pixels_removed);

    Ok(())
}
