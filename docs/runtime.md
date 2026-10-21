# Running a showme release archive

Extract the complete archive and keep the PDFium library and `pdfium-licenses`
directory beside the executable. PDFium 7881 is included for the archive's CPU
architecture. Its license notices are included with the runtime.

All archives include Unicode, Kitty, iTerm2, QOI, SVG/SVGZ, and PDF support.
Sixel is available when building from source and is not included in these
archives. Video is included for Linux x86_64 and macOS Intel/ARM64. The Linux
ARM64 and Windows archives omit video.

Video-enabled binaries dynamically link FFmpeg. Install compatible FFmpeg
runtime libraries before launching them: `sudo apt-get install ffmpeg` on
Ubuntu 24.04 (the Linux archive's build platform), or `brew install ffmpeg` on
macOS. A different FFmpeg ABI may require rebuilding showme from source. The
release CI runs a real video and PDF preview from the archive on a separate
runner with runtime dependencies provisioned.

Run `showme --version` or `showme --help` after extraction. For a preview without
a terminal, use `showme --quiet --backend unicode --frames 1 -o output.txt image.png`.
`SHOWME_PDFIUM_PATH` can select an exact compatible PDFium library file;
`SHOWME_PDFIUM_DIR` can select its containing directory.

Grid and scrolling layouts require Unicode. Default file output uses Unicode
and 80×24 cells, with explicit width/height overrides. Runtime and terminal
validation scope and outstanding platform checks are recorded in `TODO.md` in
the source repository.
