/// Basic image viewer example
///
/// This example shows how to use showme to display a single image
/// using the default configuration.
///
/// Usage: cargo run --example basic_viewer path/to/image.jpg

use std::path::PathBuf;
use showme::{Config, Renderer, BackendKind, RenderSizing};
use showme::config::{PixelationMode, RotationMode, BackgroundColor};

fn main() -> showme::Result<()> {
    // Get image path from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <image-path>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  cargo run --example basic_viewer photo.jpg");
        std::process::exit(1);
    }

    let image_path = PathBuf::from(&args[1]);

    // Create a basic configuration
    let config = Config {
        inputs: vec![image_path],
        backend: BackendKind::Auto,  // Automatically detect best backend
        sizing: RenderSizing::unconstrained(),  // Use terminal size
        pixelation: PixelationMode::Quarter,  // Quarter-block for best detail
        rotation: RotationMode::Exif,  // Respect EXIF orientation
        background: BackgroundColor::Auto,
        quiet: false,
        verbose: false,
        clear_once: false,
        clear_between: false,
        wait_between_images: None,
        wait_between_rows: None,
        title_format: None,
        center: false,
        alternate_screen: false,
        hide_cursor: true,
        pattern_color: None,
        pattern_size: 1,
        grid: None,
        loop_forever: false,
        loops: None,
        max_frames: None,
        frame_offset: 0,
        max_duration: None,
        auto_crop: false,
        crop_border: 0,
        use_8bit_color: false,
        output_file: None,
        threads: None,
        compress_level: 1,
        force_video: false,
        force_image: false,
        scroll_animation: false,
        scroll_delay: std::time::Duration::from_millis(50),
        scroll_dx: 1,
        scroll_dy: 1,
    };

    // Build and run the renderer
    let renderer = Renderer::build(config)?;
    renderer.run()
}
