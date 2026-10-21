# Maintenance TODO — 2026-10-20

Reviewed local commit `7199f9d` (2026-01-21), the tracked lockfile, source and tests, all three workflows, documentation, current crates.io versions, and the RustSec advisory database. This pass changes documentation only.

The existing tests pass locally, but they leave the central rendering workflow entirely untested. Prioritize dependency advisories and a reliable CI baseline before broad dependency upgrades, then cover actual CLI/media behavior.

## Verified baseline

Environment: Apple Silicon macOS, Rust/Cargo 1.92.0, installed FFmpeg 8 libraries and libsixel 1.10.5. These results do not establish compatibility with current stable Rust, Linux, or Windows.

| Check | Result |
| --- | --- |
| `cargo test --locked` | **Pass:** 39 unit + 41 integration + 1 doc test |
| `cargo test --locked --all-features` | **Pass:** 40 unit + 41 integration + 1 doc test, using the existing build directory |
| `cargo test --locked --no-default-features --features unicode,kitty,iterm2` | **Pass:** 39 unit + 41 integration + 1 doc test |
| `cargo fmt --all -- --check` | **Fail:** formatting changes needed across source, examples, and tests |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | **Fail:** library compilation reports 54 diagnostics; later targets may expose more |
| `RUSTDOCFLAGS='-D warnings' cargo doc --locked --all-features --no-deps` | **Pass:** rustdoc succeeds; compilation still emits unused-import/dead-code warnings |
| `cargo audit --json` using cargo-audit 0.22.2 | **Fail:** 2 vulnerability findings and 6 informational warnings across 7 distinct packages |
| All-feature LLVM coverage using cargo-llvm-cov 0.9.1 | **52.63% lines** (1,818/3,454), **54.11% functions** (158/292) |
| `showme --backend unicode -o output.txt sample.ppm` with captured stdout | **Fail:** exits 1 with “standard output is not a tty” |
| `cargo run --locked -- --help` | **Fail:** ambiguous between `showme` and `ascii-title` |

Coverage required `LIBRARY_PATH=/opt/homebrew/lib` to link the installed libsixel. The first instrumented build, in a separate target directory, failed with `library 'sixel' not found`. Passing tests in an existing target directory therefore do not prove that a clean all-feature build works. Coverage ran unit/integration tests, excludes doctests and native dependency internals, and includes in-source test bodies. Branch coverage was not measured.

The newest GitHub runs returned by the API are the failed [CI run](https://github.com/sandwichfarm/showme/actions/runs/19011814931) and [Build run](https://github.com/sandwichfarm/showme/actions/runs/19011814930) from 2025-11-02, for `5de1f6c`, not the reviewed local commit. Those statuses establish historical failures, not their causes or the status of the January commit.

## P1 — Address before the next release

- [x] **Patch the lockfile advisories and replace `atty`.** First update the compatible versions listed below; replace the `atty` calls in [src/capabilities.rs](src/capabilities.rs) with `std::io::IsTerminal`. Recheck the dependency tree, run the feature matrix, and make `cargo audit` a scheduled and PR check. Treat informational warnings explicitly rather than silently ignoring them. Done when both vulnerability findings are resolved and each remaining warning has an owner and documented disposition.

  | Locked dependency | Finding / dependency path | Required action |
  | --- | --- | --- |
  | `bytes 1.10.1` | Vulnerability; `showme → pdfium-render → bytes` | Update to **≥1.11.1**. [RUSTSEC-2026-0007](https://rustsec.org/advisories/RUSTSEC-2026-0007.html) |
  | `crossbeam-epoch 0.9.18` | Vulnerability; Rayon/EXR dependency chain | Update to **≥0.9.20**. [RUSTSEC-2026-0204](https://rustsec.org/advisories/RUSTSEC-2026-0204.html) |
  | `anyhow 1.0.100` | Unsoundness warning | Update to **≥1.0.103**, or remove if unused. [RUSTSEC-2026-0190](https://rustsec.org/advisories/RUSTSEC-2026-0190.html) |
  | `memmap2 0.9.9` | Unsoundness warning; `resvg → usvg → fontdb` | Update to **≥0.9.11**. [RUSTSEC-2026-0186](https://rustsec.org/advisories/RUSTSEC-2026-0186.html) |
  | `atty 0.2.14` | Both unmaintained and Windows unsoundness warnings | Replace with the standard library. [Maintenance advisory](https://rustsec.org/advisories/RUSTSEC-2024-0375.html), [unsoundness advisory](https://rustsec.org/advisories/RUSTSEC-2021-0145.html) |
  | `rustybuzz 0.18.0`, `ttf-parser 0.24.1` | Unmaintained SVG/font dependencies | Check whether the selected `resvg` upgrade removes them; otherwise track upstream migration and document the remaining exposure. [Rustybuzz advisory](https://rustsec.org/advisories/RUSTSEC-2026-0206.html), [ttf-parser advisory](https://rustsec.org/advisories/RUSTSEC-2026-0192.html) |

  These are dependency/version matches, not demonstrated exploits in showme. In particular, the `atty` unsoundness warning is Windows-specific and describes allocator-dependent alignment; the other memory-safety advisories concern particular APIs whose reachability was not established in this pass. Database snapshot: `e6f94f3760d5e6c54c1b05f204002263bc025524`, 1,348 advisories.

- [x] **Restore a reproducible CI/build matrix.** Update [build.yml](.github/workflows/build.yml) and [release.yml](.github/workflows/release.yml), which both select retired `macos-13` runners. GitHub retired that image on December 4, 2025; choose a currently supported Intel runner such as `macos-15-intel` and verify its lifecycle when implementing. [Retirement notice](https://github.com/actions/runner-images/issues/13046), [current runner inventory](https://github.com/actions/runner-images/blob/main/README.md).

  In [ci.yml](.github/workflows/ci.yml), define features per platform: portable Unicode/Kitty/iTerm2/QOI/SVG/PDF builds everywhere, video where FFmpeg development libraries are provisioned, and a separate supported Sixel job. CI currently requests all features on every OS, while Windows release builds disable video. Its Chocolatey FFmpeg setup needs verification of headers/import libraries, not merely an executable. Resolve clean Sixel discovery/building: `sixel-sys 0.3.1` runs a bundled configure/make build and links dynamically, so merely adding a system package may be insufficient. Done when fresh Linux, Intel/ARM macOS, and Windows jobs pass their advertised feature sets without relying on a previous target cache.

- [x] **Make formatting and lint gates pass.** Run rustfmt, remove the unused `scale_frame` imports in Kitty/iTerm2, resolve the unused `scale_frame_for_graphics` implementation, then address or narrowly justify the remaining Clippy diagnostics. Keep this separate from behavior and dependency changes. Done when formatting and `cargo clippy --locked --all-features --all-targets -- -D warnings` pass, with lint coverage for the minimal build too.

- [x] **Add tests that exercise the renderer and real CLI output.** [src/renderer.rs](src/renderer.rs) has **0/535 covered lines**. The five tests in [tests/integration_tests.rs](tests/integration_tests.rs) construct configuration values and assert fields; they never build or run a renderer. The single executable test in [tests/cli.rs](tests/cli.rs) checks `--help`. Add a small controllable writer/clock seam or PTY harness and test an image reaching output, frame order, offsets, frame limits, finite loops, duration limits, grid spacing, and error cleanup. Done when regressions in `Renderer::run`, `render_sequence`, and `render_grid` fail tests rather than only compilation.

- [x] **Fix explicit output-file use without a terminal.** `Renderer::build` unconditionally calls `ensure_tty_stdout` before inspecting `output_file`. Reproduced with a valid 2×2 PPM and captured stdout. Permit the explicit file-output path, decide sizing when no terminal exists, and keep cursor/alternate-screen control away from unrelated stdout. Done when an executable test produces a nonempty output file without a PTY and verifies failure handling for an unwritable destination.

- [x] **Gate publishing on verification and consistent release results.** [release.yml](.github/workflows/release.yml) publishes the crate independently of binary builds and does not run the test/lint jobs on tag pushes. `create-release` explicitly allows a release when binary builds succeed even if `publish-crate` fails. Validate tag/version agreement, run verification for the tagged commit, and only publish/create the release after required checks succeed. Make the intended already-published case explicit. Done when a failing verification or unexpected publish failure cannot yield an apparently complete release.

## P2 — Dependencies, behavior, and coverage

- [x] **Upgrade dependencies in compatible and breaking batches.** Refresh compatible lockfile entries first, then review terminal/rendering dependencies, then native/media integrations. Do not assume a newer wrapper fixes native-library packaging or all transitive advisories. Add Dependabot or Renovate for Cargo and GitHub Actions once CI is reliable. Recheck versions when implementing; the snapshot below is from the crates.io API on 2026-10-20.

  | Direct dependency | Locked | Latest stable | Upgrade class under current requirement |
  | --- | --- | --- | --- |
  | [anyhow](https://crates.io/crates/anyhow) | 1.0.100 | 1.0.104 | Compatible; advisory fix |
  | [atty](https://crates.io/crates/atty) | 0.2.14 | 0.2.14 | Replace; unmaintained |
  | [base64](https://crates.io/crates/base64) | 0.22.1 | 0.23.1 | Manifest/API review |
  | [clap](https://crates.io/crates/clap) | 4.5.50 | 4.6.7 | Compatible |
  | [crossterm](https://crates.io/crates/crossterm) | 0.27.0 | 0.29.0 | Manifest/API review |
  | [figlet-rs](https://crates.io/crates/figlet-rs) | 0.1.5 | 1.0.0 | Manifest/API review |
  | [kamadak-exif](https://crates.io/crates/kamadak-exif) | 0.5.5 | 0.6.1 | Manifest/API review |
  | [image](https://crates.io/crates/image) | 0.25.8 | 0.25.10 | Compatible; latest declares Rust 1.88 minimum |
  | [once_cell](https://crates.io/crates/once_cell) | 1.21.3 | 1.21.4 | Compatible; consider standard-library replacement |
  | [rayon](https://crates.io/crates/rayon) | 1.11.0 | 1.12.0 | Compatible |
  | [thiserror](https://crates.io/crates/thiserror) | 1.0.69 | 2.0.21 | Manifest/API review |
  | [libc](https://crates.io/crates/libc) | 0.2.177 | 0.2.191 | Compatible |
  | [sixel-sys](https://crates.io/crates/sixel-sys) | 0.3.1 | 0.5.0 | Manifest/API and native-build review |
  | [ffmpeg-next](https://crates.io/crates/ffmpeg-next) | 8.0.0 | 9.0.0 | Manifest/API and FFmpeg ABI review |
  | [qoi](https://crates.io/crates/qoi) | 0.4.1 | 0.4.1 | No newer stable version found |
  | [pdfium-render](https://crates.io/crates/pdfium-render) | 0.8.35 | 0.9.4 | Manifest/API and PDFium ABI review |
  | [resvg](https://crates.io/crates/resvg) | 0.44.0 | 0.48.1 | Manifest/API and font-output review |
  | [assert_cmd](https://crates.io/crates/assert_cmd) | 2.0.17 | 2.2.2 | Compatible |
  | [predicates](https://crates.io/crates/predicates) | 3.1.3 | 3.1.4 | Compatible |

- [x] **Make coverage visible and prevent regressions.** Add `cargo llvm-cov` output as a CI artifact with a modest baseline gate, then raise coverage through behavior tests. Track renderer/media modules independently so parser or configuration tests cannot conceal gaps. Coverage baseline:

  | Area | Line coverage | What needs attention |
  | --- | ---: | --- |
  | `renderer.rs` | 0.00% | Output, animation, grid, scrolling, terminal guards |
  | `video.rs` | 6.75% | Actual decoding, timing, stride handling, errors |
  | `image.rs` | 31.21% | Format loaders, stdin, EXIF transformations, malformed inputs |
  | `backend/image_util.rs` | 42.97% | Scaling, transparency, PNG compression |
  | `capabilities.rs` | 56.25% | Terminal probing/timeouts and non-TTY paths |
  | `cli.rs` | 67.81% | Invalid inputs, conflicting options, boundary values |
  | `backend/unicode.rs` | 79.54% | Pixel/color output and sizing edges |
  | `backend/iterm2.rs` / `kitty.rs` / `sixel.rs` | 86.67% / 90.55% / 90.37% | Payload validity, large chunks, sizing, actual terminal compatibility |
  | `autocrop.rs` / `backend/chunk_util.rs` | 97.12% / 97.53% | Preserve existing coverage |

- [x] **Add small media fixtures and decoder assertions.** Cover PNG/JPEG transparency and dimensions, all eight EXIF orientations, multi-frame GIF timing, QOI, SVG/SVGZ, PDF pages, and a short video. Include corrupt/truncated inputs and disabled-feature errors. The current EXIF tests parse enum values or inspect a generated image; they do not call the orientation transforms. DDS has one uncompressed 2×2 happy-path test. Use bounded fixtures and assert pixel/frame results; run PDF tests with an explicitly provisioned compatible PDFium library.

- [x] **Wire the advertised force-format flags into loading.** `force_video` and `force_image` are parsed into `Config` in [src/cli.rs](src/cli.rs), but the loader dispatch in [src/image.rs](src/image.rs) and [src/renderer.rs](src/renderer.rs) never reads them. This is a source-confirmed gap; no runtime reproduction was attempted. Define behavior for extensionless/misnamed files and unsupported builds, then test it end to end.

- [x] **Apply frame/duration/crop semantics consistently.** Source inspection shows `render_scrolling` does not consult `max_duration` or `max_frames`; `render_sequence` clamps an excessive frame offset to the last frame; the GIF stdin branch omits the cropping applied to file GIFs. Add targeted regressions, decide/document intended semantics, and fix confirmed discrepancies. Include `--loops 0`, `--frames 0`, negative scrolling deltas, an offset beyond EOF, and `--loop --scroll --duration ...` with a hard test timeout.

- [x] **Bound media loading and measure memory.** `load_video` decodes every frame into a `Vec`, PDF loading renders every page, and `load_sequences` loads all inputs in parallel before rendering. Frame/duration limits apply later. Start with early decode limits or a bounded streaming path, plus limits for stdin bytes, dimensions, frames, and pages where appropriate. Done when a short requested preview of a long video does not require decoding/retaining the entire video. Record peak memory and time to first frame for representative images/video; avoid a large benchmark framework until these measurements justify it.

- [x] **Test terminal lifecycle and protocol behavior.** Add PTY tests for query timeout, piped input preservation, raw-mode restoration, cursor/alternate-screen cleanup on ordinary errors and interruption, and broken pipes. Check tmux/screen escaping and terminal-specific output with decoded payload assertions; keep a small manual smoke checklist for Kitty, iTerm2/WezTerm, Sixel, and multiplexers. High coverage of protocol constructors does not verify real terminal rendering.

- [x] **Validate release binaries in clean runtime environments.** Current archives contain only `showme`; FFmpeg is dynamically linked and PDFium is loaded at runtime. Verify dependency discovery on every advertised platform, document or package the required libraries, and test one actual video/PDF per supported artifact. Align the README/release feature table with build flags: Linux release builds use default features, which do **not** include Sixel despite the current “Yes (optional)” entry. These are packaging gaps to validate; this pass did not download and execute released artifacts.

- [x] **Declare and test a supported Rust version.** Edition 2024 is not a minimum toolchain version declaration. Set `package.rust-version` after selecting upgraded dependencies, test that MSRV plus current stable, and establish a consistent lint/format toolchain. Keep `Cargo.lock` tracked, remove its contradictory `.gitignore` entry, and use `--locked` consistently in CI. Pin the `cross` installation to a reviewed version/revision instead of an unpinned Git branch.

## P3 — Documentation and maintenance hygiene

- [x] **Repair documented commands and library examples.** Add `default-run = "showme"` or use `cargo run --bin showme` throughout. Replace commands such as `cargo test config_tests` with `cargo test --test config_tests` when selecting integration-test binaries. Make the README's library snippets compile: `Cli::parse()` requires the `clap::Parser` trait in scope, and the displayed `Config` literal omits required fields. Extract runnable snippets or doctests so documentation drift is caught automatically.

- [x] **Document native dependencies and actual limitations.** Explain PDFium installation/version matching and `SHOWME_PDFIUM_PATH` / `SHOWME_PDFIUM_DIR`; the code supports them but the user docs omit them. State that grid/scrolling currently require Unicode. Replace the “comprehensive test coverage” claim with measured scope, document the output-file behavior once fixed, and reconcile the multiple “Unreleased” changelog sections with actual releases.

- [x] **Refresh and simplify workflow maintenance.** Review current supported versions of `actions/*` and `softprops/action-gh-release@v1`, pin reviewed revisions where appropriate, set explicit minimal permissions, and keep dependency updates automated. Reduce duplicated build/release setup so feature and native-library changes cannot drift. Key build caches by target, toolchain and feature set; retain at least one clean build check.

## Reproduction notes

The commands below reproduce the main checks; optional native libraries must be installed first. No dependency upgrades or source fixes were applied during this review.

```sh
cargo test --locked
cargo test --locked --all-features
cargo test --locked --no-default-features --features unicode,kitty,iterm2
cargo fmt --all -- --check
cargo clippy --locked --all-features --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --all-features --no-deps
cargo audit --json
cargo llvm-cov --locked --all-features --json --output-path coverage.json
```

On the audited machine the coverage command additionally needed `LIBRARY_PATH=/opt/homebrew/lib`. Temporary logs, the dependency-version snapshot, audit JSON, Cargo metadata and coverage JSON were saved under `/tmp/showme-audit-20261020/`; the durable findings and baseline are recorded above. Current versions/advisories should be queried again before implementation.


## Execution progress — 2026-10-21

The historical baseline above is preserved. The active maintenance work follows `GOAL.md`.

- Plan: security fixes → reproducible CI/builds and lint → renderer/CLI regressions → media/resource/terminal coverage → dependency/release maintenance → documentation and final verification.
- Baseline rechecked: `cargo test --locked --no-default-features --features unicode,kitty,iterm2` and `cargo test --locked --all-features` pass on the current macOS host. `cargo fmt --all -- --check` failed (1,383 lines of formatting diff); rustfmt has now been applied in a formatting batch. Cross-platform and clean native builds remain unverified.
- Security implementation: removed unused direct `anyhow`; replaced all `atty` calls with `std::io::IsTerminal`; updated `bytes` to 1.12.1, `crossbeam-epoch` to 0.9.21, and `memmap2` to 0.9.11. Audit against the existing database snapshot reports zero vulnerabilities and two unmaintained-package warnings. Updated feature-matrix tests and scheduled/PR auditing are pending.
- Remaining advisory disposition: `RUSTSEC-2026-0206` (`rustybuzz 0.18.0`) and `RUSTSEC-2026-0192` (`ttf-parser 0.24.1`) are transitive SVG/font dependencies. Owner: showme maintainers. Action: evaluate the current resvg/usvg upgrade and its shaping/font stack; if upstream still retains them, record upstream migration status and reassess each dependency update. No advisory is ignored. Their presence does not establish reachability of a vulnerability.

- Renderer regression batch: introduced a destination-owning cleanup guard and private controllable clock. Explicit file output uses Unicode by default, 80×24 fallback geometry with explicit size overrides, and skips terminal probing/passthrough setup. Cursor/alternate-screen controls stay on the active terminal destination. Zero frames/loops/duration and offsets beyond EOF emit no frames; scrolling now honors frame and duration limits, caps sleeps, and supports negative deltas without arithmetic overflow. Added nine behavior tests plus CLI tests for captured output, unwritable destinations, and the default non-TTY refusal. Minimal feature tests now pass 48 unit + 44 integration + 1 doc test. All-feature results are being rechecked after this batch.
- Lint batch: removed the unused graphics scaler/imports; applied rustfmt and reviewed Clippy fixes. Three argument-count allowances are scoped to pixel-emission, DDS header decoding, and FFmpeg receive boundaries with rationale. All-feature Clippy passed before the renderer batch; minimal Clippy passes after the renderer batch. Final toolchain/feature checks remain pending.
- Added `.github/workflows/security.yml`: scheduled, PR, and main-branch audits, with pinned reviewed action revisions and report artifacts. Vulnerabilities and unsoundness fail the gate; documented unmaintained warnings remain visible. Removed the contradictory `Cargo.lock` ignore rule and set `default-run = "showme"`.

- Compatible dependency batch: refreshed the lockfile to versions compatible with the existing manifest, including image 0.25.10, clap 4.6.7, rayon 1.12.0, ffmpeg-next 8.1.0, pdfium-render 0.8.37, assert_cmd 2.2.2, and predicates 3.1.4. All-feature tests and all-target Clippy pass with Rust 1.92.0. Terminal/rendering API upgrades are the next isolated batch.
- Force-format and crop regressions: `force-image` now bypasses video probing for misnamed files; `force-video` explicitly selects FFmpeg, rejects unsupported stdin video, and reports disabled-feature builds. Unsupported QOI/PDF/SVG/video extensions now report their missing feature. File and stdin GIF cropping use the same requested semantics. Correction to the historical source claim: current-source inspection and the new executable regression show the original file GIF loader ignored cropping too, rather than applying it. Added actual pixel-coordinate assertions for all eight EXIF transforms and an executable infinite-scroll duration timeout test.

- Native/media upgrade batch: ffmpeg-next 9.0.0 (only format/codec/scaling components), sixel-sys 0.5.0 (bundled static libsixel), pdfium-render 0.9.4 with explicit PDFium 7881 ABI, and resvg/usvg 0.48.1. The upgraded native batch passed all-feature tests on Rust 1.92.0 after disabling Autoconf's C23 probe. `.cargo/config.toml` now records that requirement. Installed local Autoconf/Automake/libtool build prerequisites. Selected the tested Rust 1.92 MSRV and pinned development tools to installed stable 1.98.1; final MSRV checks after the resource changes remain pending.
- Security review rejected figlet-rs 1.0.0: its ZIP 0.6.6 dependency introduces RUSTSEC-2026-0367, -0368, and -0369. Retained ZIP-free 0.1.5 explicitly until upstream upgrades ZIP. The current lockfile audit against a refreshed advisory database has zero vulnerabilities and no SVG/font warnings. Remaining warning: RUSTSEC-2024-0436 (`paste 1.0.15`, from the image/EXR SIMD chain). Owner: showme maintainers; disposition: upstream build-time token-pasting dependency, no vulnerability reported, keep warning visible and review with image/EXR updates. No advisories are ignored. This supersedes the earlier rustybuzz/ttf-parser follow-up, which the new resvg/font stack resolved.
- Resource batch implementation: bounded byte readers, dimension limits, GIF/video decode budgets (1024 frames / 512 MiB retained RGBA), PDF page and bitmap limits, early preview stopping, and per-input loading. Grids load/decode/render one image at a time and retain only the row's rendered output. Added a preview regression that stops before a corrupt later GIF frame and an extensionless forced-video CLI regression. Runtime measurements, additional boundary tests, and final gates remain pending.
- Native decoder tests: synthetic QOI/SVG/SVGZ/PDF/FFV1 fixtures plus generated PNG/JPEG/GIF assert real pixels, dimensions, timing, and frame/page order. All eight EXIF transforms assert actual coordinate movement. A fresh Rust 1.98.1 all-feature target successfully built bundled Sixel without a system libsixel link-path override. Real PDF tests, using checksum-verified PDFium 7881, caught and fixed a red/blue swap: use pdfium-render's bitmap conversion instead of manually swapping channels. Unit/integration tests pass in the fresh target; final doctests are being repaired and rerun.
- Documentation: README examples are now included in crate doctests, with terminal/file examples compiled via `no_run`. Documented native prerequisites, PDFium ABI/discovery, file output, decode limits, and Unicode-only layouts; corrected integration-test selection commands and the Linux binary's unsupported Sixel claim. Changelog reconciliation, workflow reuse, cross-platform CI, terminal interruption/PTY coverage, coverage gates, release gating/packaging/runtime checks, and the completion audit remain open.

### Current verification — 2026-10-21

The earlier progress notes describe intermediate states. The following evidence
supersedes their pending local checks; native remote jobs remain pending until
their run links and results are recorded.

| Acceptance area | Implementation and current evidence |
| --- | --- |
| Security | Current lockfile has zero RustSec vulnerabilities. The sole informational warning is `paste 1.0.15`, with owner/disposition in `docs/dependency-advisories.json`. The audit gate fails unknown warnings and vulnerabilities; 14 Python gate/provisioning tests pass. Scheduled auditing and reusable PR/tag verification are implemented. |
| Reproducible platform builds | Shared provisioning selects native runners, platform features, pinned PDFium 7881/checksums, and static Sixel prerequisites. Fresh local all-feature compilation passes without system libsixel link-path overrides. Windows portable all-target cross-check passes, but does not prove native execution. Linux, Intel macOS, ARM macOS, and Windows remote jobs are pending. |
| Formatting/lint | Rust 1.98.1 formatting, all-feature/all-target Clippy, and minimal/all-target Clippy pass with warnings denied. The Unicode-only configuration also has a separate gate. Formatting, dependency, lint, behavior, and CI changes are separate commits. |
| Renderer and output files | Fake-clock renderer tests exercise sequence order/offsets, frame/duration limits, finite loops, grids/gaps, scrolling, and cleanup. Executable tests assert nonempty non-TTY output, destination errors, actual forced-format behavior, invalid CLI values, and hard timeouts. |
| Media semantics and fixtures | Actual pixel/timing tests cover PNG, JPEG, GIF, QOI, SVG/SVGZ, PDF pages, FFV1 video, all eight EXIF transforms, malformed DDS, corrupt/truncated inputs, and disabled features. Real PDF tests use checksum-verified PDFium; repeated loads reuse its process-wide bindings. CLI and library-default output agree. |
| Loading limits | Byte/dimension/frame/page/retained-pixel limits and sequential input loading are implemented. A preview can stop before a later corrupt GIF frame. The five-minute video measurement below confirms a one-frame request does not retain all 9,000 frames. Decoder/protocol working buffers are additional to retained-media limits. |
| Terminal/protocol | Unix PTY tests pass for query timeout, cell-size reply, existing raw mode, piped input, ordinary errors, Ctrl-C during long waits/pending stdin, and broken output pipes. Decoded PNG/protocol tests cover large Kitty chunks, iTerm2 multipart framing, tmux escaping, and bounded GNU screen DCS packets. Isolated GNU screen byte passthrough was also checked locally. Actual graphics display and Windows console interruption are not established by these tests; `docs/terminal-smoke.md` records the manual checklist. |
| Coverage | All-feature LLVM coverage with real PDF/PTY tests: **82.94%** lines (3,564/4,297), renderer **87.38%**, image **81.11%**, video **86.42%**. Independent gates require 80/80/75/80%, respectively. Includes in-source test bodies, excludes doctests/native dependency internals; branch coverage was not measured. Remote coverage artifact/gate is pending. |
| Dependencies/MSRV | Compatible updates precede API/native upgrades. Removed unused dependencies; Rust 1.92.0 all-feature tests including PDF/PTY and 15 doctests pass in a fresh target. Current stable 1.98.1 is pinned for development/quality. figlet-rs remains at ZIP-free 0.1.5 to avoid the new ZIP advisories. Native CI tests stable and MSRV; no cross-tool installation is needed with native ARM runners. |
| Release gates | Tag/manifest agreement precedes verification; binaries and their separate runtime jobs precede crate publishing; successful publishing precedes release creation. Publishing errors propagate. Only registry API 404 establishes an unpublished version; an existing non-yanked archive must pass its registry checksum and exact packaged-file comparison. Negative gate tests cover mismatches, registry failures, and package/publish failures. No crate, release tag, or public release has been published. |
| Runtime archives | PDFium and its license notices accompany binaries. The current optimized Mac ARM archive passes actual image/PDF/video previews from a temporary directory with PDFium overrides cleared. Separate fresh runtime jobs cover all five archives; their remote results are pending. |
| Docs/workflow hygiene | README and library-guide Rust snippets compile as 15 doctests; `Config::default()` supports a real programmatic example. Commands, native requirements, file output, layout limitations, measured coverage, changelog history, and archive features are corrected. Reviewed SHA-pinned actions, minimal permissions, reusable setup/workflows, download-only cache keys, Dependabot, and actionlint pass local validation. |

### Reproducible preview measurements

Local Apple Silicon macOS, Rust 1.98.1 optimized release, FFmpeg 8.0.1,
one process per measurement, no concurrent compilation. Output uses the default
80×24 non-terminal geometry and Unicode. First-output times are upper bounds
from polling every 2 ms; these single runs are observations, not performance
guarantees or total-process memory limits.

```sh
cargo build --release --locked --bin showme --no-default-features \
  --features unicode,kitty,iterm2,qoi,pdf,svg,video
mkdir -p /tmp/showme-preview-measurements
ffmpeg -hide_banner -loglevel error -y -f lavfi -i testsrc2=size=4096x2048:rate=1 -frames:v 1 -threads 2 \
  /tmp/showme-preview-measurements/large.png
ffmpeg -hide_banner -loglevel error -y -f lavfi -i testsrc2=size=256x144:rate=30 -t 300 -c:v ffv1 -threads 2 \
  /tmp/showme-preview-measurements/long.mkv
python3 scripts/measure_preview.py --binary target/release/showme \
  /tmp/showme-preview-measurements/large.png
python3 scripts/measure_preview.py --binary target/release/showme \
  /tmp/showme-preview-measurements/long.mkv
```

| Input | One-frame first output (upper bound) | Peak child RSS | Output |
| --- | ---: | ---: | ---: |
| 4096×2048 PNG, 824,034 bytes | 370.72 ms | 68,272,128 bytes (65.11 MiB) | 48,035 bytes |
| 256×144 FFV1, 300 seconds / 9,000 frames / 51,923,572 bytes | 15.46 ms | 10,223,616 bytes (9.75 MiB) | 50,384 bytes |

`ffprobe -count_packets` confirms 9,000 video packets and 300 seconds. Retaining
all frames as RGBA alone would require about 1.24 GiB, exceeding the observed
RSS and the configured retained-frame budget. Local temporary reports are under
`/tmp/showme-preview-measurements`; the optimized archive smoke used
`scripts/package_binary.py` followed by `scripts/smoke_binary.py --archive ...
--video true`.

- Local completion update: the ten checked items above meet their stated acceptance criteria with the recorded local tests and measurements. Native CI, archive runtime results, coverage artifacts, and final workflow checks remain open. `cargo package --locked` successfully packaged and verified the source crate (90 files), including the documentation used by doctests. Draft PR: https://github.com/sandwichfarm/showme/pull/1. The first native test jobs exposed a cache-key restriction (commas are invalid); feature names are now normalized before constructing the key.
- First remote CI evidence: the Linux all-feature coverage and dependency-audit jobs passed in [CI run 39632715564](https://github.com/sandwichfarm/showme/actions/runs/39632715564). Quality passed Rust/Python checks but failed to install actionlint through a Rust-only installer; its official Linux binary is now downloaded with a pinned SHA-256 instead. Both Linux release package builds passed in the still-running [Build run 39632715576](https://github.com/sandwichfarm/showme/actions/runs/39632715576). These partial results do not complete the platform/runtime requirements.


### Native verification and release-history follow-up

- [Build run 39633246076](https://github.com/sandwichfarm/showme/actions/runs/39633246076) is **successful** at `c57f35c`: all five native packages and all five separate fresh runtime jobs passed. Each archive executed an image and PDF preview with PDFium overrides cleared; Linux x64 and both macOS archives also executed video. Linux ARM64 and Windows correctly omit video. No Cargo build cache or development-library provisioning is used by the runtime jobs.
- [CI run 39633246008](https://github.com/sandwichfarm/showme/actions/runs/39633246008) has passed audit, quality, coverage, Windows, Linux x64/ARM, Rust 1.92 MSRV, and ARM macOS. Intel macOS has passed portable and advertised native features; its final minimal-feature phase is still running. The reproducible-matrix checkbox remains open until the complete run succeeds.
- The downloaded Linux coverage artifact independently passes the 80/80/75/80% gates and reports the same 82.94/87.38/81.11/86.42% line measurements recorded locally. The corrected actionlint binary installation and normalized cache identities passed on the hosted runner.
- Both the basic CLI and programmatic README snippets also compile as a separate consumer crate using the README's actual dependency declarations. This exposed the need to declare `clap` directly; the examples now use the current local checkout API, rather than promising unpublished APIs in the registry's 0.1.1.
- crates.io confirms 0.1.1 was published on 2026-01-21; its checksum-verified archive records VCS commit `7199f9d6ceec7fd24ac5bb3ff831d4daf08dded8` and includes the DDS/fit-height/compression changes. The changelog now has a dated 0.1.1 crate section separate from maintenance work under Unreleased. A new version must be selected before tagging these maintenance changes. The real registry archive differs from this candidate, and the release guard correctly rejects treating it as an identical retry.
- The remaining manual terminal checklist records display compatibility and native Windows console interruption as unverified manual scope; the requested checklist and automated Unix lifecycle/protocol acceptance tests are complete. No graphics display compatibility is inferred from payload coverage.


### Completion audit

All 18 original TODO acceptance items are checked against the implementation,
observable regression tests, measured results, or actual hosted jobs cited above.
The historical findings remain unchanged apart from their completion boxes.

[CI run 39633246008](https://github.com/sandwichfarm/showme/actions/runs/39633246008)
and [Build run 39633246076](https://github.com/sandwichfarm/showme/actions/runs/39633246076)
are both **successful** at `c57f35cd9c69236a87bf619180eb5df9ba578354`:
all nine verification jobs and all ten package/runtime jobs passed. Intel macOS
completed its minimal phase successfully, closing the last native matrix item.
The subsequent changes correct documentation and release-history evidence;
15 doctests, strict rustdoc, and compilation of a separate README consumer crate
pass with those corrections.

Final local evidence includes the required locked default/all/minimal test
matrix; all/minimal/Unicode-only all-target Clippy with warnings denied;
rustfmt; strict rustdoc; real native PDF/video/Sixel and Unix PTY tests; fresh
MSRV/native builds; a successful `cargo package --locked`; 14 Python tests;
actionlint; audited dependencies; independent coverage gates; and the recorded
preview memory/first-output measurements. The CI workflows independently exercise
native stable/MSRV/platform features, package actual binaries with pinned PDFium,
and test their runtime discovery on separate runners.

No implementation or required automated verification remains open. The delivered
manual display/console checklist is intentionally recorded as manual scope,
not a claim that all named terminals were visually tested. The `paste` warning
and retained FIGlet version have explicit dispositions. A new package version,
release tag, crate publishing, public release, and merging the draft PR are
maintainer release actions; none was performed as part of maintenance validation.
