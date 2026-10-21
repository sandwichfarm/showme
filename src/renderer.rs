use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use crate::backend::{Backend, BackendFactory, BackgroundStyle, RenderOptions};
use crate::capabilities::{current_terminal_size, detect_terminal_backend, ensure_tty_stdout};
use crate::config::{BackendKind, BackgroundColor, Config};
use crate::error::{Result, RimgError};
use crate::image::{Frame, ImageSequence, load_configured_image};
use crate::tmux;

pub struct Renderer {
    config: Config,
    backend: Box<dyn Backend + Send + Sync>,
    terminal: crate::capabilities::TerminalSize,
    background: BackgroundStyle,
    clock: Box<dyn Clock>,
    pool: Option<rayon::ThreadPool>,
    cancelled: Arc<AtomicBool>,
}

impl Renderer {
    pub fn build(config: Config) -> Result<Self> {
        let file_output = config.output_file.is_some();
        if !file_output {
            ensure_tty_stdout()?;
        }
        // File output has deterministic geometry and never probes an unrelated terminal.
        let terminal = if file_output {
            crate::capabilities::TerminalSize {
                columns: config
                    .sizing
                    .width_cells
                    .unwrap_or(80)
                    .clamp(1, u16::MAX as u32) as u16,
                rows: config
                    .sizing
                    .height_cells
                    .unwrap_or(24)
                    .clamp(1, u16::MAX as u32) as u16,
                width_pixels: None,
                height_pixels: None,
            }
        } else {
            current_terminal_size()
        };
        let guess = detect_terminal_backend();

        let backend_kind = match config.backend {
            BackendKind::Auto if file_output => BackendKind::Unicode,
            BackendKind::Auto => guess.backend,
            other => other,
        };

        let backend = match BackendFactory::build(backend_kind) {
            Ok(backend) => backend,
            Err(err) => {
                if config.backend != BackendKind::Auto {
                    return Err(err);
                }
                if !matches!(backend_kind, BackendKind::Unicode) && !config.quiet {
                    eprintln!("warning: {} (falling back to unicode renderer)", err);
                }
                BackendFactory::auto_from_guess(BackendKind::Unicode)
            }
        };

        // Enable tmux passthrough if using graphics protocols in tmux
        if !file_output
            && matches!(backend_kind, BackendKind::Kitty | BackendKind::Iterm2)
            && tmux::in_tmux()
            && tmux::enable_tmux_passthrough()
            && !config.quiet
        {
            eprintln!("Enabled tmux passthrough for graphics protocol");
        }

        let background = background_style(&config);

        // Print verbose terminal info
        if config.verbose {
            eprintln!("Terminal information:");
            eprintln!("  Size: {}x{} cells", terminal.columns, terminal.rows);
            if let Some(w) = terminal.width_pixels {
                eprintln!(
                    "  Pixel size: {}x{}",
                    w,
                    terminal.height_pixels.unwrap_or(0)
                );
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

        let pool = config
            .threads
            .map(|threads| {
                rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .map_err(|err| RimgError::other(format!("failed to create thread pool: {err}")))
            })
            .transpose()?;

        Ok(Self {
            config,
            backend,
            terminal,
            background,
            clock: Box::new(SystemClock(Instant::now())),
            pool,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Supply a flag checked between frames and during playback delays.
    /// Cancellation restores terminal state and returns an interrupted I/O error.
    pub fn with_cancellation(mut self, cancelled: Arc<AtomicBool>) -> Self {
        self.cancelled = cancelled;
        self
    }

    fn check_cancelled(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err(io::Error::from(io::ErrorKind::Interrupted).into())
        } else {
            Ok(())
        }
    }

    pub fn run(&self) -> Result<()> {
        if let Some(path) = &self.config.output_file {
            let mut file = std::fs::File::create(path).map_err(|err| {
                RimgError::other(format!(
                    "failed to create output file '{}': {}",
                    path.display(),
                    err
                ))
            })?;
            self.run_to(&mut file, false)
        } else {
            self.run_to(&mut io::stdout().lock(), true)
        }
    }

    fn run_to(&self, output: &mut dyn Write, terminal_controls: bool) -> Result<()> {
        let mut session = TerminalSession::new(output);
        if terminal_controls {
            session.enter(self.config.alternate_screen, self.config.hide_cursor)?;
        }
        self.run_contents(&mut session)?;
        self.check_cancelled()
    }

    fn run_contents(&self, output: &mut dyn Write) -> Result<()> {
        self.check_cancelled()?;
        if self.config.inputs.is_empty() {
            return Err(RimgError::MissingInput);
        }
        if self.config.max_frames == Some(0)
            || self.config.loops == Some(0)
            || self.config.max_duration == Some(Duration::ZERO)
        {
            return Ok(());
        }

        if self.config.clear_once {
            output.write_all(b"\x1b[2J\x1b[H")?;
        }

        if let Some(grid) = &self.config.grid {
            self.render_grid(
                grid,
                self.config
                    .inputs
                    .iter()
                    .map(|path| self.load_sequence(path)),
                output,
            )?;
        } else {
            for (idx, path) in self.config.inputs.iter().enumerate() {
                // Decode and render one input at a time so the first frame does not
                // wait for the entire input list or retain unrelated media.
                let sequence = self.load_sequence(path)?;
                if self.config.clear_between && (idx > 0 || !self.config.clear_once) {
                    output.write_all(b"\x1b[2J\x1b[H")?;
                }

                if !self.config.quiet {
                    if let Some(title) = self.make_title(idx, &sequence) {
                        writeln!(output, "{title}")?;
                    } else {
                        writeln!(output, "# {} - {}", idx + 1, sequence.path.display())?;
                    }
                }

                if self.config.verbose
                    && let Some(frame) = sequence.first_frame()
                {
                    eprintln!(
                        "  Image {}: {}x{} pixels, {} frames",
                        idx + 1,
                        frame.pixels.width(),
                        frame.pixels.height(),
                        sequence.frames.len()
                    );
                }

                self.render_sequence(&sequence, output)?;

                if let Some(wait) = self.config.wait_between_images
                    && idx + 1 < self.config.inputs.len()
                {
                    self.sleep_interruptible(wait);
                }
            }
        }

        output.flush()?;
        Ok(())
    }

    fn load_sequence(&self, path: &std::path::Path) -> Result<ImageSequence> {
        self.check_cancelled()?;
        let start = Instant::now();
        let load = || load_configured_image(path, &self.config, Some(Arc::clone(&self.cancelled)));
        let sequence = if let Some(pool) = &self.pool {
            pool.install(load)
        } else {
            load()
        }?;
        self.check_cancelled()?;
        if self.config.verbose {
            eprintln!(
                "Loaded '{}': {} frames in {:.3}s",
                path.display(),
                sequence.frames.len(),
                start.elapsed().as_secs_f64()
            );
        }
        Ok(sequence)
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

        // Zero limits mean no output, including single-frame images and scrolling.
        if self.config.max_frames == Some(0)
            || self.config.loops == Some(0)
            || self.config.max_duration == Some(Duration::ZERO)
            || self.config.frame_offset >= sequence.frames.len()
        {
            return Ok(());
        }

        // Handle scrolling animation mode
        if self.config.scroll_animation {
            return self.render_scrolling(sequence, options, stdout);
        }

        // Apply frame offset and limit
        let start_frame = self.config.frame_offset;
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
        } else {
            self.config.loops.unwrap_or(1)
        };

        let mut first = true;
        let mut last_height = 0u32;
        let mut current_loop = 0i32;
        let start_time = self.clock.now();

        loop {
            for frame in frames_to_render {
                self.check_cancelled()?;
                // Check duration limit
                if let Some(max_duration) = self.config.max_duration
                    && self.clock.now().saturating_sub(start_time) >= max_duration
                {
                    return Ok(());
                }

                let rendered = self.backend.render(frame, options)?;

                if !first && last_height > 0 {
                    write!(stdout, "\x1b[{}A", last_height)?;
                }

                let indent = self.indent_for(&rendered);
                self.write_rendered(&rendered, indent, stdout)?;
                stdout.flush()?;

                last_height = rendered.height_cells;
                first = false;
                self.pause(frame.delay, start_time);
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
        let frame = sequence
            .frames
            .get(self.config.frame_offset)
            .ok_or_else(|| RimgError::other("cannot scroll: image has no frames"))?;

        // Render the full image
        let rendered = self.backend.render(frame, options)?;

        if rendered.lines.is_empty() {
            return Ok(());
        }

        let rendered_cells: Vec<Vec<String>> = rendered
            .lines
            .iter()
            .map(|line| split_rendered_line_into_cells(line))
            .collect();

        let image_width = rendered.width_cells as i32;
        let image_height = rendered_cells.len() as i32;
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
        let mut x = if dx < 0 { scroll_x_range } else { 0 };
        let mut y = if dy < 0 { scroll_y_range } else { 0 };
        let mut positions = Vec::new();

        // Scroll until we reach the end
        loop {
            positions.push((x, y));

            if self
                .config
                .max_frames
                .is_some_and(|limit| positions.len() >= limit)
            {
                break;
            }
            let next_x = x.saturating_add(dx).clamp(0, scroll_x_range);
            let next_y = y.saturating_add(dy).clamp(0, scroll_y_range);

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
        } else {
            self.config.loops.unwrap_or(1)
        };

        // Animate the scroll
        let mut first = true;
        let mut current_loop = 0i32;
        let start_time = self.clock.now();

        loop {
            for (scroll_x, scroll_y) in &positions {
                self.check_cancelled()?;
                if self
                    .config
                    .max_duration
                    .is_some_and(|limit| self.clock.now().saturating_sub(start_time) >= limit)
                {
                    return Ok(());
                }
                if !first {
                    // Move cursor back to start
                    if viewport_height > 0 {
                        write!(stdout, "\x1b[{}A", viewport_height)?;
                    }
                }

                // Extract viewport from rendered image
                let start_x = (*scroll_x).max(0) as usize;
                let end_x = start_x.saturating_add(viewport_width.max(0) as usize);
                let start_y = (*scroll_y).max(0) as usize;
                let end_y = start_y.saturating_add(viewport_height.max(0) as usize);

                for row in rendered_cells
                    .iter()
                    .skip(start_y)
                    .take(end_y.saturating_sub(start_y))
                {
                    let mut line = String::from("\x1b[0m");
                    let start = start_x.min(row.len());
                    let end = end_x.min(row.len());
                    for cell in &row[start..end] {
                        line.push_str(cell);
                    }
                    line.push_str("\x1b[0m");
                    stdout.write_all(line.as_bytes())?;
                    stdout.write_all(b"\n")?;
                }

                stdout.flush()?;
                first = false;

                // Delay between scroll steps
                self.pause(self.config.scroll_delay, start_time);
            }

            current_loop += 1;

            // Check if we should continue looping
            if loop_count >= 0 && current_loop >= loop_count {
                break;
            }
        }

        Ok(())
    }

    fn pause(&self, delay: Duration, start: Duration) {
        let delay = if let Some(limit) = self.config.max_duration {
            delay.min(limit.saturating_sub(self.clock.now().saturating_sub(start)))
        } else {
            delay
        };
        self.sleep_interruptible(delay);
    }

    fn sleep_interruptible(&self, duration: Duration) {
        let end = self.clock.now().saturating_add(duration);
        while !self.cancelled.load(Ordering::Relaxed) && self.clock.now() < end {
            self.clock.sleep(
                end.saturating_sub(self.clock.now())
                    .min(Duration::from_millis(25)),
            );
        }
    }

    fn print_frame(
        &self,
        frame: &Frame,
        options: RenderOptions,
        stdout: &mut dyn Write,
    ) -> Result<()> {
        let rendered = self.backend.render(frame, options)?;
        let indent = self.indent_for(&rendered);
        self.write_rendered(&rendered, indent, stdout)?;
        stdout.flush()?;
        Ok(())
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

        if self.backend.supported_kind() != BackendKind::Unicode {
            if let Some(pad) = &padding {
                stdout.write_all(pad.as_bytes())?;
            }
            // Continuations are adjacent control sequences, not terminal rows.
            for sequence in &rendered.lines {
                stdout.write_all(sequence.as_bytes())?;
            }
            stdout.write_all(b"\n")?;
            return Ok(());
        }

        for line in &rendered.lines {
            if let Some(pad) = &padding {
                stdout.write_all(pad.as_bytes())?;
            }
            stdout.write_all(line.as_bytes())?;
            stdout.write_all(b"\n")?;
        }
        Ok(())
    }

    fn render_grid<I: ExactSizeIterator<Item = Result<ImageSequence>>>(
        &self,
        grid: &crate::config::GridOptions,
        mut sequences: I,
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

        let mut row_index = 0;
        while sequences.len() > 0 && row_index < max_rows {
            let effective_columns = sequences.len().min(columns);

            // Use actual terminal width for grid allocation
            let available_cells = self.terminal.columns as usize;
            let reserved_for_gaps =
                (grid.spacing as usize).saturating_mul(effective_columns.saturating_sub(1));
            let per_column =
                available_cells.saturating_sub(reserved_for_gaps).max(1) / effective_columns.max(1);

            let rendered: Vec<_> = sequences
                .by_ref()
                .take(columns)
                .map(|sequence| {
                    let sequence = sequence?;
                    let Some(frame) = sequence.frames.get(self.config.frame_offset) else {
                        return Ok(crate::backend::RenderedFrame {
                            lines: Vec::new(),
                            width_cells: 0,
                            height_cells: 0,
                            delay: Duration::ZERO,
                        });
                    };
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
                .collect::<Result<Vec<_>>>()?;

            let max_lines = rendered
                .iter()
                .map(|frame| frame.lines.len())
                .max()
                .unwrap_or(0);

            if max_lines == 0 {
                row_index += 1;
                continue;
            }

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

            stdout.flush()?;
            row_index += 1;
            if let Some(wait) = self.config.wait_between_rows
                && row_index < max_rows
                && sequences.len() > 0
            {
                self.sleep_interruptible(wait);
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

fn split_rendered_line_into_cells(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut esc = String::new();

    let bytes = line.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' {
            let start = i;
            if i + 1 < bytes.len() && bytes[i + 1] == b'[' {
                i += 2;
                while i < bytes.len() {
                    let b = bytes[i];
                    i += 1;
                    if (0x40..=0x7e).contains(&b) {
                        break;
                    }
                }
                esc.push_str(&line[start..i]);
                continue;
            }

            esc.push('\x1b');
            i += 1;
            continue;
        }

        let ch = line[i..].chars().next().unwrap();
        i += ch.len_utf8();

        let mut cell = String::new();
        if !esc.is_empty() {
            cell.push_str(&esc);
        }
        cell.push(ch);
        cells.push(cell);
        esc.clear();
    }

    cells
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

trait Clock: Send + Sync {
    fn now(&self) -> Duration;
    fn sleep(&self, duration: Duration);
}

struct SystemClock(Instant);

impl Clock for SystemClock {
    fn now(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }
}

// Own the destination borrow so initialization failures and ordinary errors use
// the same cleanup path as successful rendering.
struct TerminalSession<'a> {
    output: &'a mut dyn Write,
    alternate: bool,
    hidden: bool,
}

impl<'a> TerminalSession<'a> {
    fn new(output: &'a mut dyn Write) -> Self {
        Self {
            output,
            alternate: false,
            hidden: false,
        }
    }

    fn enter(&mut self, alternate: bool, hidden: bool) -> Result<()> {
        if alternate {
            self.alternate = true;
            self.output.write_all(b"\x1b[?1049h")?;
        }
        if hidden {
            self.hidden = true;
            self.output.write_all(b"\x1b[?25l")?;
        }
        self.output.flush()?;
        Ok(())
    }
}

impl Write for TerminalSession<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.output.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

impl Drop for TerminalSession<'_> {
    fn drop(&mut self) {
        if self.hidden {
            let _ = self.output.write_all(b"\x1b[?25h");
        }
        if self.alternate {
            let _ = self.output.write_all(b"\x1b[?1049l");
        }
        if self.hidden || self.alternate {
            let _ = self.output.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::RenderedFrame;
    use clap::Parser;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct TestClock(Arc<Mutex<Duration>>);
    impl Clock for TestClock {
        fn now(&self) -> Duration {
            *self.0.lock().unwrap()
        }
        fn sleep(&self, duration: Duration) {
            *self.0.lock().unwrap() += duration;
        }
    }

    struct MarkerBackend;
    impl Backend for MarkerBackend {
        fn name(&self) -> &'static str {
            "markers"
        }
        fn supported_kind(&self) -> BackendKind {
            BackendKind::Unicode
        }
        fn render(&self, frame: &Frame, _: RenderOptions) -> Result<RenderedFrame> {
            Ok(RenderedFrame {
                lines: frame
                    .pixels
                    .rows()
                    .map(|row| row.map(|pixel| pixel[0] as char).collect())
                    .collect(),
                width_cells: frame.pixels.width(),
                height_cells: frame.pixels.height(),
                delay: frame.delay,
            })
        }
    }

    fn renderer() -> Renderer {
        let input = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/red.ppm");
        let config = crate::Cli::try_parse_from([
            "showme",
            "--quiet",
            "--width-stretch",
            "2",
            input.to_str().unwrap(),
        ])
        .unwrap()
        .into_config()
        .unwrap();
        Renderer {
            background: background_style(&config),
            config,
            backend: Box::new(MarkerBackend),
            terminal: crate::capabilities::TerminalSize {
                columns: 80,
                rows: 24,
                width_pixels: None,
                height_pixels: None,
            },
            clock: Box::new(TestClock::default()),
            pool: None,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    fn sequence(markers: &str, delay: Duration) -> ImageSequence {
        ImageSequence {
            path: "markers.ppm".into(),
            frames: markers
                .bytes()
                .map(|marker| Frame {
                    pixels: image::RgbaImage::from_pixel(1, 1, image::Rgba([marker, 0, 0, 255])),
                    delay,
                })
                .collect(),
        }
    }

    fn text(output: &[u8]) -> String {
        let input = String::from_utf8(output.to_vec()).unwrap();
        let mut chars = input.chars();
        let mut plain = String::new();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                assert_eq!(chars.next(), Some('['));
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            } else {
                plain.push(c);
            }
        }
        plain
    }

    #[test]
    fn frame_order_offset_limit_and_finite_loops_reach_the_writer() {
        let mut renderer = renderer();
        renderer.config.frame_offset = 1;
        renderer.config.max_frames = Some(2);
        renderer.config.loops = Some(2);
        let mut output = Vec::new();
        renderer
            .render_sequence(&sequence("abcd", Duration::from_millis(2)), &mut output)
            .unwrap();
        assert_eq!(text(&output), "b\nc\nb\nc\n");
    }

    #[test]
    fn zero_limits_and_offsets_beyond_eof_emit_no_frames() {
        for case in 0..5 {
            let mut renderer = renderer();
            match case {
                0 => renderer.config.max_frames = Some(0),
                1 => renderer.config.loops = Some(0),
                2 => renderer.config.max_duration = Some(Duration::ZERO),
                3 => renderer.config.frame_offset = 3,
                _ => renderer.config.frame_offset = usize::MAX,
            }
            for scrolling in [false, true] {
                renderer.config.scroll_animation = scrolling;
                let mut output = Vec::new();
                renderer
                    .render_sequence(&sequence("abc", Duration::ZERO), &mut output)
                    .unwrap();
                assert!(output.is_empty());
            }
        }
        renderer()
            .render_sequence(&sequence("", Duration::ZERO), &mut Vec::new())
            .unwrap();
    }

    #[test]
    fn duration_limits_cap_frame_sleep_and_end_infinite_loops() {
        let mut renderer = renderer();
        let clock = TestClock::default();
        renderer.clock = Box::new(clock.clone());
        renderer.config.loop_forever = true;
        renderer.config.max_duration = Some(Duration::from_millis(10));
        let mut output = Vec::new();
        renderer
            .render_sequence(&sequence("ab", Duration::from_secs(30)), &mut output)
            .unwrap();
        assert_eq!(text(&output), "a\n");
        assert_eq!(clock.now(), Duration::from_millis(10));
    }

    fn scrolling_renderer() -> (Renderer, ImageSequence) {
        let mut renderer = renderer();
        renderer.terminal.columns = 2;
        renderer.terminal.rows = 1;
        renderer.config.scroll_animation = true;
        let mut sequence = sequence("a", Duration::ZERO);
        sequence.frames[0].pixels =
            image::RgbaImage::from_fn(5, 1, |x, _| image::Rgba([b'a' + x as u8, 0, 0, 255]));
        (renderer, sequence)
    }

    #[test]
    fn scrolling_honors_frame_limits_and_loops() {
        let (mut renderer, sequence) = scrolling_renderer();
        renderer.config.max_frames = Some(2);
        renderer.config.loops = Some(2);
        let mut output = Vec::new();
        renderer.render_sequence(&sequence, &mut output).unwrap();
        assert_eq!(text(&output), "ab\nbc\nab\nbc\n");
    }

    #[test]
    fn negative_scrolling_starts_at_the_far_edge_and_does_not_overflow() {
        let (mut renderer, sequence) = scrolling_renderer();
        renderer.config.scroll_dx = i32::MIN;
        let mut output = Vec::new();
        renderer.render_sequence(&sequence, &mut output).unwrap();
        assert_eq!(text(&output), "de\nab\n");
    }

    #[test]
    fn scrolling_duration_caps_delay_and_stops_infinite_playback() {
        let (mut renderer, sequence) = scrolling_renderer();
        let clock = TestClock::default();
        renderer.clock = Box::new(clock.clone());
        renderer.config.loop_forever = true;
        renderer.config.max_duration = Some(Duration::from_millis(15));
        renderer.config.scroll_delay = Duration::from_secs(30);
        let mut output = Vec::new();
        renderer.render_sequence(&sequence, &mut output).unwrap();
        assert_eq!(text(&output), "ab\n");
        assert_eq!(clock.now(), Duration::from_millis(15));
    }

    #[test]
    fn grid_preserves_spacing_and_row_limit() {
        let renderer = renderer();
        let grid = crate::config::GridOptions {
            columns: std::num::NonZeroUsize::new(2).unwrap(),
            spacing: 3,
            rows: std::num::NonZeroUsize::new(1),
        };
        let sequences = vec![
            sequence("a", Duration::ZERO),
            sequence("b", Duration::ZERO),
            sequence("c", Duration::ZERO),
        ];
        let mut output = Vec::new();
        renderer
            .render_grid(&grid, sequences.into_iter().map(Ok), &mut output)
            .unwrap();
        assert_eq!(text(&output), "a   b\n\n");
    }

    #[test]
    fn ordinary_load_errors_restore_cursor_and_alternate_screen() {
        let mut renderer = renderer();
        renderer.config.inputs = vec!["/this-input-does-not-exist.ppm".into()];
        renderer.config.alternate_screen = true;
        let mut output = Vec::new();
        assert!(renderer.run_to(&mut output, true).is_err());
        assert_eq!(output, b"\x1b[?1049h\x1b[?25l\x1b[?25h\x1b[?1049l");
    }

    #[test]
    fn writer_errors_propagate_and_attempt_cleanup() {
        struct FailNewlineOnce {
            bytes: Vec<u8>,
            failed: bool,
        }
        impl Write for FailNewlineOnce {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if bytes == b"\n" && !self.failed {
                    self.failed = true;
                    return Err(io::ErrorKind::BrokenPipe.into());
                }
                self.bytes.extend_from_slice(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut renderer = renderer();
        renderer.config.alternate_screen = true;
        let mut output = FailNewlineOnce {
            bytes: Vec::new(),
            failed: false,
        };
        assert!(
            matches!(renderer.run_to(&mut output, true), Err(RimgError::Io(err)) if err.kind() == io::ErrorKind::BrokenPipe)
        );
        assert!(output.failed);
        assert!(output.bytes.ends_with(b"\x1b[?25h\x1b[?1049l"));
    }
}
