use std::path::PathBuf;
use showme::{BackendKind, Config, PixelationMode, RenderSizing, RotationMode};

#[test]
fn test_config_with_all_options() {
    let config = Config {
        inputs: vec![PathBuf::from("test.jpg")],
        backend: BackendKind::Unicode,
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Exif,
        sizing: RenderSizing {
            width_cells: Some(100),
            height_cells: Some(50),
            fit_width: false,
            upscale: true,
        },
        grid: None,
        loop_forever: false,
        loops: Some(3),
        max_frames: Some(100),
        frame_offset: 10,
        max_duration: Some(std::time::Duration::from_secs(5)),
        quiet: false,
        clear_between: false,
        clear_once: true,
        wait_between_images: Some(std::time::Duration::from_millis(500)),
        wait_between_rows: None,
        title_format: Some(String::from("%f")),
        center: true,
        alternate_screen: true,
        background: showme::config::BackgroundColor::Auto,
        pattern_color: None,
        pattern_size: 1,
        auto_crop: false,
        crop_border: 0,
    };

    assert!(config.validate());
    assert_eq!(config.inputs.len(), 1);
    assert_eq!(config.pixelation, PixelationMode::Quarter);
    assert_eq!(config.rotation, RotationMode::Exif);
    assert_eq!(config.loops, Some(3));
    assert_eq!(config.max_frames, Some(100));
    assert_eq!(config.frame_offset, 10);
}

#[test]
fn test_config_validation_empty_inputs() {
    let config = Config {
        inputs: vec![],
        backend: BackendKind::Auto,
        pixelation: PixelationMode::default(),
        rotation: RotationMode::default(),
        sizing: RenderSizing::default(),
        grid: None,
        loop_forever: false,
        loops: None,
        max_frames: None,
        frame_offset: 0,
        max_duration: None,
        quiet: false,
        clear_between: false,
        clear_once: false,
        wait_between_images: None,
        wait_between_rows: None,
        title_format: None,
        center: false,
        alternate_screen: false,
        background: showme::config::BackgroundColor::Auto,
        pattern_color: None,
        pattern_size: 1,
        auto_crop: false,
        crop_border: 0,
    };

    assert!(!config.validate());
}

#[test]
fn test_animation_control_combinations() {
    // Test that loops and frame controls can coexist
    let config = Config {
        inputs: vec![PathBuf::from("animation.gif")],
        backend: BackendKind::Auto,
        pixelation: PixelationMode::Quarter,
        rotation: RotationMode::Off,
        sizing: RenderSizing::default(),
        grid: None,
        loop_forever: false,
        loops: Some(2),
        max_frames: Some(50),
        frame_offset: 10,
        max_duration: Some(std::time::Duration::from_secs(10)),
        quiet: false,
        clear_between: false,
        clear_once: false,
        wait_between_images: None,
        wait_between_rows: None,
        title_format: None,
        center: false,
        alternate_screen: false,
        background: showme::config::BackgroundColor::Auto,
        pattern_color: None,
        pattern_size: 1,
        auto_crop: false,
        crop_border: 0,
    };

    assert!(config.validate());
    assert_eq!(config.loops, Some(2));
    assert_eq!(config.max_frames, Some(50));
    assert_eq!(config.frame_offset, 10);
    assert!(config.max_duration.is_some());
}

#[test]
fn test_infinite_loops() {
    let config = Config {
        inputs: vec![PathBuf::from("video.mp4")],
        backend: BackendKind::Auto,
        pixelation: PixelationMode::Half,
        rotation: RotationMode::Exif,
        sizing: RenderSizing::default(),
        grid: None,
        loop_forever: false,
        loops: Some(-1), // Infinite
        max_frames: None,
        frame_offset: 0,
        max_duration: None,
        quiet: false,
        clear_between: false,
        clear_once: false,
        wait_between_images: None,
        wait_between_rows: None,
        title_format: None,
        center: false,
        alternate_screen: false,
        background: showme::config::BackgroundColor::Auto,
        pattern_color: None,
        pattern_size: 1,
        auto_crop: false,
        crop_border: 0,
    };

    assert_eq!(config.loops, Some(-1));
}

#[test]
fn test_sizing_combinations() {
    // Test upscale with dimensions
    let sizing = RenderSizing {
        width_cells: Some(200),
        height_cells: Some(100),
        fit_width: false,
        upscale: true,
    };

    assert!(sizing.upscale);
    assert!(!sizing.fit_width);
    assert_eq!(sizing.width_cells, Some(200));

    // Test fit_width alone
    let sizing2 = RenderSizing {
        width_cells: None,
        height_cells: None,
        fit_width: true,
        upscale: false,
    };

    assert!(sizing2.fit_width);
    assert!(!sizing2.upscale);
}
