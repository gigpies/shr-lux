# Development and validation

Date: 2026-09-28

Status: current commands and test policy. Tags: #development #testing

Rust is pinned by `rust-toolchain.toml` to installed toolchain 1.97.1. The host's global
1.85 default is unchanged. Commit Cargo.lock for reproducible application dependencies.
Ratatui + Crossterm form the TUI; rusb uses vendored libusb so the scaffold does not
require system libusb development headers. Building still needs a C compiler/linker. Linux MIDI output adds ALSA development
headers (`libasound2-dev`) and `pkg-config`.

```sh
cargo run --locked -- --help
cargo run --locked -- doctor
cargo run --locked
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
python3 scripts/check-simulation.py
cargo build --locked --release
python3 scripts/check-terminal.py
```

## Test classes

- Default: fast address/encoding contract checks, target-screen rendering, undersized UI,
  show dwell/hysteresis, feature validity, burst bounds and recovery; replay channel order,
  timing, seeking, format validation and errors; synthetic audio onset/activity/pulse
  checks, audio-to-show integration, MIDI encoding, bounded writes, queue expiry,
  watchdog blackout and shutdown.
- Default Python preparation checks use tiny generated audio and no network/SoX.
- Default Linux PTY checks: setup and simulation exit keys, simulation EOF/SIGTERM,
  terminal restoration and absence of mouse/clipboard capture.
- Add normal safety, recovery, schema, routing and persistence regressions as those paths exist.
- Physical fixture tests are manual and pending; never run from CI.
- Historical algorithm comparisons, recordings, exhaustive matrices and long benchmarks
  belong in opt-in tests once introduced, with documented commands and saved evidence.
  The complete-song audit is opt-in: `python3 scripts/verify-simulation.py`.
  The composed-show audit is also opt-in:
  `cargo run --locked --release --example show-audition`.
  These require the private recordings; see [simulation evidence](0017-local-aux-simulation.md).
- Run the complete normal suite for shared models, rendering, output, concurrency or safety
  changes. Use focused tests while iterating; do not ask the user to classify tests.

GitHub Actions runs formatting, Clippy, default tests, a release build and PTY checks
on Linux for pushes and pull requests. See [CI details](../../contrib/ci/README.md).
The local build verifies the AArch64 development host, not Pi 1/ARMv6.
See [Pi 1 deployment](0014-pi1-feasibility.md); CI does not emulate fixtures.
No release binaries are published automatically. No distribution license has been chosen.

Related: [terminal contract](0007-terminal-contract.md), [roadmap](0010-roadmap.md).

## 2026-10-03 source checkpoint

The complete normal check script passed on the Pi 5 with `CARGO_INCREMENTAL=0`:
formatting, warning-denied Clippy, 42 Rust tests, six synthetic preparation checks,
the release build and eight Linux PTY exit/recovery cases. Historical full-song
auditions and physical MIDI/DMX tests were intentionally skipped. This checkpoint
preserves the existing simulation, pad-preview and research work; it adds no
hardware acceptance claim.

[Notebook index](../reference/NOTEBOOK.md)
