/// Custom backend selection example
///
/// This example demonstrates how to force a specific rendering backend
/// instead of using automatic detection.
///
/// Usage: cargo run --example custom_backend <backend> <image-path>
///        Backends: auto, unicode, kitty, iterm2

use std::path::PathBuf;
use terminal_media::{Config, Renderer, BackendKind, RenderSizing};
use terminal_media::config::{PixelationMode, RotationMode, BackgroundColor};

fn main() -> terminal_media::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <backend> <image-path>", args[0]);
        eprintln!("\nAvailable backends:");
        eprintln!("  auto     - Automatically detect best backend");
        eprintln!("  unicode  - Unicode block renderer (works everywhere)");
        eprintln!("  kitty    - Kitty graphics protocol");
        eprintln!("  iterm2   - iTerm2 inline images");
        eprintln!("\nExample:");
        eprintln!("  cargo run --example custom_backend kitty photo.jpg");
        std::process::exit(1);
    }

    let backend = match args[1].as_str() {
        "auto" => BackendKind::Auto,
        "unicode" => BackendKind::Unicode,
        "kitty" => BackendKind::Kitty,
        "iterm2" => BackendKind::Iterm2,
        other => {
            eprintln!("Unknown backend: {}", other);
            eprintln!("Valid options: auto, unicode, kitty, iterm2");
            std::process::exit(1);
        }
    };

    let image_path = PathBuf::from(&args[2]);

    println!("Using backend: {:?}", backend);

    let config = Config {
        inputs: vec![image_path],
        backend,
        sizing: RenderSizing::unconstrained(),
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Exif,
        background: BackgroundColor::Auto,
        quiet: false,
        verbose: true,  // Enable verbose mode to see backend info
        clear_once: false,
        clear_between: false,
        wait_between_images: None,
        wait_between_rows: None,
        title_format: None,
        center: true,
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

    let renderer = Renderer::build(config)?;
    renderer.run()
}
