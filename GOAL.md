# Maintenance goal for GPT-6.1 sol

Complete the maintenance work in TODO.md for the showme repository. Implement and verify the fixes across P1, P2, and P3, using each item’s “Done when” criteria as acceptance requirements. Deliver working code, meaningful regression tests, reliable workflows, and accurate documentation.

Read TODO.md fully before starting. It is the scope and historical baseline, not proof of the current state. Recheck findings against the current checkout, and verify current dependency advisories, versions, GitHub runner availability, and native-library compatibility using authoritative sources before selecting upgrades.

Working approach:
- Follow repository instructions. Prefer codebase-memory-mcp for code discovery; index this repository first if needed. Fall back to targeted searches when the graph is unavailable or insufficient.
- Inspect git status and preserve existing work, including untracked files.
- Make a short dependency-aware execution plan, then begin implementation immediately. Continue through all priorities without waiting for approval between routine steps.
- Use the smallest maintainable changes that satisfy acceptance criteria. Keep formatting-only changes, dependency updates, and behavior changes in separate reviewable batches.
- Maintain concise progress notes in TODO.md, preserving its historical findings. For each item, record implementation status, verification evidence, and any remaining blocker. Mark it complete only when its acceptance criteria are demonstrated.
- Choose reasonable behavior where existing contracts provide guidance. Document and test decisions. Ask only when essential information or access is unavailable.

Execution order:

1. Establish the current baseline and address security findings.
Run the relevant existing checks, inspect dependency paths, apply compatible advisory fixes, and replace atty with std::io::IsTerminal. Resolve current vulnerability findings and explicitly document the owner, rationale, and follow-up for remaining informational warnings. Add scheduled and PR dependency auditing. Do not silently suppress findings.

2. Restore reproducible builds and verification.
Repair CI/build runner selection, platform-specific feature matrices, FFmpeg/PDFium/Sixel provisioning, and clean native-library discovery. Cover Linux, Intel and ARM macOS, and Windows according to advertised support. Resolve formatting and Clippy failures without blanket allowances. Declare and test an evidence-based MSRV alongside stable, retain the lockfile, and pin build tooling appropriately.

3. Fix renderer and CLI behavior with regression tests.
Establish a small writer/clock seam or PTY harness that exercises actual rendering. Fix explicit output-file operation without a TTY, deterministic non-terminal sizing, and terminal-control isolation. Test image output, frame ordering and offsets, frame/duration limits, finite loops, grid spacing, and cleanup on errors. Wire force-image/force-video options into loading and reconcile scrolling, zero-value, offset-beyond-EOF, and stdin/file cropping semantics.

Use bounded timeouts for animation and subprocess tests. Assertions must verify observable output or behavior, rather than merely inspecting configuration fields.

4. Complete media, resource, and terminal coverage.
Add small deterministic fixtures and decoder assertions for the formats and edge cases listed in TODO.md, including all EXIF orientations, animation timing, corrupt inputs, and disabled-feature errors. Provision compatible PDFium explicitly for PDF tests.

Bound media loading so a short preview does not decode and retain an entire long video. Handle input-byte, dimension, frame, and page limits where appropriate. Measure peak memory and time to first frame with reproducible commands; avoid introducing a large benchmark framework.

Test terminal probing, piped input preservation, raw-mode/cursor/alternate-screen restoration, interruption, broken pipes, multiplexer escaping, and protocol payload validity. Add coverage artifacts and justified regression gates that expose renderer/media coverage separately.

5. Finish dependency and release maintenance.
Upgrade remaining dependencies in controlled compatible and breaking batches, checking API, ABI, MSRV, and rendering changes. Add automated dependency updates once CI is reliable.

Make release verification apply to the tagged commit. Validate tag/version agreement and ensure required tests, builds, and runtime smoke checks succeed before publication. Unexpected publishing failures must prevent an apparently successful release; handle already-published versions explicitly.

Validate packaged binaries in clean runtime environments and align advertised features and native-library requirements with actual artifacts. Implement and verify release workflow changes without publishing a crate, pushing a release tag, or creating a public release.

6. Complete documentation and workflow hygiene.
Repair cargo run commands and executable library examples, document PDFium configuration and actual backend limitations, reconcile changelog sections, and replace unsupported coverage claims with measured results. Review action versions and permissions, reduce duplicated setup, and make cache keys reflect target/toolchain/features while retaining clean-build verification.

Verification:
Run targeted checks after each coherent change, then run the final relevant matrix, including:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo test --locked --all-features
cargo test --locked --no-default-features --features unicode,kitty,iterm2
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo clippy --locked --no-default-features --features unicode,kitty,iterm2 --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --locked --all-features --no-deps
cargo audit --json
```

Also verify the declared MSRV, portable feature set, examples, coverage, workflow configuration, and platform/runtime cases required by TODO.md. Demonstrate clean native builds with fresh target directories. Cached local success does not establish clean-build or cross-platform success.

Completion:
Every TODO item must have implementation and acceptance evidence, or remain explicitly open with a concrete external blocker. Continue independent work when one item is blocked. Workflow edits alone do not prove CI success; distinguish local results, observed remote results, and pending checks.

Do not mark the goal complete while required work or verification remains outstanding. End with a concise report of changes, checks and results, compatibility decisions, and any unresolved blockers with exact next steps.
