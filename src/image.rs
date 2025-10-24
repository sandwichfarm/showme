use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, ImageFormat, ImageReader, RgbaImage};

use crate::autocrop;
use crate::config::RotationMode;
use crate::error::{Result, RimgError};

#[cfg(feature = "video")]
use crate::video::{VideoLoader, load_video};

#[derive(Debug, Clone)]
pub struct Frame {
    pub pixels: RgbaImage,
    pub delay: Duration,
}

impl Frame {
    pub fn single(pixels: RgbaImage) -> Self {
        Self {
            pixels,
            delay: Duration::ZERO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageSequence {
    pub path: PathBuf,
    pub frames: Vec<Frame>,
}

impl ImageSequence {
    pub fn first_frame(&self) -> Option<&Frame> {
        self.frames.first()
    }
}

pub fn load_image(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
) -> Result<ImageSequence> {
    // Handle stdin as special case
    if path.to_str() == Some("-") {
        return load_from_stdin(rotation_mode, auto_crop, crop_border);
    }

    #[cfg(feature = "video")]
    {
        if VideoLoader::is_video_candidate(path) {
            if let Some(sequence) = load_video(path)? {
                return Ok(sequence);
            }
        }
    }

    let reader = ImageReader::open(path).map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: image::ImageError::IoError(err),
    })?;

    let reader = reader
        .with_guessed_format()
        .map_err(|err| RimgError::ImageOpen {
            path: path.to_path_buf(),
            source: image::ImageError::IoError(err),
        })?;

    // Check for QOI format first (not supported by image crate's auto-detection)
    #[cfg(feature = "qoi")]
    {
        if path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) == Some("qoi".to_string()) {
            return load_qoi(path, rotation_mode, auto_crop, crop_border);
        }
    }

    // Check for PDF format
    #[cfg(feature = "pdf")]
    {
        if path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase()) == Some("pdf".to_string()) {
            return load_pdf(path, rotation_mode, auto_crop, crop_border);
        }
    }

    // Check for SVG format
    #[cfg(feature = "svg")]
    {
        let ext = path.extension().and_then(|s| s.to_str()).map(|s| s.to_lowercase());
        if ext == Some("svg".to_string()) || ext == Some("svgz".to_string()) {
            return load_svg(path, rotation_mode, auto_crop, crop_border);
        }
    }

    match reader.format() {
        Some(ImageFormat::Gif) => load_gif(path, rotation_mode, auto_crop, crop_border),
        Some(_) | None => load_static_image(reader.decode(), path, rotation_mode, auto_crop, crop_border),
    }
}

fn load_from_stdin(rotation_mode: RotationMode, auto_crop: bool, crop_border: u32) -> Result<ImageSequence> {
    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|err| RimgError::other(format!("failed to read from stdin: {}", err)))?;

    let cursor = Cursor::new(buffer);
    let reader = ImageReader::new(cursor)
        .with_guessed_format()
        .map_err(|err| RimgError::other(format!("failed to detect image format from stdin: {}", err)))?;

    let path = PathBuf::from("<stdin>");
    match reader.format() {
        Some(ImageFormat::Gif) => {
            // For GIF from stdin, need to recreate cursor
            let data = reader.into_inner().into_inner();
            let cursor = Cursor::new(data);
            let decoder = GifDecoder::new(BufReader::new(cursor))
                .map_err(|err| RimgError::ImageOpen {
                    path: path.clone(),
                    source: err,
                })?;

            let frames = decoder
                .into_frames()
                .collect_frames()
                .map_err(|err| RimgError::FrameDecode {
                    path: path.clone(),
                    source: err,
                })?;

            let frames: Vec<Frame> = frames
                .into_iter()
                .map(|frame| {
                    let delay = frame.delay();
                    let (numer, denom) = delay.numer_denom_ms();
                    let millis = if denom == 0 {
                        0
                    } else {
                        (1000f32 * (numer as f32) / (denom as f32)).round() as u64
                    };
                    Frame {
                        pixels: frame.into_buffer(),
                        delay: Duration::from_millis(millis),
                    }
                })
                .collect();

            Ok(ImageSequence { path, frames })
        }
        Some(_) | None => load_static_image(reader.decode(), &path, rotation_mode, auto_crop, crop_border),
    }
}

fn load_static_image(
    result: image::ImageResult<DynamicImage>,
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
) -> Result<ImageSequence> {
    let mut image = result.map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: err,
    })?;

    // Apply EXIF orientation if enabled
    if rotation_mode == RotationMode::Exif {
        if let Some(orientation) = read_exif_orientation(path) {
            image = apply_orientation(image, orientation);
        }
    }

    // Apply cropping: first fixed border, then auto-crop
    if crop_border > 0 {
        image = autocrop::crop_border(image, crop_border);
    }
    if auto_crop {
        image = autocrop::auto_crop(image);
    }

    let rgba = image.to_rgba8();
    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames: vec![Frame::single(rgba)],
    })
}

fn load_gif(path: &Path, _rotation_mode: RotationMode, _auto_crop: bool, _crop_border: u32) -> Result<ImageSequence> {
    // Note: EXIF rotation is not applied to GIF animations as they typically don't have EXIF data
    // Note: Auto-crop is not applied to animations to maintain frame consistency
    let file = File::open(path)?;
    let decoder = GifDecoder::new(BufReader::new(file)).map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: err,
    })?;

    let frames = decoder
        .into_frames()
        .collect_frames()
        .map_err(|err| RimgError::FrameDecode {
            path: path.to_path_buf(),
            source: err,
        })?;

    let frames: Vec<Frame> = frames
        .into_iter()
        .map(|frame| {
            let delay = frame.delay();
            let (numer, denom) = delay.numer_denom_ms();
            let millis = if denom == 0 {
                0
            } else {
                (1000f32 * (numer as f32) / (denom as f32)).round() as u64
            };
            Frame {
                pixels: frame.into_buffer(),
                delay: Duration::from_millis(millis),
            }
        })
        .collect();

    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames,
    })
}

#[cfg(feature = "qoi")]
fn load_qoi(path: &Path, rotation_mode: RotationMode, auto_crop: bool, crop_border: u32) -> Result<ImageSequence> {
    // Read entire file into memory (QOI decoder needs bytes)
    let data = std::fs::read(path).map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: image::ImageError::IoError(err),
    })?;

    let (header, data) = qoi::decode_to_vec(&data).map_err(|err| {
        RimgError::other(format!("QOI decode error for '{}': {}", path.display(), err))
    })?;

    // Convert QOI data (RGB or RGBA) to RgbaImage
    let rgba_data = if header.channels == qoi::Channels::Rgba {
        data
    } else {
        // Convert RGB to RGBA
        data.chunks(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect()
    };

    let rgba_image = RgbaImage::from_raw(header.width, header.height, rgba_data)
        .ok_or_else(|| RimgError::other("QOI: failed to create image from decoded data"))?;

    let mut image = DynamicImage::ImageRgba8(rgba_image);

    // Apply rotation if needed (QOI doesn't have EXIF, but we check anyway)
    if rotation_mode == RotationMode::Exif {
        if let Some(orientation) = read_exif_orientation(path) {
            image = apply_orientation(image, orientation);
        }
    }

    // Apply cropping: first fixed border, then auto-crop
    if crop_border > 0 {
        image = autocrop::crop_border(image, crop_border);
    }
    if auto_crop {
        image = autocrop::auto_crop(image);
    }

    let pixels = image.to_rgba8();
    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames: vec![Frame::single(pixels)],
    })
}

/// Read EXIF orientation tag from an image file
fn read_exif_orientation(path: &Path) -> Option<u32> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let exif_reader = exif::Reader::new();
    let exif = exif_reader.read_from_container(&mut reader).ok()?;

    let orientation_field = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)?;

    match orientation_field.value {
        exif::Value::Short(ref vec) if !vec.is_empty() => Some(vec[0] as u32),
        _ => None,
    }
}

/// Apply EXIF orientation transformation to an image
fn apply_orientation(image: DynamicImage, orientation: u32) -> DynamicImage {
    use image::imageops::{flip_horizontal, flip_vertical, rotate180, rotate270, rotate90};

    match orientation {
        1 => image, // Normal
        2 => DynamicImage::ImageRgba8(flip_horizontal(&image.to_rgba8())),
        3 => DynamicImage::ImageRgba8(rotate180(&image.to_rgba8())),
        4 => DynamicImage::ImageRgba8(flip_vertical(&image.to_rgba8())),
        5 => {
            // Rotate 90 CW and flip horizontally
            let rotated = rotate90(&image.to_rgba8());
            DynamicImage::ImageRgba8(flip_horizontal(&rotated))
        }
        6 => DynamicImage::ImageRgba8(rotate90(&image.to_rgba8())),
        7 => {
            // Rotate 270 CW and flip horizontally
            let rotated = rotate270(&image.to_rgba8());
            DynamicImage::ImageRgba8(flip_horizontal(&rotated))
        }
        8 => DynamicImage::ImageRgba8(rotate270(&image.to_rgba8())),
        _ => image, // Unknown orientation, return as-is
    }
}

#[cfg(feature = "pdf")]
fn load_pdf(path: &Path, rotation_mode: RotationMode, auto_crop: bool, crop_border: u32) -> Result<ImageSequence> {
    use pdfium_render::prelude::*;

    // Initialize Pdfium library
    let pdfium = Pdfium::new(
        Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|err| RimgError::other(format!("Failed to initialize PDF library: {}", err)))?
    );

    // Load the PDF document
    let document = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|err| RimgError::other(format!("Failed to load PDF '{}': {}", path.display(), err)))?;

    let page_count = document.pages().len();
    if page_count == 0 {
        return Err(RimgError::other(format!("PDF '{}' has no pages", path.display())));
    }

    let mut frames = Vec::with_capacity(page_count as usize);

    // Render each page
    for page_index in 0..page_count {
        let page = document
            .pages()
            .get(page_index)
            .map_err(|err| RimgError::other(format!("Failed to get page {} from PDF: {}", page_index, err)))?;

        // Render at 150 DPI for good quality
        let render_config = PdfRenderConfig::new()
            .set_target_width(2000)
            .set_maximum_width(4000);

        let bitmap = page
            .render_with_config(&render_config)
            .map_err(|err| RimgError::other(format!("Failed to render PDF page {}: {}", page_index, err)))?;

        // Convert bitmap to RgbaImage
        let width = bitmap.width() as u32;
        let height = bitmap.height() as u32;

        let rgba_data: Vec<u8> = bitmap
            .as_raw_bytes()
            .chunks(4)
            .flat_map(|pixel| {
                // Pdfium returns BGRA, convert to RGBA
                [pixel[2], pixel[1], pixel[0], pixel[3]]
            })
            .collect();

        let rgba_image = RgbaImage::from_raw(width, height, rgba_data)
            .ok_or_else(|| RimgError::other("PDF: failed to create image from rendered data"))?;

        let mut image = DynamicImage::ImageRgba8(rgba_image);

        // Apply rotation if needed (PDFs don't typically have EXIF, but check anyway)
        if rotation_mode == RotationMode::Exif {
            if let Some(orientation) = read_exif_orientation(path) {
                image = apply_orientation(image, orientation);
            }
        }

        // Apply cropping: first fixed border, then auto-crop
        if crop_border > 0 {
            image = autocrop::crop_border(image, crop_border);
        }
        if auto_crop {
            image = autocrop::auto_crop(image);
        }

        frames.push(Frame::single(image.to_rgba8()));
    }

    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames,
    })
}

#[cfg(feature = "svg")]
fn load_svg(path: &Path, rotation_mode: RotationMode, auto_crop: bool, crop_border: u32) -> Result<ImageSequence> {
    use resvg::usvg;

    // Read SVG file
    let svg_data = std::fs::read(path).map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: image::ImageError::IoError(err),
    })?;

    // Parse SVG
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options)
        .map_err(|err| RimgError::other(format!("Failed to parse SVG '{}': {}", path.display(), err)))?;

    // Get SVG size
    let size = tree.size();

    // Render at a reasonable resolution (scale up small SVGs)
    let scale = if size.width() < 800.0 {
        800.0 / size.width()
    } else {
        1.0
    };

    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    // Create pixmap for rendering
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| RimgError::other("SVG: failed to create pixmap for rendering"))?;

    // Render SVG to pixmap
    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Convert RGBA pixmap data to RgbaImage
    let rgba_data = pixmap.data().to_vec();

    let rgba_image = RgbaImage::from_raw(width, height, rgba_data)
        .ok_or_else(|| RimgError::other("SVG: failed to create image from rendered data"))?;

    let mut image = DynamicImage::ImageRgba8(rgba_image);

    // Apply rotation if needed (SVGs don't have EXIF, but check anyway)
    if rotation_mode == RotationMode::Exif {
        if let Some(orientation) = read_exif_orientation(path) {
            image = apply_orientation(image, orientation);
        }
    }

    // Apply cropping: first fixed border, then auto-crop
    if crop_border > 0 {
        image = autocrop::crop_border(image, crop_border);
    }
    if auto_crop {
        image = autocrop::auto_crop(image);
    }

    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames: vec![Frame::single(image.to_rgba8())],
    })
}
