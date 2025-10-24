use std::ffi::{c_char, c_int, c_void};
use std::ptr;

use sixel_sys as sys;

use super::image_util::{blend_transparency, scale_frame};
use crate::backend::{Backend, RenderOptions, RenderedFrame};
use crate::config::BackendKind;
use crate::error::{Result, RimgError};
use crate::image::Frame;

#[derive(Debug, Default)]
pub struct SixelBackend;

impl Backend for SixelBackend {
    fn name(&self) -> &'static str {
        "sixel"
    }

    fn supported_kind(&self) -> BackendKind {
        BackendKind::Sixel
    }

    fn render(&self, frame: &Frame, options: RenderOptions) -> Result<RenderedFrame> {
        let (mut image, width_cells, _height_cells) = scale_frame(frame, options);
        blend_transparency(&mut image, options.background);

        let sixel_data = encode_sixel(image)?;
        let mut lines = Vec::with_capacity(1);
        lines.push(sixel_data);

        Ok(RenderedFrame {
            lines,
            width_cells,
            height_cells: 0,
            delay: frame.delay,
        })
    }
}

fn encode_sixel(image: image::RgbaImage) -> Result<String> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut data = image.into_raw();

    let remainder = height % 6;
    let mut padded_height = height;
    if remainder != 0 {
        let add_rows = 6 - remainder;
        data.extend(std::iter::repeat(0).take(add_rows * width * 4));
        padded_height += add_rows;
    }

    let mut buffer: Vec<u8> = Vec::with_capacity(width * padded_height);
    unsafe {
        let mut output: *mut sys::Output = ptr::null_mut();
        check_status(
            sys::sixel_output_new(
                &mut output,
                Some(write_callback),
                &mut buffer as *mut _ as *mut c_void,
                ptr::null_mut(),
            ),
            "failed to create sixel output",
        )?;
        let output_guard = OutputGuard { ptr: output };

        sys::sixel_output_set_8bit_availability(output_guard.ptr, sys::CharacterSize::EightBit);
        sys::sixel_output_set_encode_policy(output_guard.ptr, sys::EncodePolicy::Fast);

        let mut dither: *mut sys::Dither = ptr::null_mut();
        check_status(
            sys::sixel_dither_new(&mut dither, 256, ptr::null_mut()),
            "failed to create sixel dither",
        )?;
        let dither_guard = DitherGuard { ptr: dither };

        check_status(
            sys::sixel_dither_initialize(
                dither_guard.ptr,
                data.as_mut_ptr(),
                width as c_int,
                padded_height as c_int,
                sys::PixelFormat::RGBA8888,
                sys::MethodForLargest::Auto,
                sys::MethodForRepColor::AverageColor,
                sys::QualityMode::Auto,
            ),
            "failed to initialise sixel dither",
        )?;

        sys::sixel_dither_set_transparent(dither_guard.ptr, 1);

        check_status(
            sys::sixel_encode(
                data.as_mut_ptr(),
                width as c_int,
                padded_height as c_int,
                0,
                dither_guard.ptr,
                output_guard.ptr,
            ),
            "failed to encode sixel image",
        )?;
    }

    let mut text = String::from_utf8(buffer)
        .map_err(|_| RimgError::other("sixel encoder produced invalid UTF-8"))?;

    if !text.ends_with('\n') {
        text.push('\n');
    }

    Ok(text)
}

unsafe extern "C" fn write_callback(data: *mut c_char, size: c_int, priv_: *mut c_void) -> c_int {
    if size <= 0 || priv_.is_null() {
        return 0;
    }

    let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size as usize) };
    let output = unsafe { &mut *(priv_ as *mut Vec<u8>) };
    output.extend_from_slice(slice);
    size
}

struct OutputGuard {
    ptr: *mut sys::Output,
}

impl Drop for OutputGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                sys::sixel_output_destroy(self.ptr);
            }
        }
    }
}

struct DitherGuard {
    ptr: *mut sys::Dither,
}

impl Drop for DitherGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.ptr.is_null() {
                sys::sixel_dither_destroy(self.ptr);
            }
        }
    }
}

fn check_status(status: i32, message: &str) -> Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(RimgError::other(format!("{message}: status {status}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::TerminalSize;
    use crate::config::RenderSizing;
    use crate::image::Frame;
    use image::{ImageBuffer, Rgba};
    use std::time::Duration;

    #[test]
    fn sixel_output_has_expected_prefix() {
        let frame = Frame {
            pixels: ImageBuffer::from_pixel(1, 1, Rgba([0, 0, 0, 255])),
            delay: Duration::ZERO,
        };
        let backend = SixelBackend;
        let rendered = backend
            .render(
                &frame,
                RenderOptions {
                    sizing: RenderSizing::unconstrained(),
                    terminal: TerminalSize {
                        columns: 80,
                        rows: 24,
                    },
                    background: crate::backend::BackgroundStyle {
                        color: None,
                        pattern: None,
                        pattern_size: 1,
                    },
                    pixelation: crate::config::PixelationMode::Quarter,
                    use_8bit_color: false,
                    compress_level: 1,
                },
            )
            .expect("render sixel");

        let output = &rendered.lines[0];
        assert!(output.starts_with("\x1bP"));
        assert!(output.contains("\x1b\\"));
    }
}
