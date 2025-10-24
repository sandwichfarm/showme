# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of showme
- Multiple rendering backends:
  - Unicode half-block and quarter-block renderers with 24-bit color
  - Kitty graphics protocol backend
  - iTerm2 inline image backend (OSC 1337)
  - Optional Sixel graphics backend
  - Automatic backend detection with multiplexer support (tmux/screen)
- Wide format support:
  - Image formats: PNG, JPEG, GIF, BMP, WebP, TIFF, EXR, TGA, DDS, HDR, ICO, PNM, QOI
  - Video formats: MP4, MKV, MOV, AVI, WebM via ffmpeg
  - Document formats: PDF with multi-page support
  - Vector formats: SVG, SVGZ
- Flexible sizing options:
  - Automatic terminal size detection
  - Manual width/height constraints
  - Fit-to-width and fit-to-height modes
  - Upscaling with integer scaling support
  - EXIF orientation support for phone photos
  - Auto-crop to remove uniform borders
  - Fixed border cropping
- Display features:
  - Grid layout with configurable spacing
  - Image centering
  - Scroll animation for large images
  - Timed slideshows
  - Title format strings
  - Alternate screen buffer support
  - Cursor hiding control
  - 8-bit color mode for older terminals
- Animation support:
  - Animated GIF playback
  - Video playback with frame-accurate controls
  - Loop control (infinite or specific count)
  - Frame selection and offset
  - Time-based duration limits
- Performance:
  - Parallel image loading using Rayon
  - Configurable thread pool
  - Compression level control
- Library API:
  - Clean Rust API for integration
  - Programmatic configuration without CLI parsing
  - Direct image loading functions
  - Comprehensive examples
- Testing:
  - 34+ library tests
  - Integration tests
  - Configuration parsing tests
  - EXIF rotation tests
  - Terminal detection tests
  - Auto-crop tests

### Changed
- Complete rewrite from C++ (timg) to Rust
- Improved error handling with thiserror
- Better terminal capability detection
- Enhanced multiplexer support with automatic DCS passthrough

### Removed
- Legacy C++ codebase
- CMake build system

## [0.1.0] - Unreleased

Initial development version.

[Unreleased]: https://github.com/sandwichfarm/showme/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sandwichfarm/showme/releases/tag/v0.1.0
