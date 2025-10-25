# showme Demo Script

An interactive demonstration script showcasing all of `showme`'s capabilities.

## Quick Start

```bash
./demo.sh
```

This will run a comprehensive demo featuring:

## Features Demonstrated

1. **Basic Image Viewing** - Auto-detection and display
2. **Kitty Graphics Protocol** - Full-resolution, crisp rendering 🔥
3. **Unicode Rendering** - Quarter-block and half-block modes
4. **Grid Layouts** - Multiple images in grid format
5. **SVG Support** - Vector graphics rendering
6. **Custom Sizing** - Width/height constraints and fit modes
7. **Slideshows** - Timed displays with titles
8. **Video Playback** - Rick Roll demonstration 🎵
9. **Advanced Features** - Centering, backgrounds, positioning
10. **Backend Comparison** - Kitty vs iTerm2 vs Unicode

## Requirements

### Required
- `showme` - The terminal media viewer
- `curl` - For downloading sample images

### Optional (for full demo)
- `yt-dlp` - For video demonstration
  ```bash
  pip install yt-dlp
  ```

## Screen Recording

Perfect for creating demo videos:

```bash
# Using asciinema
asciinema rec showme-demo.cast -c ./demo.sh

# Using OBS or similar
# Just run ./demo.sh and capture your terminal
```

## Demo Assets

The script will:
- Use NoGood wallpapers from `~/Downloads/NoGood - Wallpaper Pack 03/` if available (🔥 recommended!)
- Fall back to downloading sample images from picsum.photos
- Generate an SVG logo
- Download Rick Astley's "Never Gonna Give You Up" for video demo

## Customization

Edit the timing in `demo.sh`:

```bash
SLEEP_SHORT=2   # Short pause between sections
SLEEP_MEDIUM=3  # Medium pause
SLEEP_LONG=4    # Long pause for complex demos
```

## Cleanup

The demo creates files in `/tmp/showme-demo/`. At the end, you'll be prompted to clean up or keep the files.

To manually clean up:
```bash
rm -rf /tmp/showme-demo
```

## What Makes This Demo Special

- **Visual Polish**: ASCII art banners and colored output
- **Clear Narration**: Each feature is explained before demonstration
- **Command Preview**: See the actual command before execution
- **Premium Content**: Uses high-quality NoGood wallpapers if available
- **Rick Roll**: Because why not? 🎸

## Tips for Best Results

1. **Use Kitty Terminal** - Shows off the high-resolution graphics
2. **Maximize Window** - Better viewing experience
3. **Dark Theme** - Most images look better on dark backgrounds
4. **Good Lighting** - For screen recording

Enjoy the show! 🎬
