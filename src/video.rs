use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use ffmpeg_next as ffmpeg;

use crate::error::{Result, RimgError};
use crate::image::{Frame, ImageSequence};

static FFMPEG_INIT: OnceLock<()> = OnceLock::new();

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

    fn ensure_initialized() {
        FFMPEG_INIT.get_or_init(|| {
            let _ = ffmpeg::init();
        });
    }
}

pub fn load_video(path: &Path) -> Result<Option<ImageSequence>> {
    VideoLoader::ensure_initialized();

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

    let codec_context =
        ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())
            .map_err(|err| RimgError::other(format!("failed to load stream parameters: {err}")))?;

    let mut decoder = codec_context
        .decoder()
        .video()
        .map_err(|err| RimgError::other(format!("ffmpeg failed to create decoder: {err}")))?;

    let target_width = decoder.width();
    let target_height = decoder.height();

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
            )?;
        }
    }

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
    )?;

    if frames.is_empty() {
        return Ok(None);
    }

    Ok(Some(ImageSequence {
        path: path.to_path_buf(),
        frames,
    }))
}

fn receive_frames(
    decoder: &mut ffmpeg::codec::decoder::Video,
    scaler: &mut ffmpeg::software::scaling::context::Context,
    width: u32,
    height: u32,
    time_base: ffmpeg::Rational,
    avg_frame_rate: ffmpeg::Rational,
    last_pts: &mut Option<i64>,
    output: &mut Vec<Frame>,
) -> Result<()> {
    let mut decoded = ffmpeg::util::frame::Video::empty();
    let mut converted = ffmpeg::util::frame::Video::new(ffmpeg::format::Pixel::RGBA, width, height);

    while decoder.receive_frame(&mut decoded).is_ok() {
        scaler
            .run(&decoded, &mut converted)
            .map_err(|err| RimgError::other(format!("failed to scale video frame: {err}")))?;

        let image = frame_to_image(&converted, width, height)?;
        let pts = decoded.pts();
        let delay = determine_delay(pts, time_base, avg_frame_rate, last_pts);

        output.push(Frame {
            pixels: image,
            delay,
        });
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
                let seconds = (pts_value - *prev_pts) as f64 * time_base.numerator() as f64
                    / time_base.denominator() as f64;
                if seconds > 0.0 {
                    *last_pts = Some(pts_value);
                    return Duration::from_secs_f64(seconds);
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
    let stride = frame.stride(0) as usize;
    let data = frame.data(0);
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
