/// Programmatic configuration example
///
/// This example demonstrates how to build complex configurations
/// programmatically without using CLI parsing.
///
/// Usage: cargo run --example programmatic_config <image-path>

use std::path::PathBuf;
use std::time::Duration;
use terminal_media::{Config, Renderer, BackendKind, RenderSizing};
use terminal_media::config::{PixelationMode, RotationMode, BackgroundColor};

fn main() -> terminal_media::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <image-path>", args[0]);
        std::process::exit(1);
    }

    let image_path = PathBuf::from(&args[1]);

    // Build a sophisticated configuration programmatically
    let config = Config {
        inputs: vec![image_path],

        // Rendering settings
        backend: BackendKind::Auto,
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Exif,
        background: BackgroundColor::Auto,

        // Sizing configuration
        sizing: RenderSizing {
            width_cells: Some(100),
            height_cells: Some(40),
            fit_width: false,
            fit_height: false,
            upscale: false,
            upscale_integer: false,
            antialias: true,
            width_stretch: 2.0, // Typical terminal chars are ~2x taller than wide, stretch by 2x
        },

        // Display options
        center: true,
        alternate_screen: true,
        hide_cursor: true,
        clear_once: false,
        clear_between: false,

        // Animation control
        loop_forever: false,
        loops: Some(3),
        max_frames: None,
        frame_offset: 0,
        max_duration: Some(Duration::from_secs(10)),
        wait_between_images: Some(Duration::from_secs(2)),
        wait_between_rows: None,

        // Image processing
        auto_crop: false,
        crop_border: 0,

        // Advanced options
        use_8bit_color: false,
        compress_level: 1,
        threads: Some(4),

        // Output and verbosity
        quiet: false,
        verbose: true,
        output_file: None,

        // Title and pattern
        title_format: Some("%f - %wx%h".to_string()),
        pattern_color: None,
        pattern_size: 1,

        // Grid and scrolling
        grid: None,
        scroll_animation: false,
        scroll_delay: Duration::from_millis(50),
        scroll_dx: 1,
        scroll_dy: 1,

        // Force type interpretation
        force_video: false,
        force_image: false,
    };

    println!("Configuration:");
    println!("  Backend: {:?}", config.backend);
    println!("  Size: {}x{}",
        config.sizing.width_cells.unwrap_or(0),
        config.sizing.height_cells.unwrap_or(0)
    );
    println!("  Centered: {}", config.center);
    println!("  Alternate screen: {}", config.alternate_screen);
    println!("  Max duration: {:?}", config.max_duration);
    println!();

    let renderer = Renderer::build(config)?;
    renderer.run()
}
