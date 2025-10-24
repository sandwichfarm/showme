# Contributing to showme

Thank you for your interest in contributing to showme! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Release Process](#release-process)

## Code of Conduct

This project follows standard open source community guidelines. Please be respectful and constructive in all interactions.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR-USERNAME/showme.git
   cd showme
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/sandwichfarm/showme.git
   ```

## Development Setup

### Requirements

- Rust 2024 edition (latest stable recommended)
- For video support: ffmpeg development libraries
  ```bash
  # Ubuntu/Debian
  sudo apt install libavcodec-dev libavformat-dev libavutil-dev \
                   libavfilter-dev libavdevice-dev libswscale-dev \
                   libswresample-dev clang pkg-config

  # macOS
  brew install ffmpeg pkg-config

  # Arch Linux
  sudo pacman -S ffmpeg clang pkg-config
  ```

- For Sixel support (optional):
  ```bash
  # Ubuntu/Debian
  sudo apt install libsixel-dev

  # macOS
  brew install libsixel
  ```

### Building

```bash
# Build with default features
cargo build

# Build with all features including Sixel
cargo build --features sixel

# Build without default features
cargo build --no-default-features --features unicode,kitty,iterm2

# Run tests
cargo test

# Run with example
cargo run -- path/to/image.jpg
```

## Making Changes

### Branch Naming

- Feature branches: `feature/description`
- Bug fixes: `fix/description`
- Documentation: `docs/description`

### Commit Messages

Follow conventional commit format:

```
type(scope): brief description

Longer explanation if needed.

Fixes #123
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

Examples:
- `feat(backend): add Sixel graphics support`
- `fix(exif): correct rotation for landscape images`
- `docs(readme): add installation instructions`
- `test(config): add parser validation tests`

## Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test config_tests
cargo test rendering_tests
cargo test capabilities_tests

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_tests
```

### Adding Tests

When adding new features:
1. **Unit tests** - Add to the relevant module in `src/`
2. **Integration tests** - Add to `tests/` directory
3. **Examples** - Consider adding example code in `examples/`

Test coverage areas:
- Configuration parsing and validation
- Image loading and processing
- EXIF handling and rotation
- Rendering backends
- Terminal capability detection
- Error handling

### Test Guidelines

- Write descriptive test names: `test_parse_hex_color_with_hash_prefix`
- Test both success and failure cases
- Include edge cases (empty input, large values, invalid data)
- Use `assert_eq!` with clear failure messages
- Keep tests focused and isolated

## Submitting Changes

### Pull Request Process

1. **Update your fork**:
   ```bash
   git fetch upstream
   git rebase upstream/master
   ```

2. **Make your changes** in a feature branch

3. **Run tests and linting**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   ```

4. **Update documentation**:
   - Update README.md if adding features
   - Update CHANGELOG.md under `[Unreleased]`
   - Add rustdoc comments for public APIs
   - Update examples if API changes

5. **Push to your fork**:
   ```bash
   git push origin feature/your-feature
   ```

6. **Create Pull Request** on GitHub with:
   - Clear description of changes
   - Reference related issues
   - Screenshots/examples for visual changes
   - Note any breaking changes

### PR Review Process

- Maintainers will review your PR
- Address feedback by pushing new commits
- Once approved, PR will be merged
- Feel free to ask questions or request clarification

## Coding Standards

### Rust Style

Follow standard Rust conventions:

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for common mistakes
cargo clippy -- -D warnings
```

### Code Organization

- Keep functions focused and small
- Use descriptive variable names
- Add comments for complex logic
- Document public APIs with rustdoc
- Use `Result` and `?` for error handling
- Prefer iterator chains over loops where clear

### Error Handling

```rust
// Use custom error types
use crate::error::{RimgError, Result};

// Return Result from fallible functions
pub fn load_image(path: &Path) -> Result<ImageSequence> {
    // ...
}

// Use context for better error messages
.map_err(|e| RimgError::other(format!("Failed to load {}: {}", path.display(), e)))?
```

### Documentation

```rust
/// Brief one-line description.
///
/// More detailed explanation if needed. Can span multiple
/// paragraphs and include examples.
///
/// # Arguments
///
/// * `path` - Path to the image file
/// * `mode` - Rotation mode to apply
///
/// # Returns
///
/// Returns an `ImageSequence` containing all frames.
///
/// # Errors
///
/// Returns `RimgError` if the file cannot be read or decoded.
///
/// # Examples
///
/// ```
/// use terminal_media::image::load_image;
/// use terminal_media::config::RotationMode;
///
/// let seq = load_image(Path::new("photo.jpg"), RotationMode::Exif, false, 0)?;
/// ```
pub fn load_image(path: &Path, mode: RotationMode, auto_crop: bool, crop_border: u32) -> Result<ImageSequence> {
    // ...
}
```

## Release Process

Releases are automated via GitHub Actions:

1. Update `CHANGELOG.md` with release notes
2. Update version in `Cargo.toml`
3. Create and push a git tag:
   ```bash
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push upstream v0.2.0
   ```
4. GitHub Actions will:
   - Build release binary
   - Publish to crates.io
   - Create GitHub release
   - Generate changelog from commits

## Areas for Contribution

Looking for something to work on? Check out:

- **GitHub Issues** - Tagged with `good-first-issue` or `help-wanted`
- **Documentation** - Improve examples, tutorials, or API docs
- **Testing** - Increase test coverage
- **Performance** - Optimize rendering or image loading
- **Features**:
  - Additional image format support
  - New rendering backends
  - Terminal capability detection improvements
  - Cross-platform compatibility
  - Accessibility features

## Questions?

- Open an issue for bug reports or feature requests
- Start a discussion for questions or ideas
- Check existing issues before opening new ones

Thank you for contributing to showme!
