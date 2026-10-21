# Changelog

All notable changes follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

The manifest's 0.1.1 is a development version. No 0.1.1 GitHub release or local
tag was present when the release history was checked on 2026-10-21.

### Added

- Deterministic `Config::default()` for library callers and compiled extended documentation examples.
- Actual renderer, CLI output, decoder pixel/timing, protocol payload, and Unix
  PTY lifecycle regression tests with small synthetic media fixtures.
- Scheduled and PR dependency audits, independent renderer/media coverage gates,
  a platform-specific CI matrix, and packaged-binary runtime smoke jobs.
- Checksum-pinned PDFium 7881 provisioning and runtime/license packaging.
- Existing 0.1.1 development features: DDS decoding, Unicode fit-to-height,
  configurable PNG compression, integer scaling and antialias fixes, and
  improved PDFium discovery.

### Changed

- Updated compatible dependencies and reviewed terminal/media APIs. Replaced
  `atty` with the standard library and removed unused direct dependencies.
- Upgraded the SVG/font stack, FFmpeg wrapper, PDFium bindings, and static Sixel
  binding. Retained ZIP-free FIGlet 0.1.5 because 1.0.0 introduces vulnerable ZIP.
- Declared Rust 1.92 minimum; pinned development formatting/linting to 1.98.1.
- Decode/render inputs in order, release media between inputs, and stop bounded
  previews during decoding. Added byte, dimension, frame, page, and retained-data
  limits with explicit errors.
- Build/release workflows share native setup and artifact generation. Tagged
  releases require version agreement, verification, runtime checks, and successful
  publishing. An existing crate is accepted only when its packaged files match.

### Fixed

- Explicit file output without a terminal, with deterministic sizing and isolated
  terminal controls.
- Force-format dispatch, GIF file/stdin crop consistency, frame offsets and zero
  limits, scrolling limits/direction, and duration-capped playback delays.
- PDF red/blue channel order, native video stride validation, and timestamp
  overflow handling.
- Terminal character-cell size interpretation, unbuffered query replies, raw-mode
  preservation, and Ctrl-C cleanup during playback waits or Unix stdin reads.
- Graphics chunk framing/newlines, documented iTerm2 multipart transfer, and GNU
  screen DCS escaping/size limits.
- Non-finite/overflowing duration and sizing inputs, malformed Unicode color
  strings, conflicting layouts, and disabled-backend errors.
- README library snippets, Cargo command selection, native-library documentation,
  and unsupported release-feature claims.

## [0.1.0] - 2025-10-25

Initial Rust release, published as
[v0.1.0](https://github.com/sandwichfarm/showme/releases/tag/v0.1.0).

[Unreleased]: https://github.com/sandwichfarm/showme/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sandwichfarm/showme/releases/tag/v0.1.0
