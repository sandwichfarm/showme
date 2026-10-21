# Terminal compatibility smoke checks

Run these on each named terminal before claiming display compatibility. Record
terminal/version, OS, backend, multiplexer/version, and the commands/results in
`TODO.md`. Automated payload and PTY tests establish protocol bytes and lifecycle
behavior, but do not prove that each terminal displays the content correctly.

| Environment | Backend | Check |
| --- | --- | --- |
| Kitty / Ghostty | `kitty` | Image colors/alpha, an image large enough for multiple chunks, animation replacement |
| iTerm2 / WezTerm | `iterm2` | PNG dimensions/colors/alpha, animation replacement |
| A Sixel-capable terminal | `sixel` | Build with Sixel, render colors, check cursor positioning after the image |
| tmux with passthrough | Kitty or iTerm2 | Payload passthrough, display, and cleanup after Ctrl-C |
| GNU screen | A supported graphics backend | DCS escaping, display, and cleanup after an ordinary error |
| Any Unicode terminal | `unicode` | Grid gaps, scrolling in both directions, finite loops, duration and frame limits |

Suggested inputs are `tests/fixtures/red.ppm`, `colors.svg`, `short.mkv`, and
`two-pages.pdf` (PDFium must be installed). Include a transparent PNG, a larger
image, and a multi-frame GIF. Verify `--alternate-screen` returns to the original
screen, the cursor becomes visible, and shell input/echo still work after errors
and Ctrl-C during playback or a slideshow wait. Check a piped image with `cat
image.png | showme -` and confirm input bytes are not consumed by terminal probes.

The automated Unix PTY suite in `scripts/pty_smoke.py` covers query timeout,
character-cell replies, normal and existing raw-mode restoration, piped image
preservation, ordinary errors, Ctrl-C during a long wait or pending stdin read,
and a real broken output pipe. It uses terminal-device access unavailable in
some sandboxes. Windows console interruption and real graphics display require
native verification; a cross-compilation check is insufficient evidence.
