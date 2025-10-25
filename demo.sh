#!/bin/bash
#
# showme - Terminal Media Viewer Demo
# A comprehensive showcase of showme's capabilities
#

# Don't crash on errors - handle them gracefully
set +e

# Reset terminal on exit/interrupt
cleanup() {
    echo ""
    echo "Resetting terminal..."
    tput reset 2>/dev/null || reset 2>/dev/null || true
    echo "Demo stopped!"
}
trap cleanup EXIT INT TERM

# Colors for text output
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
MAGENTA='\033[0;35m'
RED='\033[0;31m'
RESET='\033[0m'

# Demo configuration
DEMO_DIR="/tmp/showme-demo"
SLEEP_SHORT=2
SLEEP_MEDIUM=3
SLEEP_LONG=4

# Safe mode: limit image sizes
MAX_WIDTH=80
MAX_HEIGHT=40

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

# Safe showme execution with error handling
safe_showme() {
    local output
    local exit_code

    # Capture both stdout and stderr
    output=$(showme "$@" 2>&1)
    exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo "$output"
        return 0
    else
        echo -e "${RED}⚠ Command failed:${RESET} showme $*"
        echo -e "${YELLOW}Error output:${RESET}"
        echo "$output" | head -5
        echo -e "${YELLOW}Continuing with demo...${RESET}"
        sleep 2
        return 1
    fi
}

# Clear screen between demos
clear_demo() {
    sleep 0.5
    clear
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
        info "Found NoGood Wallpaper Pack! Shuffling for variety 🎲"

        # Get all available NoGood wallpapers and shuffle them
        local nogood_files=()
        for f in "$NOGOOD_DIR"/NoGood_WallpaperPack_*.jpg; do
            [ -f "$f" ] && nogood_files+=("$f")
        done

        if [ ${#nogood_files[@]} -gt 0 ]; then
            # Shuffle the array (Fisher-Yates shuffle)
            for ((i=${#nogood_files[@]}-1; i>0; i--)); do
                j=$((RANDOM % (i+1)))
                temp="${nogood_files[i]}"
                nogood_files[i]="${nogood_files[j]}"
                nogood_files[j]="$temp"
            done

            # Copy first 4 shuffled images
            [ ! -f "wallpaper.jpg" ] && [ -n "${nogood_files[0]}" ] && cp "${nogood_files[0]}" wallpaper.jpg 2>/dev/null || true
            [ ! -f "wallpaper1.jpg" ] && [ -n "${nogood_files[1]}" ] && cp "${nogood_files[1]}" wallpaper1.jpg 2>/dev/null || true
            [ ! -f "wallpaper2.jpg" ] && [ -n "${nogood_files[2]}" ] && cp "${nogood_files[2]}" wallpaper2.jpg 2>/dev/null || true
            [ ! -f "wallpaper3.jpg" ] && [ -n "${nogood_files[3]}" ] && cp "${nogood_files[3]}" wallpaper3.jpg 2>/dev/null || true

            # Use shuffled images for portrait/landscape too
            [ ! -f "portrait.jpg" ] && [ -f "wallpaper.jpg" ] && cp "wallpaper.jpg" portrait.jpg
            [ ! -f "landscape.jpg" ] && [ -f "wallpaper1.jpg" ] && cp "wallpaper1.jpg" landscape.jpg

            info "Shuffled ${#nogood_files[@]} NoGood wallpapers - these are FIRE! 🔥"
        fi
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
    clear_demo
    banner "DEMO 1: Basic Image Viewing"
    info "Display a wallpaper with auto-detection (limited to terminal size)"
    sleep $SLEEP_SHORT

    show_command "showme --width $MAX_WIDTH wallpaper.jpg"
    safe_showme --width $MAX_WIDTH wallpaper.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 2: Kitty Graphics Protocol
demo_kitty() {
    clear_demo
    banner "DEMO 2: Kitty Graphics Protocol (High-Res)"
    info "Kitty backend for crisp rendering (size-limited for safety)"
    sleep $SLEEP_SHORT

    show_command "showme --backend kitty --width $MAX_WIDTH --verbose wallpaper.jpg"
    safe_showme --backend kitty --width $MAX_WIDTH --verbose wallpaper.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 3: Unicode rendering modes
demo_unicode() {
    clear_demo
    banner "DEMO 3: Unicode Block Rendering"
    info "Quarter-block mode (default) - best detail"
    sleep $SLEEP_SHORT

    show_command "showme --backend unicode -p quarter --width 60 portrait.jpg"
    safe_showme --backend unicode -p quarter --width 60 portrait.jpg

    sleep $SLEEP_MEDIUM

    clear_demo
    banner "DEMO 3: Unicode Block Rendering (continued)"
    info "Half-block mode - better color accuracy"
    sleep $SLEEP_SHORT

    show_command "showme --backend unicode -p half --width 60 portrait.jpg"
    safe_showme --backend unicode -p half --width 60 portrait.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 4: Image grid layout
demo_grid() {
    clear_demo
    banner "DEMO 4: Grid Layout - Portrait Images"
    info "Display multiple tall images in 4x1 horizontal strip"
    info "Perfect for those fire NoGood wallpapers! 🔥"
    sleep $SLEEP_SHORT

    # Check if we have enough files for a grid
    if [ ! -f "wallpaper.jpg" ]; then
        echo -e "${RED}⚠ No images available for grid demo, skipping...${RESET}"
        return
    fi

    # Use NoGood wallpapers if available - they're tall so 4x1 grid works best!
    if [ -f "wallpaper1.jpg" ] && [ -f "wallpaper2.jpg" ] && [ -f "wallpaper3.jpg" ] && [ -f "wallpaper.jpg" ]; then
        info "4 NoGood wallpapers in horizontal strip - CRISPY! 🔥"
        show_command "showme --grid 4x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg"
        safe_showme --grid 4x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg
    elif [ -f "wallpaper1.jpg" ] && [ -f "wallpaper2.jpg" ] && [ -f "wallpaper3.jpg" ]; then
        info "3 NoGood wallpapers in horizontal strip"
        show_command "showme --grid 3x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg"
        safe_showme --grid 3x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg
    elif [ -f "wallpaper.jpg" ] && [ -f "portrait.jpg" ] && [ -f "landscape.jpg" ]; then
        show_command "showme --grid 3x1 --width $MAX_WIDTH wallpaper.jpg portrait.jpg landscape.jpg"
        safe_showme --grid 3x1 --width $MAX_WIDTH wallpaper.jpg portrait.jpg landscape.jpg
    else
        # Fallback to just the wallpaper 3 times
        show_command "showme --grid 3x1 --width $MAX_WIDTH wallpaper.jpg wallpaper.jpg wallpaper.jpg"
        safe_showme --grid 3x1 --width $MAX_WIDTH wallpaper.jpg wallpaper.jpg wallpaper.jpg
    fi

    sleep $SLEEP_LONG
}

# Demo 5: SVG rendering
demo_svg() {
    clear_demo
    banner "DEMO 5: SVG Vector Graphics"
    info "Render SVG files directly in terminal"
    sleep $SLEEP_SHORT

    show_command "showme --width 50 logo.svg"
    safe_showme --width 50 logo.svg

    sleep $SLEEP_MEDIUM
}

# Demo 6: Image sizing options
demo_sizing() {
    clear_demo
    banner "DEMO 6: Custom Sizing"
    info "Fit image to specific dimensions (40 columns x 20 rows)"
    sleep $SLEEP_SHORT

    show_command "showme --width 40 --height 20 landscape.jpg"
    safe_showme --width 40 --height 20 landscape.jpg

    sleep $SLEEP_MEDIUM

    clear_demo
    banner "DEMO 6: Custom Sizing (continued)"
    info "Fit to width only"
    sleep $SLEEP_SHORT

    show_command "showme --fit-width --width 70 landscape.jpg"
    safe_showme --fit-width --width 70 landscape.jpg

    sleep $SLEEP_MEDIUM
}

# Demo 7: Slideshow
demo_slideshow() {
    clear_demo
    banner "DEMO 7: Slideshow with Titles"
    info "Automatic slideshow with 2-second delay and image info"
    sleep $SLEEP_SHORT

    # Check which files we have
    FILES=""
    [ -f "wallpaper.jpg" ] && FILES="$FILES wallpaper.jpg"
    [ -f "portrait.jpg" ] && FILES="$FILES portrait.jpg"
    [ -f "landscape.jpg" ] && FILES="$FILES landscape.jpg"

    if [ -z "$FILES" ]; then
        echo -e "${RED}⚠ No images available for slideshow, skipping...${RESET}"
        return
    fi

    show_command "showme --wait 2 --title '%f (%wx%h)' --width 60$FILES"
    safe_showme --wait 2 --title '%f (%wx%h)' --width 60 $FILES

    sleep $SLEEP_SHORT
}

# Demo 8: Video playback (Rick Roll!)
demo_video() {
    clear_demo
    banner "DEMO 8: Video Playback 🎵"

    # Check if video file exists
    if [ ! -f "rickroll.mp4" ]; then
        echo -e "${YELLOW}⚠ Rick Roll video not found, skipping video demo...${RESET}"
        info "To enable video demo: Install yt-dlp and re-run"
        sleep $SLEEP_SHORT
        return
    fi

    info "Never Gonna Give You Up - Rick Astley"
    info "Playing first 5 seconds..."
    sleep $SLEEP_MEDIUM

    show_command "showme --duration 5s --width 60 rickroll.mp4"
    if safe_showme --duration 5s --width 60 rickroll.mp4; then
        info "You just got Rick Rolled! 🎸"
    else
        echo -e "${YELLOW}⚠ Video playback failed (video feature may not be available)${RESET}"
    fi

    sleep $SLEEP_MEDIUM
}

# Demo 9: Advanced features
demo_advanced() {
    clear_demo
    banner "DEMO 9: Advanced Features"
    info "Center image with custom background"
    sleep $SLEEP_SHORT

    show_command "showme --center --background '#1e1e2e' --width 50 logo.svg"
    safe_showme --center --background '#1e1e2e' --width 50 logo.svg

    sleep $SLEEP_MEDIUM
}

# Demo 10: Kitty vs Unicode comparison
demo_backends() {
    clear_demo
    banner "DEMO 10: Kitty vs Unicode Rendering"

    info "1. Kitty Graphics Protocol - Full Resolution 🔥"
    info "   Sharp, crisp rendering using terminal graphics"
    sleep $SLEEP_SHORT
    show_command "showme --backend kitty --width 60 portrait.jpg"
    safe_showme --backend kitty --width 60 portrait.jpg
    sleep $SLEEP_LONG

    clear_demo
    banner "DEMO 10: Kitty vs Unicode (continued)"
    info "2. Unicode Blocks - Universal Fallback"
    info "   Works everywhere, but lower resolution"
    sleep $SLEEP_SHORT
    show_command "showme --backend unicode -p quarter --width 60 portrait.jpg"
    safe_showme --backend unicode -p quarter --width 60 portrait.jpg
    sleep $SLEEP_LONG

    info ""
    info "Notice the difference? Kitty protocol is WAY crisper! 🔥"
    sleep $SLEEP_MEDIUM
}

# Main demo sequence
main() {
    clear

    banner "🎬 showme - Terminal Media Viewer Demo 🎬"
    info "Showcasing powerful terminal graphics capabilities"
    info ""
    info "✨ Safe mode enabled: Images limited to $MAX_WIDTH columns max"
    info "🛡️  Terminal reset on exit/interrupt (Ctrl+C safe)"
    info "🔥 Using NoGood wallpapers if available!"
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
