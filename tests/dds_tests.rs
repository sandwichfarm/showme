use std::path::PathBuf;

use showme::RotationMode;
use showme::image::load_image;

fn write_temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let mut path = std::env::temp_dir();
    let uniq = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Keep the original extension at the end of the file name so format sniffing works.
    path.push(format!("showme-test-{uniq}-{name}"));
    std::fs::write(&path, bytes).expect("write temp file");
    path
}

fn make_uncompressed_argb8888_dds_2x2() -> Vec<u8> {
    const DDSD_PITCH: u32 = 0x0000_0008;
    const DDSD_PIXELFORMAT: u32 = 0x0000_1000;
    const DDSD_CAPS: u32 = 0x0000_0001;
    const DDSD_HEIGHT: u32 = 0x0000_0002;
    const DDSD_WIDTH: u32 = 0x0000_0004;

    const DDPF_ALPHAPIXELS: u32 = 0x0000_0001;
    const DDPF_RGB: u32 = 0x0000_0040;
    const DDSCAPS_TEXTURE: u32 = 0x0000_1000;

    let width = 2u32;
    let height = 2u32;
    let pitch = width * 4;

    let mut out = Vec::new();
    out.extend_from_slice(b"DDS ");

    // DDS_HEADER (124 bytes)
    out.extend_from_slice(&124u32.to_le_bytes()); // dwSize
    out.extend_from_slice(
        &(DDSD_CAPS | DDSD_HEIGHT | DDSD_WIDTH | DDSD_PITCH | DDSD_PIXELFORMAT).to_le_bytes(),
    ); // dwFlags
    out.extend_from_slice(&height.to_le_bytes()); // dwHeight
    out.extend_from_slice(&width.to_le_bytes()); // dwWidth
    out.extend_from_slice(&pitch.to_le_bytes()); // dwPitchOrLinearSize
    out.extend_from_slice(&0u32.to_le_bytes()); // dwDepth
    out.extend_from_slice(&0u32.to_le_bytes()); // dwMipMapCount
    out.extend_from_slice(&[0u8; 44]); // dwReserved1[11]

    // DDS_PIXELFORMAT (32 bytes)
    out.extend_from_slice(&32u32.to_le_bytes()); // dwSize
    out.extend_from_slice(&(DDPF_RGB | DDPF_ALPHAPIXELS).to_le_bytes()); // dwFlags
    out.extend_from_slice(&0u32.to_le_bytes()); // dwFourCC
    out.extend_from_slice(&32u32.to_le_bytes()); // dwRGBBitCount
    out.extend_from_slice(&0x00ff_0000u32.to_le_bytes()); // dwRBitMask
    out.extend_from_slice(&0x0000_ff00u32.to_le_bytes()); // dwGBitMask
    out.extend_from_slice(&0x0000_00ffu32.to_le_bytes()); // dwBBitMask
    out.extend_from_slice(&0xff00_0000u32.to_le_bytes()); // dwABitMask

    // caps
    out.extend_from_slice(&DDSCAPS_TEXTURE.to_le_bytes()); // dwCaps
    out.extend_from_slice(&0u32.to_le_bytes()); // dwCaps2
    out.extend_from_slice(&0u32.to_le_bytes()); // dwCaps3
    out.extend_from_slice(&0u32.to_le_bytes()); // dwCaps4
    out.extend_from_slice(&0u32.to_le_bytes()); // dwReserved2

    // Pixel data: 2x2 BGRA8888
    // Row 0: red, green; Row 1: blue, yellow.
    out.extend_from_slice(&[0, 0, 255, 255]); // red
    out.extend_from_slice(&[0, 255, 0, 255]); // green
    out.extend_from_slice(&[255, 0, 0, 255]); // blue
    out.extend_from_slice(&[0, 255, 255, 255]); // yellow

    out
}

#[test]
fn decodes_uncompressed_dds_argb8888() {
    let bytes = make_uncompressed_argb8888_dds_2x2();
    let path = write_temp_file("pattern.dds", &bytes);

    let seq = load_image(&path, RotationMode::Off, false, 0).expect("load dds");
    assert_eq!(seq.frames.len(), 1);

    let frame = seq.first_frame().expect("frame");
    assert_eq!(frame.pixels.width(), 2);
    assert_eq!(frame.pixels.height(), 2);

    let tl = frame.pixels.get_pixel(0, 0).0;
    let tr = frame.pixels.get_pixel(1, 0).0;
    let bl = frame.pixels.get_pixel(0, 1).0;
    let br = frame.pixels.get_pixel(1, 1).0;

    assert_eq!(tl, [255, 0, 0, 255]);
    assert_eq!(tr, [0, 255, 0, 255]);
    assert_eq!(bl, [0, 0, 255, 255]);
    assert_eq!(br, [255, 255, 0, 255]);

    let _ = std::fs::remove_file(&path);
}
