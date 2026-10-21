use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, DynamicImage, ImageDecoder, ImageFormat, ImageReader, RgbaImage};

use crate::autocrop;
use crate::config::RotationMode;
use crate::error::{Result, RimgError};

#[cfg(feature = "video")]
use crate::video::{VideoLoader, load_video_with_limits};

// Bounds apply to retained decoded RGBA data; native codec working buffers are separate.
const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DECODED_BYTES: u64 = 512 * 1024 * 1024;
const MAX_FRAMES: usize = 1024;

#[derive(Clone, Default)]
pub(crate) struct LoadLimits {
    end_frame: Option<usize>,
    offset: usize,
    duration: Option<Duration>,
    cancelled: Option<Arc<AtomicBool>>,
}

impl LoadLimits {
    pub(crate) fn check_cancelled(&self) -> Result<()> {
        if self
            .cancelled
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
        {
            Err(std::io::Error::from(std::io::ErrorKind::Interrupted).into())
        } else {
            Ok(())
        }
    }

    fn for_config(config: &crate::config::Config) -> Result<Self> {
        let max_frames = if config.scroll_animation || config.grid.is_some() {
            Some(1)
        } else {
            config.max_frames
        };
        let end_frame = max_frames
            .map(|count| {
                config
                    .frame_offset
                    .checked_add(count)
                    .ok_or_else(|| RimgError::other("frame offset plus frame count is too large"))
            })
            .transpose()?;
        Ok(Self {
            end_frame,
            offset: config.frame_offset,
            duration: config.max_duration,
            cancelled: None,
        })
    }
}

pub(crate) struct DecodeBudget {
    limits: LoadLimits,
    frames: usize,
    bytes: u64,
    elapsed: Duration,
}

impl DecodeBudget {
    pub(crate) fn new(limits: LoadLimits) -> Self {
        Self {
            limits,
            frames: 0,
            bytes: 0,
            elapsed: Duration::ZERO,
        }
    }

    pub(crate) fn full(&self) -> bool {
        self.limits.end_frame.is_some_and(|end| self.frames >= end)
            || (self.frames >= self.limits.offset
                && self
                    .limits
                    .duration
                    .is_some_and(|duration| self.elapsed >= duration))
    }

    pub(crate) fn check_frame(&self, width: u32, height: u32) -> Result<()> {
        self.limits.check_cancelled()?;
        check_dimensions(width, height)?;
        if self.frames >= MAX_FRAMES {
            return Err(RimgError::other(
                "media exceeds the 1024-frame decode limit; select a shorter preview with --frames or --duration",
            ));
        }
        if self.bytes + u64::from(width) * u64::from(height) * 4 > MAX_DECODED_BYTES {
            return Err(RimgError::other(
                "media exceeds the 512 MiB decoded-data limit; select a shorter preview",
            ));
        }
        Ok(())
    }

    pub(crate) fn add(&mut self, frame: &Frame) {
        self.bytes += u64::from(frame.pixels.width()) * u64::from(frame.pixels.height()) * 4;
        self.frames += 1;
        if self.frames > self.limits.offset {
            self.elapsed = self.elapsed.saturating_add(frame.delay);
        }
    }
}

pub(crate) fn check_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0
        || height == 0
        || width > 16384
        || height > 16384
        || u64::from(width) * u64::from(height) > 64 * 1024 * 1024
    {
        return Err(RimgError::other(
            "image dimensions exceed the limit (16384 per side, 64 megapixels)",
        ));
    }
    Ok(())
}

fn image_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits
}

fn read_bounded(reader: impl Read) -> Result<Vec<u8>> {
    let mut data = Vec::new();
    reader.take(MAX_INPUT_BYTES + 1).read_to_end(&mut data)?;
    if data.len() as u64 > MAX_INPUT_BYTES {
        return Err(RimgError::other("input exceeds the 256 MiB byte limit"));
    }
    Ok(data)
}

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
    load_image_inner(
        path,
        rotation_mode,
        auto_crop,
        crop_border,
        true,
        LoadLimits::default(),
    )
}

pub(crate) fn load_configured_image(
    path: &Path,
    config: &crate::config::Config,
    cancelled: Option<Arc<AtomicBool>>,
) -> Result<ImageSequence> {
    let mut limits = LoadLimits::for_config(config)?;
    limits.cancelled = cancelled;
    limits.check_cancelled()?;
    if config.force_video && config.force_image {
        return Err(RimgError::other(
            "force-video and force-image are mutually exclusive",
        ));
    }
    if config.force_video {
        #[cfg(feature = "video")]
        {
            if path.to_str() == Some("-") {
                return Err(RimgError::other(
                    "force-video requires a file path; stdin supports images only",
                ));
            }
            return load_video_with_limits(path, limits)?.ok_or_else(|| {
                RimgError::other(format!("failed to decode video '{}'", path.display()))
            });
        }
        #[cfg(not(feature = "video"))]
        return Err(RimgError::other(
            "video decoding requested but the video feature is disabled",
        ));
    }
    load_image_inner(
        path,
        config.rotation,
        config.auto_crop,
        config.crop_border,
        !config.force_image,
        limits,
    )
}

fn load_image_inner(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
    probe_video: bool,
    limits: LoadLimits,
) -> Result<ImageSequence> {
    #[cfg(not(feature = "video"))]
    let _ = probe_video;
    // Handle stdin as special case
    if path.to_str() == Some("-") {
        return load_from_stdin(rotation_mode, auto_crop, crop_border, limits);
    }

    #[cfg(feature = "video")]
    {
        if probe_video
            && VideoLoader::is_video_candidate(path)
            && let Some(sequence) = load_video_with_limits(path, limits.clone())?
        {
            return Ok(sequence);
        }
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let disabled_feature = match extension.as_str() {
        "qoi" if !cfg!(feature = "qoi") => Some("qoi"),
        "pdf" if !cfg!(feature = "pdf") => Some("pdf"),
        "svg" | "svgz" if !cfg!(feature = "svg") => Some("svg"),
        "mp4" | "mkv" | "mov" | "avi" | "webm" if probe_video && !cfg!(feature = "video") => {
            Some("video")
        }
        _ => None,
    };
    if let Some(feature) = disabled_feature {
        return Err(RimgError::other(format!(
            "{feature} decoding requested but the {feature} feature is disabled"
        )));
    }

    if std::fs::metadata(path)?.len() > MAX_INPUT_BYTES {
        return Err(RimgError::other("input exceeds the 256 MiB byte limit"));
    }
    let reader = ImageReader::open(path).map_err(|err| RimgError::ImageOpen {
        path: path.to_path_buf(),
        source: image::ImageError::IoError(err),
    })?;

    let mut reader = reader
        .with_guessed_format()
        .map_err(|err| RimgError::ImageOpen {
            path: path.to_path_buf(),
            source: image::ImageError::IoError(err),
        })?;

    reader.limits(image_limits());

    // Check for QOI format first (not supported by image crate's auto-detection)
    #[cfg(feature = "qoi")]
    {
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            == Some("qoi".to_string())
        {
            return load_qoi(path, rotation_mode, auto_crop, crop_border);
        }
    }

    // Check for PDF format
    #[cfg(feature = "pdf")]
    {
        if path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            == Some("pdf".to_string())
        {
            return load_pdf(path, rotation_mode, auto_crop, crop_border, limits);
        }
    }

    // Check for SVG format
    #[cfg(feature = "svg")]
    {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());
        if ext == Some("svg".to_string()) || ext == Some("svgz".to_string()) {
            return load_svg(path, rotation_mode, auto_crop, crop_border);
        }
    }

    // Check for DDS format (uncompressed DDS variants are not handled by the `image` crate)
    if path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("dds"))
        .unwrap_or(false)
    {
        return load_dds(path, rotation_mode, auto_crop, crop_border);
    }

    match reader.format() {
        Some(ImageFormat::Gif) => load_gif(path, rotation_mode, auto_crop, crop_border, limits),
        Some(_) | None => {
            load_static_image(reader.decode(), path, rotation_mode, auto_crop, crop_border)
        }
    }
}

#[cfg(unix)]
fn read_stdin(limits: &LoadLimits) -> Result<Vec<u8>> {
    use std::os::fd::AsRawFd;
    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let mut data = Vec::new();
    loop {
        limits.check_cancelled()?;
        let mut descriptor = libc::pollfd {
            fd: stdin.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: the live descriptor is borrowed for this bounded poll call.
        let ready = unsafe { libc::poll(&mut descriptor, 1, 25) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if ready == 0 {
            continue;
        }
        let mut buffer = [0_u8; 8192];
        // Read the polled descriptor directly, avoiding StdinLock prefetching.
        // SAFETY: the borrowed descriptor and writable buffer remain live.
        let count =
            unsafe { libc::read(stdin.as_raw_fd(), buffer.as_mut_ptr().cast(), buffer.len()) };
        if count < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if count == 0 {
            break;
        }
        if data.len() as u64 + count as u64 > MAX_INPUT_BYTES {
            return Err(RimgError::other("input exceeds the 256 MiB byte limit"));
        }
        data.extend_from_slice(&buffer[..count as usize]);
    }
    Ok(data)
}

#[cfg(not(unix))]
fn read_stdin(limits: &LoadLimits) -> Result<Vec<u8>> {
    limits.check_cancelled()?;
    let data = read_bounded(std::io::stdin().lock())?;
    limits.check_cancelled()?;
    Ok(data)
}

fn load_from_stdin(
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
    limits: LoadLimits,
) -> Result<ImageSequence> {
    let buffer = read_stdin(&limits)?;
    let mut reader = ImageReader::new(Cursor::new(buffer))
        .with_guessed_format()
        .map_err(|err| {
            RimgError::other(format!("failed to detect image format from stdin: {err}"))
        })?;
    reader.limits(image_limits());
    let path = PathBuf::from("<stdin>");
    match reader.format() {
        Some(ImageFormat::Gif) => {
            let decoder =
                GifDecoder::new(reader.into_inner()).map_err(|source| RimgError::ImageOpen {
                    path: path.clone(),
                    source,
                })?;
            decode_gif(decoder, &path, auto_crop, crop_border, limits)
        }
        _ => load_static_image(
            reader.decode(),
            &path,
            rotation_mode,
            auto_crop,
            crop_border,
        ),
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

    check_dimensions(image.width(), image.height())?;

    // Apply EXIF orientation if enabled
    if rotation_mode == RotationMode::Exif
        && let Some(orientation) = read_exif_orientation(path)
    {
        image = apply_orientation(image, orientation);
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

fn load_gif(
    path: &Path,
    _rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
    limits: LoadLimits,
) -> Result<ImageSequence> {
    let decoder = GifDecoder::new(BufReader::new(File::open(path)?)).map_err(|source| {
        RimgError::ImageOpen {
            path: path.to_path_buf(),
            source,
        }
    })?;
    decode_gif(decoder, path, auto_crop, crop_border, limits)
}

fn decode_gif<R: std::io::BufRead + std::io::Seek>(
    mut decoder: GifDecoder<R>,
    path: &Path,
    auto_crop: bool,
    crop_border: u32,
    limits: LoadLimits,
) -> Result<ImageSequence> {
    decoder
        .set_limits(image_limits())
        .map_err(|source| RimgError::ImageOpen {
            path: path.to_path_buf(),
            source,
        })?;
    let (width, height) = decoder.dimensions();
    check_dimensions(width, height)?;
    let mut budget = DecodeBudget::new(limits);
    let mut frames = Vec::new();
    let mut decoded = decoder.into_frames();
    while !budget.full() {
        let Some(frame) = decoded.next() else {
            break;
        };
        budget.check_frame(width, height)?;
        let frame = frame.map_err(|source| RimgError::FrameDecode {
            path: path.to_path_buf(),
            source,
        })?;
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay =
            Duration::from_secs_f64(f64::from(numerator) / f64::from(denominator.max(1)) / 1000.0);
        let mut pixels = DynamicImage::ImageRgba8(frame.into_buffer());
        if crop_border > 0 {
            pixels = autocrop::crop_border(pixels, crop_border);
        }
        if auto_crop {
            pixels = autocrop::auto_crop(pixels);
        }
        let frame = Frame {
            pixels: pixels.into_rgba8(),
            delay,
        };
        budget.add(&frame);
        frames.push(frame);
    }
    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames,
    })
}

fn load_dds(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
) -> Result<ImageSequence> {
    const DDS_MAGIC: &[u8; 4] = b"DDS ";
    const DDS_HEADER_LEN: usize = 124;
    const DDSD_PITCH: u32 = 0x0000_0008;

    const DDPF_ALPHAPIXELS: u32 = 0x0000_0001;
    const DDPF_FOURCC: u32 = 0x0000_0004;
    const DDPF_RGB: u32 = 0x0000_0040;

    fn u32_le(data: &[u8], offset: usize) -> Result<u32> {
        let bytes: [u8; 4] = data
            .get(offset..offset + 4)
            .ok_or_else(|| RimgError::other("DDS: header too short"))?
            .try_into()
            .expect("slice length checked");
        Ok(u32::from_le_bytes(bytes))
    }

    fn decode_with_image_dds(data: &[u8], path: &Path) -> Result<DynamicImage> {
        let decoder = image::codecs::dds::DdsDecoder::new(Cursor::new(data)).map_err(|err| {
            RimgError::ImageOpen {
                path: path.to_path_buf(),
                source: err,
            }
        })?;
        DynamicImage::from_decoder(decoder).map_err(|err| RimgError::ImageOpen {
            path: path.to_path_buf(),
            source: err,
        })
    }

    fn mask_to_scale(mask: u32) -> (u32, u32) {
        if mask == 0 {
            return (0, 0);
        }
        let shift = mask.trailing_zeros();
        let width = (mask >> shift).count_ones();
        (shift, width)
    }

    fn extract_channel(value: u32, mask: u32) -> u8 {
        if mask == 0 {
            return 0;
        }
        let (shift, width) = mask_to_scale(mask);
        if width == 0 {
            return 0;
        }
        let raw = (value & mask) >> shift;
        if width == 8 {
            raw as u8
        } else {
            let max = (1u64 << width) - 1;
            let raw = raw as u64;
            ((raw * 255) / max) as u8
        }
    }

    // These arguments mirror the fields of the DDS pixel-format header.
    #[allow(clippy::too_many_arguments)]
    fn decode_uncompressed(
        data: &[u8],
        data_offset: usize,
        flags: u32,
        width: u32,
        height: u32,
        pitch_or_linear_size: u32,
        rgb_bits: u32,
        r_mask: u32,
        g_mask: u32,
        b_mask: u32,
        a_mask: u32,
    ) -> Result<RgbaImage> {
        if rgb_bits == 0 || rgb_bits > 32 {
            return Err(RimgError::other(format!(
                "DDS: unsupported RGB bit depth {rgb_bits}"
            )));
        }

        let bytes_per_pixel = rgb_bits.div_ceil(8) as usize;
        if !(2..=4).contains(&bytes_per_pixel) {
            return Err(RimgError::other(format!(
                "DDS: unsupported bytes-per-pixel {bytes_per_pixel}"
            )));
        }

        let pitch = if (flags & DDSD_PITCH) != 0 && pitch_or_linear_size != 0 {
            pitch_or_linear_size as usize
        } else {
            width as usize * bytes_per_pixel
        };

        if pitch < width as usize * bytes_per_pixel {
            return Err(RimgError::other("DDS: pitch is shorter than a pixel row"));
        }

        let required = data_offset.saturating_add(pitch.saturating_mul(height as usize));
        if data.len() < required {
            return Err(RimgError::other("DDS: file truncated"));
        }

        let mut out = vec![0u8; (width as usize) * (height as usize) * 4];
        for y in 0..height as usize {
            let row = data_offset + y * pitch;
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let px = row + x * bytes_per_pixel;

                let mut value = 0u32;
                for i in 0..bytes_per_pixel {
                    value |= (data[px + i] as u32) << (8 * i);
                }

                out[idx] = extract_channel(value, r_mask);
                out[idx + 1] = extract_channel(value, g_mask);
                out[idx + 2] = extract_channel(value, b_mask);
                out[idx + 3] = if a_mask == 0 {
                    255
                } else {
                    extract_channel(value, a_mask)
                };
            }
        }

        RgbaImage::from_raw(width, height, out)
            .ok_or_else(|| RimgError::other("DDS: failed to build image buffer"))
    }

    let data = read_bounded(File::open(path)?)?;

    if data.len() < 4 + DDS_HEADER_LEN || &data[..4] != DDS_MAGIC {
        return Err(RimgError::other(format!(
            "failed to open {}: not a DDS file",
            path.display()
        )));
    }

    check_dimensions(u32_le(&data, 16)?, u32_le(&data, 12)?)?;

    // Try the `image` crate's DDS decoder first (supports DXT variants).
    // If it fails, fall back to uncompressed DDS decoding.
    let decoded = decode_with_image_dds(&data, path).or_else(|image_err| {
        // Parse legacy header for uncompressed RGB(A) formats.
        let header_base = 4;
        let header_flags = u32_le(&data, header_base + 4)?;
        let height = u32_le(&data, header_base + 8)?;
        let width = u32_le(&data, header_base + 12)?;
        let pitch_or_linear_size = u32_le(&data, header_base + 16)?;

        let pf_base = header_base + 72;
        let pf_flags = u32_le(&data, pf_base + 4)?;
        let mut fourcc = [0u8; 4];
        fourcc.copy_from_slice(
            data.get(pf_base + 8..pf_base + 12)
                .ok_or_else(|| RimgError::other("DDS: header too short"))?,
        );
        let rgb_bits = u32_le(&data, pf_base + 12)?;
        let mut r_mask = u32_le(&data, pf_base + 16)?;
        let mut g_mask = u32_le(&data, pf_base + 20)?;
        let mut b_mask = u32_le(&data, pf_base + 24)?;
        let mut a_mask = u32_le(&data, pf_base + 28)?;

        // DX10 header: handle the common uncompressed RGBA/BGRA formats.
        let mut data_offset = 4 + DDS_HEADER_LEN;
        if (pf_flags & DDPF_FOURCC) != 0 && &fourcc == b"DX10" {
            let dx10_base = data_offset;
            if data.len() < dx10_base + 20 {
                return Err(RimgError::other("DDS: DX10 header truncated"));
            }
            let dxgi_format = u32_le(&data, dx10_base)?;
            data_offset += 20;
            match dxgi_format {
                28 => {
                    // DXGI_FORMAT_R8G8B8A8_UNORM
                    r_mask = 0x0000_00ff;
                    g_mask = 0x0000_ff00;
                    b_mask = 0x00ff_0000;
                    a_mask = 0xff00_0000;
                }
                87 => {
                    // DXGI_FORMAT_B8G8R8A8_UNORM
                    r_mask = 0x00ff_0000;
                    g_mask = 0x0000_ff00;
                    b_mask = 0x0000_00ff;
                    a_mask = 0xff00_0000;
                }
                _ => return Err(image_err),
            }
        }

        if (pf_flags & DDPF_RGB) == 0 {
            return Err(image_err);
        }
        if (pf_flags & DDPF_ALPHAPIXELS) == 0 {
            a_mask = 0;
        }

        let rgba = decode_uncompressed(
            &data,
            data_offset,
            header_flags,
            width,
            height,
            pitch_or_linear_size,
            rgb_bits,
            r_mask,
            g_mask,
            b_mask,
            a_mask,
        )?;

        Ok(DynamicImage::ImageRgba8(rgba))
    })?;

    let mut image = decoded;

    // Apply rotation if enabled (DDS typically doesn't have EXIF, but check anyway)
    if rotation_mode == RotationMode::Exif
        && let Some(orientation) = read_exif_orientation(path)
    {
        image = apply_orientation(image, orientation);
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

#[cfg(feature = "qoi")]
fn load_qoi(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
) -> Result<ImageSequence> {
    // Read entire file into memory (QOI decoder needs bytes)
    let data = read_bounded(File::open(path)?)?;

    if data.len() >= 14 && &data[..4] == b"qoif" {
        check_dimensions(
            u32::from_be_bytes(data[4..8].try_into().unwrap()),
            u32::from_be_bytes(data[8..12].try_into().unwrap()),
        )?;
    }

    let (header, data) = qoi::decode_to_vec(&data).map_err(|err| {
        RimgError::other(format!(
            "QOI decode error for '{}': {}",
            path.display(),
            err
        ))
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
    if rotation_mode == RotationMode::Exif
        && let Some(orientation) = read_exif_orientation(path)
    {
        image = apply_orientation(image, orientation);
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
    use image::imageops::{flip_horizontal, flip_vertical, rotate90, rotate180, rotate270};

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
fn load_pdf(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
    limits: LoadLimits,
) -> Result<ImageSequence> {
    use pdfium_render::prelude::*;

    fn bind_pdfium() -> std::result::Result<Box<dyn PdfiumLibraryBindings>, PdfiumError> {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();

        if let Ok(path) = std::env::var("SHOWME_PDFIUM_PATH")
            && !path.trim().is_empty()
        {
            candidates.push(std::path::PathBuf::from(path));
        }

        if let Ok(dir) = std::env::var("SHOWME_PDFIUM_DIR") {
            let dir = dir.trim();
            if !dir.is_empty() {
                candidates.push(Pdfium::pdfium_platform_library_name_at_path(
                    std::path::Path::new(dir),
                ));
            }
        }

        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            candidates.push(Pdfium::pdfium_platform_library_name_at_path(dir));
            candidates.push(Pdfium::pdfium_platform_library_name_at_path(
                &dir.join("lib"),
            ));
        }

        // Backwards-compatible: current working directory.
        candidates.push(Pdfium::pdfium_platform_library_name_at_path(
            std::path::Path::new("."),
        ));

        // Common install locations.
        candidates.push(Pdfium::pdfium_platform_library_name_at_path(
            std::path::Path::new("/opt/homebrew/lib"),
        ));
        candidates.push(Pdfium::pdfium_platform_library_name_at_path(
            std::path::Path::new("/usr/local/lib"),
        ));

        // Try each explicit candidate path, then fall back to system search.
        for candidate in candidates {
            if let Ok(bindings) = Pdfium::bind_to_library(&candidate) {
                return Ok(bindings);
            }
        }

        Pdfium::bind_to_system_library()
    }

    // Initialize Pdfium library
    let pdfium = match bind_pdfium() {
        Ok(bindings) => Pdfium::new(bindings),
        // pdfium-render 0.9 promotes bindings into a process-wide OnceCell.
        // Its Default implementation explicitly reuses initialized bindings.
        Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Pdfium::default(),
        Err(err) => {
            return Err(RimgError::other(format!(
                "Failed to initialize PDF library: {err}"
            )));
        }
    };

    // Load the PDF document
    let document = pdfium.load_pdf_from_file(path, None).map_err(|err| {
        RimgError::other(format!("Failed to load PDF '{}': {}", path.display(), err))
    })?;

    let page_count = document.pages().len();
    if page_count == 0 {
        return Err(RimgError::other(format!(
            "PDF '{}' has no pages",
            path.display()
        )));
    }

    let mut frames = Vec::new();
    let mut budget = DecodeBudget::new(limits);

    // Render each page
    for page_index in 0..page_count {
        if budget.full() {
            break;
        }
        if page_index >= 128 {
            return Err(RimgError::other(
                "PDF exceeds the 128-page decode limit; select fewer pages with --frames",
            ));
        }
        let page = document.pages().get(page_index).map_err(|err| {
            RimgError::other(format!(
                "Failed to get page {} from PDF: {}",
                page_index, err
            ))
        })?;

        // Render at 150 DPI for good quality
        let render_config = PdfRenderConfig::new()
            .set_target_width(2000)
            .set_maximum_width(4000)
            .set_maximum_height(4000);

        let bitmap = page.render_with_config(&render_config).map_err(|err| {
            RimgError::other(format!("Failed to render PDF page {}: {}", page_index, err))
        })?;

        // Convert bitmap to RgbaImage
        let width = bitmap.width() as u32;
        let height = bitmap.height() as u32;

        budget.check_frame(width, height)?;
        // Let pdfium-render handle bitmap format, stride, and channel order.
        let mut image = bitmap
            .as_image()
            .map_err(|err| RimgError::other(format!("failed to convert PDF page bitmap: {err}")))?;

        // Apply rotation if needed (PDFs don't typically have EXIF, but check anyway)
        if rotation_mode == RotationMode::Exif
            && let Some(orientation) = read_exif_orientation(path)
        {
            image = apply_orientation(image, orientation);
        }

        // Apply cropping: first fixed border, then auto-crop
        if crop_border > 0 {
            image = autocrop::crop_border(image, crop_border);
        }
        if auto_crop {
            image = autocrop::auto_crop(image);
        }

        let frame = Frame::single(image.to_rgba8());
        budget.add(&frame);
        frames.push(frame);
    }

    Ok(ImageSequence {
        path: path.to_path_buf(),
        frames,
    })
}

#[cfg(feature = "svg")]
fn load_svg(
    path: &Path,
    rotation_mode: RotationMode,
    auto_crop: bool,
    crop_border: u32,
) -> Result<ImageSequence> {
    use resvg::usvg;

    // Read SVG file
    let svg_data = read_bounded(File::open(path)?)?;

    // Bound decompressed SVGZ bytes before passing them to the XML parser.
    let svg_data = if svg_data.starts_with(&[0x1f, 0x8b]) {
        read_bounded(flate2::read::GzDecoder::new(svg_data.as_slice()))?
    } else {
        svg_data
    };

    // Parse SVG
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(&svg_data, &options).map_err(|err| {
        RimgError::other(format!("Failed to parse SVG '{}': {}", path.display(), err))
    })?;

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

    check_dimensions(width, height)?;

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
    if rotation_mode == RotationMode::Exif
        && let Some(orientation) = read_exif_orientation(path)
    {
        image = apply_orientation(image, orientation);
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

#[cfg(test)]
mod orientation_tests {
    use super::*;

    #[test]
    fn all_eight_exif_transforms_move_pixels_to_expected_coordinates() {
        let pixels =
            RgbaImage::from_fn(3, 2, |x, y| image::Rgba([(y * 3 + x + 1) as u8, 0, 0, 255]));
        let cases = [
            (1, (3, 2), vec![1, 2, 3, 4, 5, 6]),
            (2, (3, 2), vec![3, 2, 1, 6, 5, 4]),
            (3, (3, 2), vec![6, 5, 4, 3, 2, 1]),
            (4, (3, 2), vec![4, 5, 6, 1, 2, 3]),
            (5, (2, 3), vec![1, 4, 2, 5, 3, 6]),
            (6, (2, 3), vec![4, 1, 5, 2, 6, 3]),
            (7, (2, 3), vec![6, 3, 5, 2, 4, 1]),
            (8, (2, 3), vec![3, 6, 2, 5, 1, 4]),
        ];
        for (orientation, dimensions, expected) in cases {
            let transformed =
                apply_orientation(DynamicImage::ImageRgba8(pixels.clone()), orientation)
                    .into_rgba8();
            assert_eq!(
                transformed.dimensions(),
                dimensions,
                "orientation {orientation}"
            );
            assert_eq!(
                transformed
                    .pixels()
                    .map(|pixel| pixel[0])
                    .collect::<Vec<_>>(),
                expected,
                "orientation {orientation}"
            );
        }
    }
}
