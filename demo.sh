#!/bin/bash
#
# showme - Terminal Media Viewer Demo
# A comprehensive showcase of showme's capabilities
#

set -e

# Colors for text output
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
RESET='\033[0m'

# Demo configuration
DEMO_DIR="/tmp/showme-demo"
SLEEP_SHORT=2
SLEEP_MEDIUM=3
SLEEP_LONG=4

# Banner function
banner() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${CYAN}║$(printf '%62s' | tr ' ' ' ')║${RESET}"
    printf "${CYAN}║${YELLOW}%62s${CYAN}║${RESET}\n" "$1"
    echo -e "${CYAN}║$(printf '%62s' | tr ' ' ' ')║${RESET}"
    echo -e "${CYAN}╚════════════════════════════════════════════════════════════════╝${RESET}"
    echo ""
}

# Info function
info() {
    echo -e "${GREEN}➜${RESET} $1"
}

# Command preview function
show_command() {
    echo -e "${MAGENTA}$ $1${RESET}"
    sleep 1
}

# Setup demo directory
setup_demo() {
    banner "Setting up demo environment..."
    mkdir -p "$DEMO_DIR"
    cd "$DEMO_DIR"
    info "Demo directory: $DEMO_DIR"
    sleep $SLEEP_SHORT
}

# Download sample images
download_samples() {
    banner "Preparing stunning visuals... 🔥"

    # Check for those fire NoGood wallpapers!
    NOGOOD_DIR="/home/sandwich/Downloads/NoGood - Wallpaper Pack 03"

    if [ -d "$NOGOOD_DIR" ]; then
        info "Found NoGood Wallpaper Pack! Using premium wallpapers 🎨"

        # Copy a few different ones for variety
        [ ! -f "wallpaper1.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_001.jpg" wallpaper1.jpg 2>/dev/null || true
        [ ! -f "wallpaper2.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_002.jpg" wallpaper2.jpg 2>/dev/null || true
        [ ! -f "wallpaper3.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_003.jpg" wallpaper3.jpg 2>/dev/null || true

        # Use first one as main wallpaper
        [ ! -f "wallpaper.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_001.jpg" wallpaper.jpg

        # These are portrait, perfect for portrait demo
        [ ! -f "portrait.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_001.jpg" portrait.jpg

        # Create a symlink for landscape demo (or copy another)
        if [ -f "$NOGOOD_DIR/NoGood_WallpaperPack_002.jpg" ]; then
            [ ! -f "landscape.jpg" ] && cp "$NOGOOD_DIR/NoGood_WallpaperPack_002.jpg" landscape.jpg
        fi

        info "Using premium NoGood wallpapers - these are FIRE! 🔥"
    else
        info "Downloading sample images..."
        [ ! -f "wallpaper.jpg" ] && curl -sL "https://picsum.photos/1920/1080" -o wallpaper.jpg
        [ ! -f "portrait.jpg" ] && curl -sL "https://picsum.photos/1080/1920" -o portrait.jpg
        [ ! -f "landscape.jpg" ] && curl -sL "https://picsum.photos/2560/1440" -o landscape.jpg
    fi

    # Create a simple SVG
    if [ ! -f "logo.svg" ]; then
        info "Creating sample SVG..."
        cat > logo.svg <<'EOF'
<svg width="400" height="400" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="400" fill="#1e1e1e"/>
  <circle cx="200" cy="200" r="150" fill="none" stroke="#00ff00" stroke-width="10"/>
  <text x="200" y="220" font-family="monospace" font-size="48" fill="#00ff00" text-anchor="middle">showme</text>
</svg>
EOF
    fi

    sleep $SLEEP_SHORT
}

# Download Rick Roll video
download_rickroll() {
    banner "Downloading Rick Roll... 🎵"

    if [ ! -f "rickroll.mp4" ]; then
        info "Using yt-dlp to download Never Gonna Give You Up..."
        show_command "yt-dlp -f 'bestvideo[height<=720]+bestaudio/best[height<=720]' --merge-output-format mp4 -o rickroll.mp4 'https://www.youtube.com/watch?v=dQw4w9WgXcQ'"

        # Download with quality limit to keep file size reasonable
        yt-dlp -f 'bestvideo[height<=720]+bestaudio/best[height<=720]' \
               --merge-output-format mp4 \
               -o rickroll.mp4 \
               'https://www.youtube.com/watch?v=dQw4w9WgXcQ' 2>&1 | grep -E "Downloading|Merging" || true
    else
        info "Rick Roll video already downloaded!"
    fi

    sleep $SLEEP_SHORT
}

# Demo 1: Basic image viewing
demo_basic() {
    banner "DEMO 1: Basic Image Viewing"
    info "Display a high-resolution wallpaper with auto-detection"
    sleep $SLEEP_SHORT

    show_command "showme wallpaper.jpg"
    showme wallpaper.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 2: Kitty Graphics Protocol
demo_kitty() {
    banner "DEMO 2: Kitty Graphics Protocol (High-Res)"
    info "Force Kitty backend for crisp, full-resolution rendering"
    sleep $SLEEP_SHORT

    show_command "showme --backend kitty --verbose wallpaper.jpg"
    showme --backend kitty --verbose wallpaper.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 3: Unicode rendering modes
demo_unicode() {
    banner "DEMO 3: Unicode Block Rendering"
    info "Quarter-block mode (default) - best detail"
    sleep $SLEEP_SHORT

    show_command "showme --backend unicode -p quarter portrait.jpg"
    showme --backend unicode -p quarter portrait.jpg

    sleep $SLEEP_MEDIUM

    info "Half-block mode - better color accuracy"
    sleep $SLEEP_SHORT

    show_command "showme --backend unicode -p half portrait.jpg"
    showme --backend unicode -p half portrait.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 4: Image grid layout
demo_grid() {
    banner "DEMO 4: Grid Layout"
    info "Display multiple images in a 2x2 grid"
    sleep $SLEEP_SHORT

    # Use NoGood wallpapers if available for extra fire
    if [ -f "wallpaper1.jpg" ] && [ -f "wallpaper2.jpg" ] && [ -f "wallpaper3.jpg" ]; then
        show_command "showme --grid 2 wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg"
        showme --grid 2 wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg
        info "NoGood wallpapers looking CRISP in grid mode! 🔥"
    else
        show_command "showme --grid 2 wallpaper.jpg portrait.jpg landscape.jpg wallpaper.jpg"
        showme --grid 2 wallpaper.jpg portrait.jpg landscape.jpg wallpaper.jpg
    fi

    sleep $SLEEP_LONG
}

# Demo 5: SVG rendering
demo_svg() {
    banner "DEMO 5: SVG Vector Graphics"
    info "Render SVG files directly in terminal"
    sleep $SLEEP_SHORT

    show_command "showme logo.svg"
    showme logo.svg

    sleep $SLEEP_MEDIUM
}

# Demo 6: Image sizing options
demo_sizing() {
    banner "DEMO 6: Custom Sizing"
    info "Fit image to specific dimensions (40 columns x 20 rows)"
    sleep $SLEEP_SHORT

    show_command "showme --width 40 --height 20 landscape.jpg"
    showme --width 40 --height 20 landscape.jpg

    sleep $SLEEP_MEDIUM

    info "Fit to width only"
    sleep $SLEEP_SHORT

    show_command "showme --fit-width landscape.jpg"
    showme --fit-width landscape.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 7: Slideshow
demo_slideshow() {
    banner "DEMO 7: Slideshow with Titles"
    info "Automatic slideshow with 3-second delay and image info"
    sleep $SLEEP_SHORT

    show_command "showme --wait 3 --title '%f (%wx%h)' wallpaper.jpg portrait.jpg landscape.jpg"
    showme --wait 3 --title '%f (%wx%h)' wallpaper.jpg portrait.jpg landscape.jpg

    sleep $SLEEP_SHORT
}

# Demo 8: Video playback (Rick Roll!)
demo_video() {
    banner "DEMO 8: Video Playback 🎵"
    info "Never Gonna Give You Up - Rick Astley"
    info "Playing first 10 seconds..."
    sleep $SLEEP_MEDIUM

    show_command "showme --duration 10s rickroll.mp4"
    showme --duration 10s rickroll.mp4

    sleep $SLEEP_SHORT

    info "You just got Rick Rolled! 🎸"
    sleep $SLEEP_MEDIUM
}

# Demo 9: Advanced features
demo_advanced() {
    banner "DEMO 9: Advanced Features"
    info "Center image with custom background"
    sleep $SLEEP_SHORT

    show_command "showme --center --background '#1e1e2e' --width 60 logo.svg"
    showme --center --background '#1e1e2e' --width 60 logo.svg

    sleep $SLEEP_MEDIUM
}

# Demo 10: Multiple backends comparison
demo_backends() {
    banner "DEMO 10: Backend Comparison"

    info "1. Kitty Graphics (full resolution)"
    sleep $SLEEP_SHORT
    show_command "showme --backend kitty portrait.jpg"
    showme --backend kitty portrait.jpg
    sleep $SLEEP_MEDIUM

    info "2. iTerm2 Inline Images"
    sleep $SLEEP_SHORT
    show_command "showme --backend iterm2 portrait.jpg"
    showme --backend iterm2 portrait.jpg
    sleep $SLEEP_MEDIUM

    info "3. Unicode Blocks (universal)"
    sleep $SLEEP_SHORT
    show_command "showme --backend unicode portrait.jpg"
    showme --backend unicode portrait.jpg
    sleep $SLEEP_MEDIUM
}

# Main demo sequence
main() {
    clear

    banner "🎬 showme - Terminal Media Viewer Demo 🎬"
    info "Showcasing powerful terminal graphics capabilities"
    sleep $SLEEP_LONG

    setup_demo
    download_samples

    # Check if user wants to download Rick Roll
    if command -v yt-dlp &> /dev/null; then
        download_rickroll
        HAS_VIDEO=true
    else
        info "yt-dlp not found, skipping video demo"
        info "Install with: pip install yt-dlp"
        HAS_VIDEO=false
        sleep $SLEEP_MEDIUM
    fi

    # Run demos
    demo_basic
    demo_kitty
    demo_unicode
    demo_grid
    demo_svg
    demo_sizing
    demo_slideshow

    if [ "$HAS_VIDEO" = true ]; then
        demo_video
    fi

    demo_advanced
    demo_backends

    # Finale
    banner "🎉 Demo Complete! 🎉"
    info "showme - View images, videos, PDFs, and SVGs in your terminal"
    info ""
    info "Features demonstrated:"
    echo "  • Multiple rendering backends (Kitty, iTerm2, Unicode)"
    echo "  • High-resolution graphics with Kitty protocol"
    echo "  • Grid layouts for multiple images"
    echo "  • SVG vector graphics support"
    echo "  • Video playback with ffmpeg"
    echo "  • Custom sizing and positioning"
    echo "  • Slideshow mode with titles"
    echo ""
    info "Installation: cargo install showme"
    info "Repository: https://github.com/sandwichfarm/showme"
    echo ""
    sleep $SLEEP_LONG

    # Cleanup option
    echo -e "${YELLOW}Clean up demo files in $DEMO_DIR? [y/N]${RESET} "
    read -t 10 -n 1 cleanup || cleanup="n"
    echo ""
    if [[ $cleanup =~ ^[Yy]$ ]]; then
        info "Cleaning up demo directory..."
        rm -rf "$DEMO_DIR"
        info "Done!"
    else
        info "Demo files preserved in: $DEMO_DIR"
    fi
}

# Run the demo!
main "$@"
