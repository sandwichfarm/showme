use std::str::FromStr;
use timg_rust::{BackendKind, PixelationMode, RotationMode};

#[test]
fn test_backend_kind_from_str() {
    assert_eq!(BackendKind::from_str("auto").unwrap(), BackendKind::Auto);
    assert_eq!(BackendKind::from_str("unicode").unwrap(), BackendKind::Unicode);
    assert_eq!(BackendKind::from_str("kitty").unwrap(), BackendKind::Kitty);
    assert_eq!(BackendKind::from_str("iterm2").unwrap(), BackendKind::Iterm2);

    // Case insensitive
    assert_eq!(BackendKind::from_str("UNICODE").unwrap(), BackendKind::Unicode);
    assert_eq!(BackendKind::from_str("Auto").unwrap(), BackendKind::Auto);

    // Invalid
    assert!(BackendKind::from_str("invalid").is_err());
}

#[test]
fn test_pixelation_mode_shortcuts() {
    assert_eq!(PixelationMode::from_str("q").unwrap(), PixelationMode::Quarter);
    assert_eq!(PixelationMode::from_str("h").unwrap(), PixelationMode::Half);
    assert_eq!(PixelationMode::from_str("Q").unwrap(), PixelationMode::Quarter);
    assert_eq!(PixelationMode::from_str("H").unwrap(), PixelationMode::Half);
}

#[test]
fn test_rotation_mode_case_insensitive() {
    assert_eq!(RotationMode::from_str("exif").unwrap(), RotationMode::Exif);
    assert_eq!(RotationMode::from_str("EXIF").unwrap(), RotationMode::Exif);
    assert_eq!(RotationMode::from_str("off").unwrap(), RotationMode::Off);
    assert_eq!(RotationMode::from_str("OFF").unwrap(), RotationMode::Off);
}

#[test]
fn test_backend_variants() {
    let variants = BackendKind::variants();
    assert!(variants.contains(&"auto"));
    assert!(variants.contains(&"unicode"));
    assert!(variants.contains(&"kitty"));
    assert!(variants.contains(&"iterm2"));
    assert!(variants.contains(&"sixel"));
}

#[test]
fn test_defaults() {
    assert_eq!(PixelationMode::default(), PixelationMode::Quarter);
    assert_eq!(RotationMode::default(), RotationMode::Exif);
}

#[test]
fn test_enum_equality() {
    assert_eq!(BackendKind::Auto, BackendKind::Auto);
    assert_ne!(BackendKind::Auto, BackendKind::Unicode);

    assert_eq!(PixelationMode::Quarter, PixelationMode::Quarter);
    assert_ne!(PixelationMode::Quarter, PixelationMode::Half);

    assert_eq!(RotationMode::Exif, RotationMode::Exif);
    assert_ne!(RotationMode::Exif, RotationMode::Off);
}
