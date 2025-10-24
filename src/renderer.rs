use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use rayon::prelude::*;

use crate::backend::{Backend, BackendFactory, BackgroundStyle, RenderOptions};
use crate::capabilities::{current_terminal_size, detect_terminal_backend, ensure_tty_stdout};
use crate::config::{BackendKind, BackgroundColor, Config};
use crate::error::{Result, RimgError};
use crate::image::{Frame, ImageSequence, load_image};
use crate::tmux;

pub struct Renderer {
    config: Config,
    backend: Box<dyn Backend + Send + Sync>,
    terminal: crate::capabilities::TerminalSize,
    background: BackgroundStyle,
}

impl Renderer {
    pub fn build(config: Config) -> Result<Self> {
        ensure_tty_stdout()?;

        let terminal = current_terminal_size();
        let guess = detect_terminal_backend();

        let backend_kind = match config.backend {
            BackendKind::Auto => guess.backend,
            other => other,
        };

        let backend = match BackendFactory::build(backend_kind) {
            Ok(backend) => backend,
            Err(err) => {
                if !matches!(backend_kind, BackendKind::Unicode) && !config.quiet {
                    eprintln!("warning: {} (falling back to unicode renderer)", err);
                }
                BackendFactory::auto_from_guess(BackendKind::Unicode)
            }
        };

        // Enable tmux passthrough if using graphics protocols in tmux
        if matches!(backend_kind, BackendKind::Kitty | BackendKind::Iterm2) && tmux::in_tmux() {
            if tmux::enable_tmux_passthrough() && !config.quiet {
                eprintln!("Enabled tmux passthrough for graphics protocol");
            }
        }

        let background = background_style(&config);

        // Print verbose terminal info
        if config.verbose {
            eprintln!("Terminal information:");
            eprintln!("  Size: {}x{} cells", terminal.columns, terminal.rows);
            if let Some(w) = terminal.width_pixels {
                eprintln!("  Pixel size: {}x{}", w, terminal.height_pixels.unwrap_or(0));
                if let Some(ratio) = terminal.cell_aspect_ratio() {
                    eprintln!("  Cell aspect ratio: {:.3}", ratio);
                }
            }
            eprintln!("  Width stretch factor: {:.3}", config.sizing.width_stretch);
            if let Some(term_name) = crate::capabilities::detect_terminal_name() {
                eprintln!("  Detected terminal: {}", term_name);
            } else {
                eprintln!("  Detected terminal: unknown");
            }
            eprintln!("  Backend: {:?}", backend_kind);
            eprintln!("  Pixelation: {:?}", config.pixelation);
            if config.use_8bit_color {
                eprintln!("  Color mode: 8-bit (256 colors)");
            } else {
                eprintln!("  Color mode: 24-bit (true color)");
            }
            if crate::capabilities::is_in_multiplexer() {
                eprintln!("  Multiplexer detected: yes");
            }
            if let Some(threads) = config.threads {
                eprintln!("  Thread pool: {} threads", threads);
            } else {
                eprintln!("  Thread pool: default (system)");
            }
            eprintln!();
        }

        Ok(Self {
            config,
            backend,
            terminal,
            background,
        })
    }

    pub fn run(&self) -> Result<()> {
        let _alternate_guard = if self.config.alternate_screen {
            Some(AlternateScreenGuard::enter()?)
        } else {
            None
        };

        let sequences = self.load_sequences()?;
        if sequences.is_empty() {
            return Err(RimgError::MissingInput);
        }

        // Open output file if specified, otherwise use stdout
        let mut file_output;
        let mut stdout_output;
        let output: &mut dyn Write = if let Some(ref path) = self.config.output_file {
            file_output = std::fs::File::create(path).map_err(|err| {
                RimgError::other(format!("failed to create output file '{}': {}", path.display(), err))
            })?;
            &mut file_output
        } else {
            stdout_output = io::stdout();
            &mut stdout_output
        };

        // Hide cursor unless explicitly told not to
        let _cursor_guard = if !self.config.hide_cursor {
            None
        } else {
            Some(CursorHideGuard::hide()?)
        };

        if self.config.clear_once {
            output.write_all(b"\x1b[2J\x1b[H")?;
        }

        if let Some(grid) = &self.config.grid {
            self.render_grid(grid, &sequences, output)?;
        } else {
            for (idx, sequence) in sequences.iter().enumerate() {
                if self.config.clear_between && (idx > 0 || !self.config.clear_once) {
                    output.write_all(b"\x1b[2J\x1b[H")?;
                }

                if !self.config.quiet {
                    if let Some(title) = self.make_title(idx, sequence) {
                        writeln!(output, "{title}")?;
                    } else {
                        writeln!(output, "# {} - {}", idx + 1, sequence.path.display())?;
                    }
                }

                if self.config.verbose {
                    if let Some(frame) = sequence.first_frame() {
                        eprintln!("  Image {}: {}x{} pixels, {} frames",
                            idx + 1,
                            frame.pixels.width(),
                            frame.pixels.height(),
                            sequence.frames.len()
                        );
                    }
                }

                self.render_sequence(sequence, output)?;

                if let Some(wait) = self.config.wait_between_images {
                    if idx + 1 < sequences.len() {
                        thread::sleep(wait);
                    }
                }
            }
        }

        output.flush()?;
        Ok(())
    }

    fn load_sequences(&self) -> Result<Vec<ImageSequence>> {
        let start = std::time::Instant::now();
        let rotation_mode = self.config.rotation;
        let auto_crop = self.config.auto_crop;
        let crop_border = self.config.crop_border;

        // Configure thread pool if specified
        let sequences: Result<Vec<ImageSequence>> = if let Some(num_threads) = self.config.threads {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build()
                .map_err(|e| RimgError::other(format!("failed to create thread pool: {}", e)))?
                .install(|| {
                    self.config
                        .inputs
                        .par_iter()
                        .map(|path| load_image(path, rotation_mode, auto_crop, crop_border))
                        .collect()
                })
        } else {
            self.config
                .inputs
                .par_iter()
                .map(|path| load_image(path, rotation_mode, auto_crop, crop_border))
                .collect()
        };

        let sequences = sequences?;

        if self.config.verbose {
            let elapsed = start.elapsed();
            let total_frames: usize = sequences.iter().map(|s| s.frames.len()).sum();
            eprintln!("Loading statistics:");
            eprintln!("  Images loaded: {}", sequences.len());
            eprintln!("  Total frames: {}", total_frames);
            eprintln!("  Load time: {:.2}s", elapsed.as_secs_f64());
            if sequences.len() > 0 {
                eprintln!("  Average: {:.0}ms per image", elapsed.as_millis() as f64 / sequences.len() as f64);
            }
            eprintln!();
        }

        Ok(sequences)
    }

    fn render_sequence(&self, sequence: &ImageSequence, stdout: &mut dyn Write) -> Result<()> {
        let options = RenderOptions {
            sizing: self.config.sizing,
            terminal: self.terminal,
            background: self.background,
            pixelation: self.config.pixelation,
            use_8bit_color: self.config.use_8bit_color,
            compress_level: self.config.compress_level,
            verbose: self.config.verbose,
        };

        // Handle scrolling animation mode
        if self.config.scroll_animation {
            return self.render_scrolling(sequence, options, stdout);
        }

        // Apply frame offset and limit
        let start_frame = self.config.frame_offset.min(sequence.frames.len().saturating_sub(1));
        let available_frames = &sequence.frames[start_frame..];

        let frames_to_render = if let Some(max) = self.config.max_frames {
            &available_frames[..max.min(available_frames.len())]
        } else {
            available_frames
        };

        if frames_to_render.is_empty() {
            return Ok(());
        }

        // Single frame or static image
        if frames_to_render.len() == 1 {
            self.print_frame(&frames_to_render[0], options, stdout)?;
            return Ok(());
        }

        // Determine loop behavior
        let loop_count = if self.config.loop_forever {
            -1i32 // Infinite
        } else if let Some(loops) = self.config.loops {
            loops
        } else {
            1 // Default: play once
        };

        let mut first = true;
        let mut last_height = 0u32;
        let mut current_loop = 0i32;
        let start_time = std::time::Instant::now();
        let mut _total_frames_rendered = 0usize; // Reserved for future verbose output

        loop {
            for frame in frames_to_render {
                // Check duration limit
                if let Some(max_duration) = self.config.max_duration {
                    if start_time.elapsed() >= max_duration {
                        return Ok(());
                    }
                }

                let rendered = self.backend.render(frame, options)?;

                if !first {
                    if last_height > 0 {
                        write!(stdout, "\x1b[{}A", last_height)?;
                    }
                }

                let indent = self.indent_for(&rendered);
                self.write_rendered(&rendered, indent, stdout)?;
                stdout.flush()?;

                last_height = rendered.height_cells;
                first = false;
                _total_frames_rendered += 1;

                if frame.delay > Duration::ZERO {
                    thread::sleep(frame.delay);
                }
            }

            current_loop += 1;

            // Check if we should continue looping
            if loop_count >= 0 && current_loop >= loop_count {
                break;
            }
        }

        Ok(())
    }

    fn render_scrolling(
        &self,
        sequence: &ImageSequence,
        options: RenderOptions,
        stdout: &mut dyn Write,
    ) -> Result<()> {
        // Only works with Unicode backend
        if self.backend.supported_kind() != BackendKind::Unicode {
            return Err(RimgError::other(
                "scrolling animation is currently only supported with the unicode backend",
            ));
        }

        // Use the first frame for scrolling
        let frame = sequence.first_frame().ok_or_else(|| {
            RimgError::other("cannot scroll: image has no frames")
        })?;

        // Render the full image
        let rendered = self.backend.render(frame, options)?;

        if rendered.lines.is_empty() {
            return Ok(());
        }

        let image_width = rendered.width_cells as i32;
        let image_height = rendered.lines.len() as i32;
        let viewport_width = self.terminal.columns as i32;
        let viewport_height = self.terminal.rows as i32;

        // If image fits in viewport, no scrolling needed
        if image_width <= viewport_width && image_height <= viewport_height {
            self.write_rendered(&rendered, self.indent_for(&rendered), stdout)?;
            stdout.flush()?;
            return Ok(());
        }

        // Calculate scroll range
        let scroll_x_range = (image_width - viewport_width).max(0);
        let scroll_y_range = (image_height - viewport_height).max(0);

        let dx = self.config.scroll_dx;
        let dy = self.config.scroll_dy;

        // Generate scroll positions
        let mut x = 0i32;
        let mut y = 0i32;
        let mut positions = Vec::new();

        // Scroll until we reach the end
        loop {
            positions.push((x, y));

            let next_x = (x + dx).clamp(0, scroll_x_range);
            let next_y = (y + dy).clamp(0, scroll_y_range);

            // Stop if we can't move further
            if next_x == x && next_y == y {
                break;
            }

            x = next_x;
            y = next_y;
        }

        if positions.is_empty() {
            positions.push((0, 0));
        }

        // Determine loop behavior
        let loop_count = if self.config.loop_forever {
            -1i32 // Infinite
        } else if let Some(loops) = self.config.loops {
            loops
        } else {
            1 // Default: play once
        };

        // Animate the scroll
        let mut first = true;
        let mut current_loop = 0i32;

        loop {
            for (scroll_x, scroll_y) in &positions {
                if !first {
                    // Move cursor back to start
                    if viewport_height > 0 {
                        write!(stdout, "\x1b[{}A", viewport_height)?;
                    }
                }

                // Extract viewport from rendered image
                let viewport_lines: Vec<&str> = rendered
                    .lines
                    .iter()
                    .skip(*scroll_y as usize)
                    .take(viewport_height as usize)
                    .map(|line| {
                        let start = (*scroll_x as usize).min(line.len());
                        let end = (start + viewport_width as usize).min(line.len());
                        &line[start..end]
                    })
                    .collect();

                // Write viewport
                for line in viewport_lines {
                    stdout.write_all(line.as_bytes())?;
                    stdout.write_all(b"\n")?;
                }

                stdout.flush()?;
                first = false;

                // Delay between scroll steps
                if self.config.scroll_delay > Duration::ZERO {
                    thread::sleep(self.config.scroll_delay);
                }
            }

            current_loop += 1;

            // Check if we should continue looping
            if loop_count >= 0 && current_loop >= loop_count {
                break;
            }
        }

        Ok(())
    }

    fn print_frame(
        &self,
        frame: &Frame,
        options: RenderOptions,
        stdout: &mut dyn Write,
    ) -> Result<()> {
        let rendered = self.backend.render(frame, options)?;
        let indent = self.indent_for(&rendered);
        let res = self.write_rendered(&rendered, indent, stdout)?;
        stdout.flush()?;
        Ok(res)
    }

    fn write_rendered(
        &self,
        rendered: &crate::backend::RenderedFrame,
        indent: usize,
        stdout: &mut dyn Write,
    ) -> Result<()> {
        let padding = if indent > 0 {
            Some(" ".repeat(indent))
        } else {
            None
        };

        for line in &rendered.lines {
            if let Some(pad) = &padding {
                stdout.write_all(pad.as_bytes())?;
            }
            stdout.write_all(line.as_bytes())?;
            stdout.write_all(b"\n")?;
        }
        Ok(())
    }

    fn render_grid(
        &self,
        grid: &crate::config::GridOptions,
        sequences: &[ImageSequence],
        stdout: &mut dyn Write,
    ) -> Result<()> {
        if self.backend.supported_kind() != BackendKind::Unicode {
            return Err(RimgError::other(
                "grid layout is currently only supported with the unicode backend",
            ));
        }

        let columns = grid.columns.get();
        if columns == 0 {
            return Err(RimgError::other("grid requires at least one column"));
        }

        let max_rows = grid.rows.map(|r| r.get()).unwrap_or(usize::MAX);
        let gap = " ".repeat(grid.spacing as usize);

        for (row_index, chunk) in sequences.chunks(columns).into_iter().enumerate() {
            if row_index >= max_rows {
                break;
            }

            let effective_columns = chunk.len();

            // Use actual terminal width for grid allocation
            let available_cells = self.terminal.columns as usize;
            let reserved_for_gaps =
                (grid.spacing as usize).saturating_mul(effective_columns.saturating_sub(1));
            let per_column =
                available_cells.saturating_sub(reserved_for_gaps).max(1) / effective_columns.max(1);

            let rendered: Vec<_> = chunk
                .iter()
                .map(|sequence| {
                    sequence
                        .frames
                        .first()
                        .ok_or_else(|| RimgError::other("image without frames"))
                        .and_then(|frame| {
                            let mut sizing = self.config.sizing;
                            let width_override = sizing
                                .width_cells
                                .map(|limit| limit.min(per_column as u32))
                                .unwrap_or(per_column as u32);
                            sizing.width_cells = Some(width_override);
                            let options = RenderOptions {
                                sizing,
                                terminal: self.terminal,
                                background: self.background,
                                pixelation: self.config.pixelation,
                                use_8bit_color: self.config.use_8bit_color,
                                compress_level: self.config.compress_level,
                                verbose: self.config.verbose,
                            };
                            self.backend.render(frame, options)
                        })
                })
                .collect::<Result<Vec<_>>>()?;

            let max_lines = rendered
                .iter()
                .map(|frame| frame.lines.len())
                .max()
                .unwrap_or(0);

            for line_idx in 0..max_lines {
                for (col_idx, frame) in rendered.iter().enumerate() {
                    if line_idx < frame.lines.len() {
                        stdout.write_all(frame.lines[line_idx].as_bytes())?;
                    } else {
                        write!(
                            stdout,
                            "\x1b[0m{:width$}",
                            "",
                            width = frame.width_cells as usize
                        )?;
                    }

                    if col_idx + 1 < rendered.len() {
                        stdout.write_all(gap.as_bytes())?;
                    }
                }
                stdout.write_all(b"\n")?;
            }

            stdout.write_all(b"\n")?;

            if let Some(wait) = self.config.wait_between_rows {
                let more_rows_available =
                    (row_index + 1) < max_rows && (row_index + 1) * columns < sequences.len();
                if more_rows_available {
                    thread::sleep(wait);
                }
            }
        }

        Ok(())
    }

    fn indent_for(&self, rendered: &crate::backend::RenderedFrame) -> usize {
        if !self.config.center || self.config.grid.is_some() {
            return 0;
        }

        let available = self.terminal.columns as isize - rendered.width_cells as isize;
        if available > 0 {
            (available / 2) as usize
        } else {
            0
        }
    }

    fn make_title(&self, index: usize, sequence: &ImageSequence) -> Option<String> {
        let format = self.config.title_format.as_deref()?;
        let full = sequence.path.display().to_string();
        let basename = sequence
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let (width, height) = sequence
            .first_frame()
            .map(|frame| (frame.pixels.width(), frame.pixels.height()))
            .unwrap_or((0, 0));

        let mut out = String::with_capacity(format.len() + full.len());
        let mut chars = format.chars();
        while let Some(ch) = chars.next() {
            if ch == '%' {
                match chars.next() {
                    Some('f') => out.push_str(&full),
                    Some('b') => out.push_str(basename),
                    Some('w') => out.push_str(&width.to_string()),
                    Some('h') => out.push_str(&height.to_string()),
                    Some('n') => out.push_str(&(index + 1).to_string()),
                    Some('%') => out.push('%'),
                    Some(other) => {
                        out.push('%');
                        out.push(other);
                    }
                    None => out.push('%'),
                }
            } else {
                out.push(ch);
            }
        }

        Some(out)
    }
}

fn background_style(config: &Config) -> BackgroundStyle {
    let color = match config.background {
        BackgroundColor::Color(rgb) => Some(rgb),
        _ => None,
    };
    let pattern = config.pattern_color;
    BackgroundStyle {
        color,
        pattern,
        pattern_size: config.pattern_size.max(1),
    }
}

struct AlternateScreenGuard;

impl AlternateScreenGuard {
    fn enter() -> Result<Self> {
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[?1049h")?;
        stdout.flush()?;
        Ok(Self)
    }
}

impl Drop for AlternateScreenGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[?1049l");
        let _ = stdout.flush();
    }
}

struct CursorHideGuard;

impl CursorHideGuard {
    fn hide() -> Result<Self> {
        let mut stdout = io::stdout();
        stdout.write_all(b"\x1b[?25l")?; // Hide cursor
        stdout.flush()?;
        Ok(Self)
    }
}

impl Drop for CursorHideGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = stdout.write_all(b"\x1b[?25h"); // Show cursor
        let _ = stdout.flush();
    }
}
