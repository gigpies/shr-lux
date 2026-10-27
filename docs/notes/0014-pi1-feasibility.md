# Raspberry Pi 1 with 512 MB: feasibility and deployment

Date: 2026-09-29

Status: researched feasibility, proposed budgets; no Pi 1 hardware measurement.
Tags: #hardware #performance #decision

## Answer and scope

**A useful version looks feasible. Make Pi 1 / 512 MB the minimum design target,
then verify it on the actual board before declaring support.** This is an engineering
inference, not a performance result. Prioritize source levels/activity, isolated-kick
onsets, slow energy estimates and a small scene director. Rich chord, solo and section
recognition remain research items. The original [idea](../../idea.md) is preserved.

The user requested this target on this date. The current development host reports
`aarch64`; its successful builds do not verify ARMv6 execution or Pi 1 timing.

## Platform facts and build implications

- Pi 1 uses BCM2835 with a single ARM1176JZF-S core. See the manufacturer's
  [processor documentation](https://www.raspberrypi.com/documentation/hardware/raspberrypi/bcm2835/).
  The 512 MB Model B revision is documented in the
  [RAM announcement](https://www.raspberrypi.com/news/model-b-now-ships-with-512mb-of-ram/).
- B and B+ differ: B+ has four USB ports; B has two. Confirm the actual revision.
  Four connectors do not establish that several audio interfaces work reliably together.
  [B+ specification](https://www.raspberrypi.com/products/raspberry-pi-1-model-b-plus/).
- Use a Pi-compatible **32-bit Lite OS**. The official downloads currently list
  Legacy 32-bit Bookworm Lite and include 1B+ in the compatibility list. Record the
  installed image/version when testing; avoid prescribing a moving “latest” image.
  [Official OS downloads](https://www.raspberrypi.com/software/operating-systems/).
- Rust's `arm-unknown-linux-gnueabihf` target is ARMv6 with the hard-float ABI.
  `armv7-...` and `aarch64-...` are different targets. Match the target ABI and C
  libraries to the selected OS.
  [Rust Arm Linux documentation](https://doc.rust-lang.org/rustc/platform-support/arm-linux.html).

Build on the development machine with pinned Rust 1.97.1. Avoid depending on a
native Rust compiler running on the Pi 1. Cross-compilation needs **both** Rust's
ARMv6 standard library and a C compiler/linker/sysroot that support the board.
`rusb` currently builds vendored libusb, so a Rust target alone is insufficient.
Generic distro `armhf` libraries may target newer CPUs; inspect their compatibility.
Do not set `target-cpu=native` on the development host.

Proposed full-build recipe, pending a suitable toolchain/sysroot:

```sh
rustup target add arm-unknown-linux-gnueabihf
# These commands must resolve to an ARMv6-compatible toolchain using the Pi OS sysroot.
export CC_arm_unknown_linux_gnueabihf=arm-linux-gnueabihf-gcc
export AR_arm_unknown_linux_gnueabihf=arm-linux-gnueabihf-ar
export CARGO_TARGET_ARM_UNKNOWN_LINUX_GNUEABIHF_LINKER=arm-linux-gnueabihf-gcc
export CFLAGS_arm_unknown_linux_gnueabihf='-march=armv6 -mfpu=vfp -mfloat-abi=hard'
cargo build --locked --release --target arm-unknown-linux-gnueabihf
```

The flags do not convert incompatible prebuilt C libraries into ARMv6 libraries.
Check ELF architecture, loader and dynamic library requirements, then run `--help`,
preview, TUI and terminal recovery tests on the board. Do not add an unverified cross
image to mandatory CI or change the repository's default target for every developer.

## Proposed resource envelope

These are acceptance targets to measure, not current measurements:

| Resource | Initial target | If exceeded |
|---|---|---|
| Application RSS | <64 MiB steady, <96 MiB peak | Inspect queues, histories and dependencies |
| CPU | <60% of the one core over a rehearsal; no sustained saturation | Lower optional analysis/UI rate; start with two inputs |
| Audio | Two inputs first, then four; device-native 44.1/48 kHz if supported | Verify actual formats and capture overruns |
| Capture period | Try 256 or 512 frames; negotiate supported settings | Increase buffer only after measuring missed deadlines |
| UI | 5–10 updates/s | Reduce redraw rate before compromising capture |
| Show control | 25 updates/s, features near 100/s | Drop old snapshots; never replay a delayed hit backlog |
| Physical hit-to-light latency | Proposed p95 <100 ms | Measure capture, analysis, USB and fixture contributions |
| Soak | 60 minutes, no steady-state capture overruns or memory growth | Fix before live use |

At 48 kHz, four channels of 16-bit PCM need 384,000 bytes/s. A 250 ms buffer
requires 96,000 bytes; even float conversion is modest. This arithmetic establishes
buffer size, not CPU cost or USB reliability. The period example 256/48,000 is
5.33 ms; 512/48,000 is 10.67 ms. ALSA defines frames, periods and buffers separately.
[ALSA frames and periods](https://www.alsa-project.org/wiki/FramesPeriods).

Use fixed ring buffers and bounded feature histories. No full-show audio in RAM.
Capture outside the UI loop, aggregate features across devices using monotonic
timestamps, and expire old estimates. Separate audio interfaces have independent
clocks; feature fusion need not require sample-perfect alignment. Device removal and
ALSA overruns must invalidate estimates rather than resemble a song ending.
[ALSA PCM errors and capture behavior](https://www.alsa-project.org/alsa-doc/alsa-lib/pcm.html).

Start without a desktop, browser, neural inference, or continuous SD-card recording.
Use inexpensive time-domain envelopes before FFTs. If a spectral feature helps,
benchmark one small FFT stream first. Downsample only with appropriate filtering and
only when the feature permits it. Do not assume NEON support on this CPU.

## Hardware acceptance, still pending

1. Record board revision, OS, CPU clock, available RAM, power supply, hub and exact
   audio models. Confirm real independent line inputs and mixer sends.
2. Establish two-input capture with levels, timestamps and overrun counters; then four.
   Test the final USB arrangement including MIDI and uDMX presence.
3. Measure RSS, CPU, maximum feature age and drop counts with an 80x25 TUI during
   a rehearsal. Include dense kick runs and quiet passages. Avoid recording by default.
4. Exercise unplug/replug, capture overrun, UI resizing and input loss. Reconnection
   must not create false kicks or restore stale burst requests.
5. Only after explicit output arming and a verified fixture patch exist, measure
   electrical/visible latency and fixture timeout behavior separately. USB detection
   is not evidence of DMX electrical output.

If the full four-input version misses its budgets, keep Pi 1 useful with kick plus
band mix, slower scene changes and no harmony analysis. Offer richer analysis on
newer hardware using the same feature and show-policy boundaries.

Related: [music and programs](0015-musical-direction.md),
[implementation evidence](0016-show-policy-validation.md), [architecture](0005-architecture.md).

[Notebook index](../reference/NOTEBOOK.md)
