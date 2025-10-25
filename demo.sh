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
SLEEP_SHORT=0.5
SLEEP_MEDIUM=1
SLEEP_LONG=1.5

# Safe mode: limit image sizes
MAX_WIDTH=80
MAX_HEIGHT=40

# Get the script directory at startup (before cd)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# ASCII art title function for large, visible demo headers
ascii_title() {
    echo ""
    echo ""

    local text="$1"
    local ascii_bin="$SCRIPT_DIR/target/release/ascii-title"

    # Use our ASCII art generator if available
    if [ -x "$ascii_bin" ]; then
        "$ascii_bin" "$text" | while IFS= read -r line; do
            echo -e "${YELLOW}${line}${RESET}"
        done
    else
        # Fallback to simple text if binary not found
        echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "${YELLOW}  $text${RESET}"
        echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
    fi

    # Add subtitle if provided
    if [ -n "$2" ]; then
        echo ""
        echo -e "${GREEN}                    $2${RESET}"
    fi

    echo ""
    echo ""
}

# Banner function (kept for compatibility)
banner() {
    ascii_title "$@"
}

# Info function
info() {
    echo -e "${GREEN}➜${RESET} $1"
}

# Command preview function
show_command() {
    echo -e "${MAGENTA}$ $1${RESET}"
    sleep 0.3
}

# Safe showme execution with error handling
safe_showme() {
    local exit_code

    # Use -o /dev/stdout to bypass TTY check while still outputting to terminal
    # This allows the wrapper to capture exit codes without breaking TTY detection
    showme -o /dev/stdout "$@"
    exit_code=$?

    if [ $exit_code -ne 0 ]; then
        echo -e "${RED}⚠ Command failed with exit code $exit_code:${RESET} showme $*"
        echo -e "${YELLOW}Continuing with demo...${RESET}"
        sleep 2
        return 1
    fi

    return 0
}

# Clear screen between demos
clear_demo() {
    sleep 0.5
    clear
}

# Setup demo directory
setup_demo() {
    mkdir -p "$DEMO_DIR"
    cd "$DEMO_DIR"
}

# Download sample images
download_samples() {
    # Check for NoGood wallpapers
    NOGOOD_DIR="$SCRIPT_DIR/demo-assets/nogood"

    if [ -d "$NOGOOD_DIR" ]; then
        # Get all available NoGood wallpapers from all packs and shuffle them
        local nogood_files=()
        while IFS= read -r -d '' f; do
            nogood_files+=("$f")
        done < <(find "$NOGOOD_DIR" -name "*.jpg" -print0)

        if [ ${#nogood_files[@]} -gt 0 ]; then
            # Shuffle the array (Fisher-Yates shuffle)
            for ((i=${#nogood_files[@]}-1; i>0; i--)); do
                j=$((RANDOM % (i+1)))
                temp="${nogood_files[i]}"
                nogood_files[i]="${nogood_files[j]}"
                nogood_files[j]="$temp"
            done

            # Copy shuffled images - ensure maximum variety across all demos
            [ ! -f "wallpaper.jpg" ] && [ -n "${nogood_files[0]}" ] && cp "${nogood_files[0]}" wallpaper.jpg 2>/dev/null || true
            [ ! -f "wallpaper1.jpg" ] && [ -n "${nogood_files[1]}" ] && cp "${nogood_files[1]}" wallpaper1.jpg 2>/dev/null || true
            [ ! -f "wallpaper2.jpg" ] && [ -n "${nogood_files[2]}" ] && cp "${nogood_files[2]}" wallpaper2.jpg 2>/dev/null || true
            [ ! -f "wallpaper3.jpg" ] && [ -n "${nogood_files[3]}" ] && cp "${nogood_files[3]}" wallpaper3.jpg 2>/dev/null || true
            [ ! -f "portrait.jpg" ] && [ -n "${nogood_files[4]}" ] && cp "${nogood_files[4]}" portrait.jpg 2>/dev/null || true
            [ ! -f "landscape.jpg" ] && [ -n "${nogood_files[5]}" ] && cp "${nogood_files[5]}" landscape.jpg 2>/dev/null || true
            # Additional images for demos to avoid repetition
            [ ! -f "kitty_demo.jpg" ] && [ -n "${nogood_files[6]}" ] && cp "${nogood_files[6]}" kitty_demo.jpg 2>/dev/null || true
            [ ! -f "slideshow1.jpg" ] && [ -n "${nogood_files[7]}" ] && cp "${nogood_files[7]}" slideshow1.jpg 2>/dev/null || true
            [ ! -f "slideshow2.jpg" ] && [ -n "${nogood_files[8]}" ] && cp "${nogood_files[8]}" slideshow2.jpg 2>/dev/null || true
            [ ! -f "slideshow3.jpg" ] && [ -n "${nogood_files[9]}" ] && cp "${nogood_files[9]}" slideshow3.jpg 2>/dev/null || true
        fi
    else
        # Fallback to random images if NoGood wallpapers not available
        [ ! -f "wallpaper.jpg" ] && curl -sL "https://picsum.photos/1920/1080" -o wallpaper.jpg
        [ ! -f "portrait.jpg" ] && curl -sL "https://picsum.photos/1080/1920" -o portrait.jpg
        [ ! -f "landscape.jpg" ] && curl -sL "https://picsum.photos/2560/1440" -o landscape.jpg
        [ ! -f "kitty_demo.jpg" ] && curl -sL "https://picsum.photos/1920/1080?random=1" -o kitty_demo.jpg
        [ ! -f "slideshow1.jpg" ] && curl -sL "https://picsum.photos/1920/1080?random=2" -o slideshow1.jpg
        [ ! -f "slideshow2.jpg" ] && curl -sL "https://picsum.photos/1080/1920?random=3" -o slideshow2.jpg
        [ ! -f "slideshow3.jpg" ] && curl -sL "https://picsum.photos/2560/1440?random=4" -o slideshow3.jpg
        [ ! -f "wallpaper1.jpg" ] && curl -sL "https://picsum.photos/1920/1080?random=5" -o wallpaper1.jpg
        [ ! -f "wallpaper2.jpg" ] && curl -sL "https://picsum.photos/1920/1080?random=6" -o wallpaper2.jpg
        [ ! -f "wallpaper3.jpg" ] && curl -sL "https://picsum.photos/1920/1080?random=7" -o wallpaper3.jpg
    fi

    # Create a simple SVG
    if [ ! -f "logo.svg" ]; then
        cat > logo.svg <<'EOF'
<svg width="400" height="400" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="400" fill="#1e1e1e"/>
  <circle cx="200" cy="200" r="150" fill="none" stroke="#00ff00" stroke-width="10"/>
  <text x="200" y="220" font-family="monospace" font-size="48" fill="#00ff00" text-anchor="middle">showme</text>
</svg>
EOF
    fi

    # Download a sample animated GIF
    if [ ! -f "animation.gif" ]; then
        # Use a small, fun animated GIF (Nyan Cat or similar)
        curl -sL "https://media.giphy.com/media/sIIhZliB2McAo/giphy.gif" -o animation.gif 2>/dev/null || true
    fi
}

# Download Rick Roll video
download_rickroll() {
    if [ ! -f "rickroll.mp4" ]; then
        yt-dlp -f 'bestvideo[height<=720]+bestaudio/best[height<=720]' \
               --cookies-from-browser firefox \
               --merge-output-format mp4 \
               -o rickroll.mp4 \
               'https://www.youtube.com/watch?v=xvFZjo5PgG0' &>/dev/null || true
    fi
}

# Demo: Auto-detection
demo_basic() {
    clear_demo
    ascii_title "AUTODETECTS BACKEND"
    show_command "showme --width $MAX_WIDTH wallpaper.jpg"
    safe_showme --width $MAX_WIDTH wallpaper.jpg
    sleep $SLEEP_LONG
}

# Demo: Kitty Graphics Protocol
demo_kitty() {
    clear_demo
    ascii_title "KITTY GRAPHICS"
    show_command "showme --backend kitty --center --width $MAX_WIDTH kitty_demo.jpg"
    safe_showme --backend kitty --center --width $MAX_WIDTH kitty_demo.jpg
    sleep $SLEEP_LONG
}

# Demo: Unicode rendering
demo_unicode() {
    clear_demo
    ascii_title "UNICODE QUARTER BLOCK" "half block supported too!"
    show_command "showme --backend unicode -p quarter --width 60 portrait.jpg"
    safe_showme --backend unicode -p quarter --width 60 portrait.jpg
    sleep $SLEEP_LONG
}

# Demo: Grid layout
demo_grid() {
    clear_demo
    ascii_title "GRID"

    # Use NoGood wallpapers if available
    if [ -f "wallpaper1.jpg" ] && [ -f "wallpaper2.jpg" ] && [ -f "wallpaper3.jpg" ] && [ -f "wallpaper.jpg" ]; then
        show_command "showme --backend unicode --grid 4x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg"
        safe_showme --backend unicode --grid 4x1 --width $MAX_WIDTH wallpaper1.jpg wallpaper2.jpg wallpaper3.jpg wallpaper.jpg
    else
        show_command "showme --backend unicode --grid 3x1 --width $MAX_WIDTH wallpaper.jpg wallpaper.jpg wallpaper.jpg"
        safe_showme --backend unicode --grid 3x1 --width $MAX_WIDTH wallpaper.jpg wallpaper.jpg wallpaper.jpg
    fi

    sleep $SLEEP_LONG
}

# Demo: SVG rendering
demo_svg() {
    clear_demo
    ascii_title "SVG"
    show_command "showme --center --width 50 logo.svg"
    safe_showme --center --width 50 logo.svg
    sleep $SLEEP_LONG
}

# Removed - positioning is now implicit in other demos

# Demo: Slideshow
demo_slideshow() {
    clear_demo
    ascii_title "SLIDESHOW"

    # Use dedicated slideshow images for variety
    FILES=""
    [ -f "slideshow1.jpg" ] && FILES="$FILES slideshow1.jpg"
    [ -f "slideshow2.jpg" ] && FILES="$FILES slideshow2.jpg"
    [ -f "slideshow3.jpg" ] && FILES="$FILES slideshow3.jpg"

    if [ -n "$FILES" ]; then
        show_command "showme --center --wait 1.5s --title '%f' --width 60$FILES"
        safe_showme --center --wait 1.5s --title '%f' --width 60 $FILES
    fi

    sleep $SLEEP_LONG
}

# Demo: Animated GIF
demo_gif() {
    clear_demo
    ascii_title "ANIMATED GIF"

    if [ ! -f "animation.gif" ]; then
        return
    fi

    show_command "showme --center --loops 2 --alternate-screen animation.gif"
    safe_showme --center --loops 2 animation.gif
    sleep $SLEEP_LONG
}

# Demo: Video playback
demo_video() {
    clear_demo
    ascii_title "VIDEO"

    if [ ! -f "rickroll.mp4" ]; then
        return
    fi

    show_command "showme --center --duration 5s rickroll.mp4"
    safe_showme --center --duration 5s rickroll.mp4
    sleep $SLEEP_LONG
}

# Removed - redundant demos

# Main demo sequence
main() {
    clear

    ascii_title "SHOWME DEMO"
    sleep 1

    setup_demo
    download_samples

    # Check if user wants to download Rick Roll (quietly)
    if command -v yt-dlp &> /dev/null; then
        download_rickroll
        HAS_VIDEO=true
    else
        HAS_VIDEO=false
    fi

    # Run demos
    demo_basic
    demo_kitty
    demo_unicode
    demo_grid
    demo_svg
    demo_slideshow
    demo_gif

    if [ "$HAS_VIDEO" = true ]; then
        demo_video
    fi

    # Finale
    clear
    ascii_title "GO OUTSIDE"
    sleep 1

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
