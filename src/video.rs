use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use ffmpeg_next as ffmpeg;

use crate::error::{Result, RimgError};
use crate::image::{DecodeBudget, Frame, ImageSequence, LoadLimits, check_dimensions};

static FFMPEG_INIT: OnceLock<std::result::Result<(), ffmpeg::Error>> = OnceLock::new();

pub struct VideoLoader;

impl VideoLoader {
    pub fn is_video_candidate(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                matches!(
                    ext.to_ascii_lowercase().as_str(),
                    "mp4" | "mkv" | "mov" | "avi" | "webm" | "mpg" | "mpeg" | "gifv" | "flv"
                )
            })
            .unwrap_or(false)
    }

    fn ensure_initialized() -> Result<()> {
        FFMPEG_INIT
            .get_or_init(ffmpeg::init)
            .as_ref()
            .map_err(|err| RimgError::other(format!("failed to initialize FFmpeg: {err}")))?;
        Ok(())
    }
}

pub fn load_video(path: &Path) -> Result<Option<ImageSequence>> {
    load_video_with_limits(path, LoadLimits::default())
}

pub(crate) fn load_video_with_limits(
    path: &Path,
    limits: LoadLimits,
) -> Result<Option<ImageSequence>> {
    limits.check_cancelled()?;
    VideoLoader::ensure_initialized()?;
    let mut budget = DecodeBudget::new(limits);
    if budget.full() {
        return Ok(Some(ImageSequence {
            path: path.to_path_buf(),
            frames: Vec::new(),
        }));
    }

    let mut input = match ffmpeg::format::input(&path) {
        Ok(ctx) => ctx,
        Err(_) => return Ok(None),
    };

    let video_stream = match input.streams().best(ffmpeg::media::Type::Video) {
        Some(stream) => stream,
        None => return Ok(None),
    };

    let stream_index = video_stream.index();
    let time_base = video_stream.time_base();
    let avg_frame_rate = video_stream.avg_frame_rate();

    let codec_context = ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
        .map_err(|err| RimgError::other(format!("failed to load stream parameters: {err}")))?;

    let mut decoder = codec_context
        .decoder()
        .video()
        .map_err(|err| RimgError::other(format!("ffmpeg failed to create decoder: {err}")))?;

    let target_width = decoder.width();
    let target_height = decoder.height();
    budget.check_frame(target_width, target_height)?;

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        target_width,
        target_height,
        ffmpeg::format::Pixel::RGBA,
        target_width,
        target_height,
        ffmpeg::software::scaling::flag::Flags::BILINEAR,
    )
    .map_err(|err| RimgError::other(format!("failed to create scaler: {err}")))?;

    let mut frames = Vec::new();

    let mut last_pts: Option<i64> = None;

    for (stream, packet) in input.packets() {
        if stream.index() == stream_index {
            decoder
                .send_packet(&packet)
                .map_err(|err| RimgError::other(format!("decoder send_packet failed: {err}")))?;
            receive_frames(
                &mut decoder,
                &mut scaler,
                target_width,
                target_height,
                time_base,
                avg_frame_rate,
                &mut last_pts,
                &mut frames,
                &mut budget,
            )?;
            if budget.full() {
                break;
            }
        }
    }

    if !budget.full() {
        decoder
            .send_eof()
            .map_err(|err| RimgError::other(format!("decoder send_eof failed: {err}")))?;
        receive_frames(
            &mut decoder,
            &mut scaler,
            target_width,
            target_height,
            time_base,
            avg_frame_rate,
            &mut last_pts,
            &mut frames,
            &mut budget,
        )?;
    }

    if frames.is_empty() {
        return Ok(None);
    }

    Ok(Some(ImageSequence {
        path: path.to_path_buf(),
        frames,
    }))
}

// Decoder state and stream timing are explicit at the FFmpeg receive boundary.
#[allow(clippy::too_many_arguments)]
fn receive_frames(
    decoder: &mut ffmpeg::codec::decoder::Video,
    scaler: &mut ffmpeg::software::scaling::context::Context,
    width: u32,
    height: u32,
    time_base: ffmpeg::Rational,
    avg_frame_rate: ffmpeg::Rational,
    last_pts: &mut Option<i64>,
    output: &mut Vec<Frame>,
    budget: &mut DecodeBudget,
) -> Result<()> {
    let mut decoded = ffmpeg::util::frame::Video::empty();
    let mut converted = ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, width, height);

    while !budget.full() {
        match decoder.receive_frame(&mut decoded) {
            Ok(()) => {}
            Err(ffmpeg::Error::Eof) => break,
            Err(ffmpeg::Error::Other { errno }) if errno == libc::EAGAIN => break,
            Err(err) => {
                return Err(RimgError::other(format!(
                    "video frame decode failed: {err}"
                )));
            }
        }
        budget.check_frame(width, height)?;
        scaler
            .run(&decoded, &mut converted)
            .map_err(|err| RimgError::other(format!("failed to scale video frame: {err}")))?;

        let image = frame_to_image(&converted, width, height)?;
        let pts = decoded.pts();
        let delay = determine_delay(pts, time_base, avg_frame_rate, last_pts);

        let frame = Frame {
            pixels: image,
            delay,
        };
        budget.add(&frame);
        output.push(frame);
    }

    Ok(())
}

fn determine_delay(
    pts: Option<i64>,
    time_base: ffmpeg::Rational,
    avg_frame_rate: ffmpeg::Rational,
    last_pts: &mut Option<i64>,
) -> Duration {
    if let Some(pts_value) = pts {
        if let Some(prev_pts) = last_pts {
            if time_base.denominator() != 0 {
                let seconds = pts_value.saturating_sub(*prev_pts) as f64
                    * time_base.numerator() as f64
                    / time_base.denominator() as f64;
                if seconds > 0.0
                    && let Ok(delay) = Duration::try_from_secs_f64(seconds)
                {
                    *last_pts = Some(pts_value);
                    return delay;
                }
            }
        } else {
            *last_pts = Some(pts_value);
        }
    }

    // Fallback: use average frame rate if available.
    if avg_frame_rate.denominator() > 0 && avg_frame_rate.numerator() > 0 {
        let fps = avg_frame_rate.numerator() as f64 / avg_frame_rate.denominator() as f64;
        if fps > 0.0 {
            return Duration::from_secs_f64(1.0 / fps);
        }
    }

    Duration::from_millis(0)
}

fn frame_to_image(
    frame: &ffmpeg::util::frame::Video,
    width: u32,
    height: u32,
) -> Result<image::RgbaImage> {
    check_dimensions(width, height)?;
    if frame.width() != width || frame.height() != height {
        return Err(RimgError::other(
            "video frame dimensions do not match the RGBA conversion target",
        ));
    }
    let stride = frame.stride(0);
    let data = frame.data(0);
    let row_bytes = width as usize * 4;
    let required = stride
        .saturating_mul(height as usize - 1)
        .saturating_add(row_bytes);
    if stride < row_bytes || data.len() < required {
        return Err(RimgError::other(
            "invalid RGBA video frame stride or data length",
        ));
    }
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    for y in 0..height as usize {
        let src_offset = y * stride;
        let dst_offset = y * width as usize * 4;
        let src = &data[src_offset..src_offset + width as usize * 4];
        let dst = &mut buffer[dst_offset..dst_offset + width as usize * 4];
        dst.copy_from_slice(src);
    }
    image::RgbaImage::from_raw(width, height, buffer)
        .ok_or_else(|| RimgError::other("failed to create RGBA image from frame"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_conversion_respects_native_row_padding() {
        let mut native = ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, 3, 2);
        let stride = native.stride(0);
        native.data_mut(0).fill(0xee);
        for y in 0..2 {
            for x in 0..3 {
                let offset = y * stride + x * 4;
                native.data_mut(0)[offset..offset + 4]
                    .copy_from_slice(&[x as u8, y as u8, 90, 255]);
            }
        }
        let image = frame_to_image(&native, 3, 2).unwrap();
        for (x, y, pixel) in image.enumerate_pixels() {
            assert_eq!(pixel.0, [x as u8, y as u8, 90, 255]);
        }
    }

    #[test]
    fn invalid_video_frame_geometry_returns_an_error() {
        let native = ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, 3, 2);
        assert!(frame_to_image(&native, 4, 2).is_err());
        assert!(frame_to_image(&native, 0, 2).is_err());
    }

    #[test]
    fn invalid_timestamps_use_frame_rate_without_overflowing() {
        let mut previous = Some(i64::MIN);
        let delay = determine_delay(
            Some(i64::MAX),
            ffmpeg::Rational(i32::MAX, 1),
            ffmpeg::Rational(50, 1),
            &mut previous,
        );
        assert_eq!(delay, Duration::from_millis(20));
    }
}
