# Show-policy prototype and validation

Date: 2026-09-29

Status: implemented and tested on the AArch64 development host; Pi 1 execution pending.
Tags: #validation #show

## Implemented scope

[src/show.rs](../../src/show.rs) is an allocation-free, fixed-size director with three
programs: punk, metal and atmospheric. It accepts supplied normalized energy, kick
hits/second, confidence and monotonic observation time. It returns a scene family,
suggested fade duration, burst request/absolute expiry, and input availability.
It has no audio, fixture, USB or MIDI dependencies of its own.

The base scene uses hysteresis, two-second candidate confirmation and program-specific
minimum dwell. Loss of input holds the last scene. Estimates older than 500 ms, future
timestamps, non-finite/out-of-range values and confidence below 0.8 are unavailable.
A caller gap over 500 ms or reversed clock clears transient history and restarts dwell;
it also imposes a burst rest interval. Consumers must call the policy regularly even
without new audio. This does not provide an output watchdog by itself.

Optional dense-kick requests require permission, energy ≥0.8, density ≥8/s and 600 ms
confirmation. Their deadline is 750 ms after entry, followed by at least 12 seconds
rest from that deadline. Permission/input loss cancels early without erasing rest.
Atmospheric always suppresses requests. Output consumers must honor `burst_expires_at`
even between policy ticks. The text preview samples every 100 ms, so it displays a
750 ms deadline cancellation on the next sample (e.g. 30.6 s on, 31.4 s off).

No flash frequency, actual fading, palette selection, tempo, audio detector, fixture
encoding or lighting output is implemented. The TUI remains the setup shell; the
programs are selected in a separate developer preview. No dependencies were added;
Cargo.lock and the pinned Rust toolchain were retained.

## Try it

```sh
cargo run --locked --example show-preview -- punk
cargo run --locked --example show-preview -- metal --bursts
cargo run --locked --example show-preview -- atmospheric
```

The example prints a deterministic 90-second synthetic timeline immediately; it does
not read audio or access hardware. `--bursts` only permits textual requests.
The metal preview was inspected: Drive at 24 s, burst requests at 30.6 and 43.4 s,
Intense at 48 s, unavailable input at 55 s, recovery at 60 s and Calm at 72 s.
The atmospheric preview holds Calm until 48 s and holds Intense beyond the end of
this 90-second timeline due to its 48-second dwell. This illustrates why preset
values require musical rehearsal tuning rather than being treated as universal.

## Checks completed

- Focused show tests: all nine passed.
- `cargo fmt --all -- --check`: passed.
- `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `cargo test --locked --all-targets`: all 14 normal tests passed, including the
  existing DMX address/range and TUI rendering checks.
- `cargo build --locked --release`: passed on AArch64.
- `python3 scripts/check-terminal.py`: q, Ctrl+C and Esc startup/exit/restoration
  checks passed against that release build.
- Installed the pinned toolchain's `arm-unknown-linux-gnueabihf` standard library and
  compiled the pure policy module to an ARMv6 Rust library with:

  ```sh
  rustc --edition=2024 --crate-type lib --target arm-unknown-linux-gnueabihf \
    src/show.rs -o target/libshow-armv6.rlib
  ```

  This checks code generation for this module only. It does not cross-link the full
  application or execute anything on a Pi 1. No ARM C cross compiler was found on
  the development host during this work.

Normal tests cover dwell, short fills, hysteresis, burst permission/energy/density,
confirmation/duration/rest under continuous dense kicks, invalid/unavailable features,
permission removal, caller stalls, clock reversal, and scene recovery after input loss.
These are fast current behavior/recovery tests and remain in the default suite.

Intentionally not run: physical fixture tests, real audio evaluation, Pi 1 soak and
latency benchmarks. No board/capture/fixture configuration was established for these.
There are no historical or exhaustive suites to reclassify. Synthetic audition is a
manual example, not a slow default test. Record future rehearsal evaluations separately.

## Next concrete implementation

Build a replayable PCM level/activity and isolated-kick detector with bounded buffers,
then connect it to ALSA capture and measure on Pi 1. Keep the preview as a small
regression/review aid. Add the fixture renderer and transport only through the existing
arming, known-patch and blackout gates. This research does not establish electrical
DMX operation or musical detection accuracy.

Related: [feasibility](0014-pi1-feasibility.md), [musical research](0015-musical-direction.md),
[normal checks](0012-development-and-validation.md), [roadmap](0010-roadmap.md).

[Notebook index](../reference/NOTEBOOK.md)
