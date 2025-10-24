// 8-bit (256 color) quantization for terminals
// Uses xterm-256 color palette

use crate::config::RgbColor;

// xterm-256 color palette
// Colors 0-15: System colors (varies by terminal)
// Colors 16-231: 6x6x6 RGB cube
// Colors 232-255: Grayscale ramp

pub fn rgb_to_256(r: u8, g: u8, b: u8) -> u8 {
    // Check if it's a grayscale color
    let max_diff = r.abs_diff(g).max(r.abs_diff(b)).max(g.abs_diff(b));
    if max_diff < 8 {
        // Use grayscale ramp (232-255)
        // 24 shades from black to white
        let gray = (r as u16 + g as u16 + b as u16) / 3;
        if gray < 4 {
            return 16; // Black from RGB cube
        } else if gray > 247 {
            return 231; // White from RGB cube
        } else {
            let index = ((gray - 4) * 24 / 244).min(23) as u8;
            return 232 + index;
        }
    }

    // Use 6x6x6 RGB cube (colors 16-231)
    // Each channel is quantized to 6 levels: 0, 95, 135, 175, 215, 255
    let r_idx = quantize_channel_to_6(r);
    let g_idx = quantize_channel_to_6(g);
    let b_idx = quantize_channel_to_6(b);

    16 + 36 * r_idx + 6 * g_idx + b_idx
}

fn quantize_channel_to_6(value: u8) -> u8 {
    // Map 0-255 to one of 6 levels: 0, 95, 135, 175, 215, 255
    // Thresholds chosen to minimize error
    if value < 48 {
        0
    } else if value < 115 {
        1
    } else if value < 155 {
        2
    } else if value < 195 {
        3
    } else if value < 235 {
        4
    } else {
        5
    }
}

// Convert 256-color index back to approximate RGB (for testing)
#[allow(dead_code)]
pub fn color_256_to_rgb(index: u8) -> RgbColor {
    match index {
        // Grayscale ramp (232-255)
        232..=255 => {
            let level = ((index - 232) * 10 + 4) as u8;
            RgbColor { r: level, g: level, b: level }
        }
        // RGB cube (16-231)
        16..=231 => {
            let idx = index - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            RgbColor {
                r: channel_6_to_rgb(r),
                g: channel_6_to_rgb(g),
                b: channel_6_to_rgb(b),
            }
        }
        // System colors - approximate
        _ => RgbColor { r: 128, g: 128, b: 128 },
    }
}

fn channel_6_to_rgb(level: u8) -> u8 {
    match level {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        5 => 255,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_and_white() {
        assert_eq!(rgb_to_256(0, 0, 0), 16); // Black
        assert_eq!(rgb_to_256(255, 255, 255), 231); // White
    }

    #[test]
    fn test_primary_colors() {
        let red = rgb_to_256(255, 0, 0);
        let green = rgb_to_256(0, 255, 0);
        let blue = rgb_to_256(0, 0, 255);

        // Should map to corners of RGB cube
        assert_eq!(red, 16 + 36 * 5); // (5,0,0)
        assert_eq!(green, 16 + 6 * 5); // (0,5,0)
        assert_eq!(blue, 16 + 5); // (0,0,5)
    }

    #[test]
    fn test_grayscale() {
        let gray_dark = rgb_to_256(50, 50, 50);
        let gray_mid = rgb_to_256(128, 128, 128);
        let gray_light = rgb_to_256(200, 200, 200);

        // All should be in grayscale ramp (232-255)
        assert!(gray_dark >= 232 && gray_dark <= 255);
        assert!(gray_mid >= 232 && gray_mid <= 255);
        assert!(gray_light >= 232 && gray_light <= 255);
        assert!(gray_dark < gray_mid);
        assert!(gray_mid < gray_light);
    }
}
