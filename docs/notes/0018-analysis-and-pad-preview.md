# Music analysis and MiniLab pad preview

Date: 2026-09-29

Status: implemented recorded-input path; user observed pad activity, burst appearance unconfirmed.
Tags: #audio #analysis #midi #validation

**Follow-up:** the user subsequently confirmed the short white marker. The diagnostic
pad layout described below has been replaced by the [composed show](0019-composed-pad-show.md).
This note preserves the initial analysis and hardware evidence.

## What runs

`simulate punk|metal|atmospheric` streams the prepared four-channel PCM through
`analysis`, `show`, and `preview`. The default destination is an 80×25 terminal.
`--midi` explicitly opens the identified MiniLab mkII. No DMX output exists.
`analyze` runs the same DSP and decisions offline, emitting a 100 Hz CSV.

```sh
cargo build --locked --release
target/release/shr-lux simulate punk
target/release/shr-lux simulate metal --midi --listen --bursts
target/release/shr-lux simulate metal --midi --headless --start 258 --seconds 20 --bursts
target/release/shr-lux pads-test
mkdir -p local/analysis
target/release/shr-lux analyze punk --bursts > local/analysis/punk.csv
target/release/shr-lux analyze metal --bursts > local/analysis/metal.csv
```

`--file` accepts another four-channel PCM16/48 kHz WAV. `--start` and `--seconds`
select a bounded passage. Seeking resets history and scene dwell, so a passage may
produce different decisions from continuous playback. `q`, Esc or Ctrl+C quits;
`b` toggles burst permission and `x` latches preview blackout.

Optional `--listen` launches SoX `play` on sibling `monitor.wav`: the four-AUX mix,
not the full original production. The process is stopped when simulation exits.
It uses the default sound output and a separate playback clock. Start/device latency
and drift are unmeasured; this is an approximate listening aid, not sample-locked
analysis/playback. No physical sound input or loopback is needed.

## Measured features and deliberate limits

The analyzer uses fixed 480-frame windows (10 ms) and bounded arrays. There is no
FFT, source separation, neural model, or allocation in its processing method.

- RMS and hysteretic activity per source, with 250 ms release.
- Kick candidates: low-frequency plus broadband envelope rise, adaptive baseline,
  and 60 ms refractory interval. Isolated kick input is assumed.
- Rolling one-second kick density, distinct from song tempo.
- Regular kick pulse from recent inter-onset intervals, folded into 60–240 BPM.
  Half/double ambiguity remains; irregular patterns report unavailable.
- Calibrated relative energy with roughly 0.5-second and 4-second smoothing.
  The director currently uses the faster energy; slow energy is diagnostic.
- A 500 ms settling period after startup/gaps/clipping; invalid PCM is rejected.
  Confidence passed to the director means usable input, not musical certainty.

File simulation first scans the entire file for each source's 90th-percentile
non-silent RMS. This explicit soundcheck uses bounded memory but adds startup time
and sees future *levels*. Subsequent onset and show decisions are causal. A live
soundcheck/calibration workflow is still needed. The final partial window, under
10 ms, is ignored. Activity is not solo detection. No section, harmony, bar or
downbeat recognition is implemented. The source's published tempo never enters DSP.

Programs retain the [existing policy](0016-show-policy-validation.md): minimum
scene dwell 16/24/48 seconds and fades 2/4/8 seconds for punk/metal/atmospheric.
Scenes require sustained energy changes. Dense energetic kick passages may request
an optional 750 ms burst after 600 ms confirmation, with a 12-second cooldown.
Atmospheric never requests bursts. Thresholds are an initial artistic policy,
not a validated genre classifier. In particular, the recordings below rarely
sustain the intense threshold; manual audition and labelled onsets are next.

The pad preview separates four activity indicators from three scene colors.
Kick indication holds 90 ms. Scene transitions fade and a 32-second modulation
keeps held terminal washes moving. Pad 8 shows a burst as **solid white**; no
optical strobe rate is generated. Real fixture strobing needs its own known patch,
output arming and transport/recovery implementation.

## Controller identification and protocol evidence

Observed USB device: **Arturia MiniLab mkII**, `1c75:0289`. This host assigned
`hw:2,0,0` and `/dev/snd/midiC2D0`; code discovers the USB identity through sysfs
rather than persisting those machine-specific numbers. Exactly one match is required.
The standard MIDI identity request returned:

```text
F0 7E 00 06 02 00 20 6B 02 00 04 02 43 07 00 01 F7
```

Sources read before output implementation:

- [Official MiniLab mkII manual](https://downloads.arturia.net/products/minilab-mkII/manual/minilab-mkii_Manual_1_1_EN.pdf):
  eight physical pads, two pad banks, and separate preset recall behavior.
- [Device-specific firsthand SysEx research](https://github.com/mhugo/sysex) and
  [original Arturia forum report](https://legacy-forum.arturia.com/index.php?topic=92480.0):
  pad-color packets. The manual does not publish this packet layout.

Allowlisted packet:
`F0 00 20 6B 7F 42 02 00 10 pp cc F7`, with pad addresses `70..7F` and
colors `00` off, `01` red, `04` green, `05` yellow, `10` blue, `11` purple,
`14` cyan, `7F` white. No preset store, mapping, global mode, firmware or bootloader
messages are sent. Both logical banks receive the same eight-pad preview.

A pad bank and a user preset are separate concepts. The user's suggestion that
User2 might be required remains unverified. The identity response and successful
writes alone cannot establish whether the default preset displays host colors.
Select a suitable preset before starting; live preset changes or pressing pads
may overwrite colors until the next changed preview value.

The controller supports a discrete palette in this protocol, not RGB intensity.
Smooth terminal fades become color steps on the hardware; pads cannot validate
fixture dimming quality. The output only owns transient colors and clears them
on exit; it does not restore any preexisting transient LED state.

## I/O and recovery

Linux ALSA raw MIDI is nonblocking. A dedicated worker has a one-frame queue;
analysis drops updates when full. Changed colors are sent at most 25 times/second.
Writes have a 40 ms deadline; stale frames and producer stalls clear pads after
250 ms plus the worker's 40 ms polling interval. Wall-clock simulation lag over
250 ms inhibits output. Disconnects terminate the simulation with an error.

Normal exit, EOF, SIGINT/SIGTERM and unwinding close the worker and attempt all-off
on both banks, with a bounded 100 ms drain. Physical disconnect, SIGKILL, process
suspension or power loss cannot guarantee clearing device LEDs; no device-side
watchdog has been established. The watchdog protects a stalled producer while
its worker still runs. Blackout is a transient preview latch, not persistent state.

## Validation evidence

Normal tests cover silence, known 120 BPM pulses, 10 Hz double kicks, activity
release, gaps/clipping/invalid PCM, calibration, complete synthetic WAV → analysis
→ burst/pads integration, burst permission, packet allowlists, partial/stalled
writes, stale frames, producer watchdog and channel closure. Terminal tests cover
setup and simulation exit keys, simulation EOF/SIGTERM, restoration and absence of
mouse/clipboard capture. These tests use no real MIDI device or private recording.

Validation commands passed: `cargo fmt --all -- --check`,
`cargo clippy --locked --all-targets -- -D warnings`, 36 Rust tests via
`cargo test --locked --all-targets`, six Python preparation tests, the release build,
and eight Linux PTY cases. No physical fixture test or exhaustive algorithm sweep
was run; the unchanged media-preparation audit was intentionally not repeated.

Opt-in complete-song analysis produced:

| Prepared source | Analyzed windows | Kick candidates | Peak candidates/s | Scene changes after initial Calm | Burst requests |
| --- | ---: | ---: | ---: | --- | --- |
| Punk: Etc | 11,401 | 354 | 7 | Drive 16.01 s; Calm 99.69 s | 0 |
| Metal: Hammer Down | 29,493 | 1,173 | 14 | Drive 24.01; Calm 48.01; Drive 72.01; Calm 101.15; Drive 125.15; Calm 228.44; Drive 252.44 s | 1, at 262.19–262.93 s |

Counts are detector outputs, **not labelled accuracy scores**. Energy peaked near
0.842 in both files. Pulse was often unavailable; one regular metal passage was
estimated near 187.5 BPM, illustrating possible double-time interpretation.
On this Pi 5 host, `/usr/bin/time` measured the complete release-mode analysis
(including calibration and CSV output): punk 0.52 s, metal 1.42 s; maximum resident
memory 2,592 KiB each. These are warm-cache host measurements, not Pi 1 benchmarks
or whole-system memory estimates. Full CSVs remain ignored under `local/analysis/`. This audition is opt-in and is
not added to CI. The earlier complete-media preparation audit is documented in
[the simulation note](0017-local-aux-simulation.md); no new extraction was needed.

Physical runs: a static 20-second color pattern via `amidi`, then a 20-second Rust
`simulate metal --midi --headless --start 258 --seconds 20 --bursts` run completed
successfully and attempted all-off cleanup. The latter reported up to 13 kick
candidates/s. A follow-up six-second run with event logging recorded the request
on at 262.20 s and off at 262.95 s; log: ignored
`local/analysis/minilab-passage.log`. **Partial visual confirmation:** the user reported mostly static pads and irregular
pulsing on the first pad, described as orange. They did not notice the white burst.
Preset selection and exact color/pad mapping remain unconfirmed. This establishes
visible activity, but does not yet validate the complete preview. MIDI write
success is not proof of optical response; neither test exercises DMX fixtures.

An additional punk run with `--listen --headless` surfaced a playback-process
error on this host: SoX reported “there is no default audio device configured.”
The simulation stopped cleanly. ALSA lists two HDMI playback devices; the connected
MiniLab exposes MIDI and no PCM playback device. The SoX ALSA driver is installed.
Choose/configure the actual listening destination before using `--listen`; SoX
also accepts an `AUDIODEV` environment setting. Audible output and latency remain
unconfirmed; terminal/MIDI replay does not need that path.

Pi 1 compatibility remains a [deployment target](0014-pi1-feasibility.md), not a
measured claim. Current tests run on the Pi 5/AArch64 development host. The bounded
DSP is suitable for evaluating that target, but ARMv6 build, actual CPU margin,
USB capture stability and memory still need target-device measurements.

Related: [audio estimates](0008-audio-and-musical-state.md),
[MIDI](0009-midi-feedback.md), [development checks](0012-development-and-validation.md).

[Notebook index](../reference/NOTEBOOK.md)
