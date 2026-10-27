# Local four-AUX simulation: punk and metal

Date: 2026-09-29

Status: two real multitrack songs downloaded, prepared and replayed locally.
Tags: #audio #simulation #validation

## Ready to use

No audio interface, mixer, AUX outputs or loopback device is needed. The replay
source streams four synchronized channels from disk into Rust buffers. The current
example measures each channel's RMS level; a later detector can consume the same
buffers. The TUI remains the setup shell. No hardware or DMX output is started.

| Program material | Artist / song | Prepared duration | Why selected |
|---|---|---:|---|
| Punk | The Complainiacs — Etc | 114.016 s | Separate recorded kick, bass DI and two electric guitars |
| Metal | Dark Ride — Hammer Down | 294.9374 s | Separate kick, bass and two long electric-guitar tracks |

Source: Mike Senior's [Cambridge multitrack library](https://cambridge-mt.com/ms2/mtk/).
The library describes these artists as punk/garage rock and heavy metal respectively.
Download links were discovered through an indexed catalog and checked against the
actual archives on Cambridge's download server. The library web pages returned HTTP
403 here; indexed listings and the downloaded source readmes were available.

The bundled readmes identify the songs, give 24-bit/44.1 kHz source formats and provide
educational-use terms excluding commercial use without permission. This preparation
is for the user's local noncommercial educational experiment. Audio is not an open
redistribution asset and is not included in Git, CI, releases or published demos.
The original readmes are preserved locally as `SOURCE-README.txt` in each prepared folder.
See the publisher's [usage FAQ](https://cambridge-mt.com/ms3/mtk-faq/) for other uses.

### Run the simulator

From the repository root:

```sh
# Simulate four live AUX feeds at real-time speed; print levels, no sound playback.
cargo run --locked --release --example aux-replay -- \
  recordings/simulation/punk/aux.wav --realtime

# Repeat a chosen passage in the metal song.
cargo run --locked --release --example aux-replay -- \
  recordings/simulation/metal/aux.wav --start 30 --seconds 20 --realtime

# Process a full song as quickly as possible and save measurements locally.
cargo run --locked --release --example aux-replay -- \
  recordings/simulation/metal/aux.wav > local/metal-aux-levels.csv

# Optional listening through the ordinary sound card; starts only when invoked.
play recordings/simulation/punk/monitor.wav
```

`--start` and `--seconds` are in seconds. No `--realtime` means offline processing.
The CSV contains the source time, frame count and four RMS levels in dBFS; digital
silence is `-inf`. Reference tempos in the source readmes (punk 174, metal 95 BPM)
are provenance only, never supplied to a detector as measured tempo.

The stereo monitor contains the four selected AUXes only, with guitars panned left
and right. It omits vocals, snare, cymbals and additional overdubs, so it is not the
original full-band mix. Listening is independent of the replay clock; launching
`play` alongside the CSV example does not provide sample-locked monitoring.

### Files prepared locally

Each directory under `recordings/simulation/{punk,metal}/` contains:

```text
kick.wav          AUX 1
bass.wav          AUX 2
guitar_1.wav      AUX 3
guitar_2.wav      AUX 4
aux.wav           Same samples interleaved in exactly that order
monitor.wav       Stereo listening aid made from these four AUXes
manifest.json     Routing, source/output hashes, sample counts and conversion details
SOURCE-README.txt Original source notice (retained locally)
```

All prepared WAVs are signed 16-bit PCM at 48 kHz. The four mono files and the
interleaved file are alternative ways to consume the same material. The interleaved
file gives the Rust reader one sample clock, avoiding drift between separate players.

Routing is explicit in [songs.json](../../simulation/songs.json):

| AUX | Etc source | Hammer Down source |
|---|---|---|
| Kick | `01_Kick.wav` | `01_Kick.wav` |
| Bass | `08_BassDI.wav` | `11_Bass1.wav` |
| Guitar 1 | `10_ElecGtr1.wav` | `13_ElecGtr01.wav` |
| Guitar 2 | `11_ElecGtr2.wav` | `14_ElecGtr02.wav` |

These are recorded parts, not AI-separated stems. Guitar labels are virtual source
assignments, not verified identities of two individual musicians. The metal session
contains additional bass, guitar, synth and vocal tracks that are not routed. This
reduction models four selected feeds and does not reproduce every part of the record.
It is useful for input plumbing and initial detector work, not sufficient evidence
of robust recognition across all punk/metal, live bleed or blast beats.

## Rebuild or verify preparation

Requires Python 3.11+ and SoX on the preparation machine; curl is needed for downloads.
No Python packages or DAW are required. On Debian/Pi OS, the external tools are supplied
by the `sox` and `curl` packages. Prepare on the development host, then copy only the
needed prepared files to Pi 1.

```sh
# Verify existing prepared hashes, or build from locally cached original ZIPs.
python3 scripts/prepare-simulation.py

# On a fresh checkout, fetch missing archives and prepare both songs.
python3 scripts/prepare-simulation.py --download

# Prepare just one program's material.
python3 scripts/prepare-simulation.py --song metal --download

# Opt-in complete-song channel/content audit; no network access.
python3 scripts/verify-simulation.py
```

Downloads are roughly 734 MiB combined. They remain in `recordings/downloads/`;
only the mapped four tracks and source readme are extracted during preparation.
Prepared WAVs occupy about 375 MiB combined. Just the two interleaved `aux.wav` files
occupy about 150 MiB, and replay memory does not grow with song length.

Preparation checks pinned archive SHA-256 values, exact member names, input mono
format and complete PCM data. Downloads use a temporary file and are renamed only
after verification. Output is staged before publication. Existing prepared data is
hash-verified instead of blindly overwritten. If a recipe or SoX version changes,
move the old generated song directory aside and rerun; keep the original archive.
A changed upstream checksum requires review of the new source, not bypassing the check.

Processing performed:

1. Preserve sample zero and all leading silence. All eight selected source WAVs have
   a Broadcast Wave time reference of zero; confirmed by inspecting their `bext` chunks.
2. Apply the same fixed 0.5 gain to each source, then high-quality SoX resampling
   from 44.1 to 48 kHz and conversion to signed 16-bit PCM. No independent normalization,
   gating, compression, beat quantization, time stretching or silence trimming.
3. Pad shorter tails to the longest selected channel. Punk needs no padding. Metal
   kick and bass receive 39,579 zero frames (about 0.825 s) at the end; guitars receive none.
4. Merge with unit gain in documented channel order, and create the stereo monitor.
5. Record source/output hashes, exact frame counts, fixed gain, padding and SoX version.

SoX 14.4.2 was used with random dither disabled. Existing source level differences
are preserved intentionally; source calibration remains future analyzer work.

## Runtime boundary

[src/replay.rs](../../src/replay.rs) accepts a seekable reader and fills caller-owned
`[f32; 4]` frame buffers. It uses `hound` 3.5.1 for WAV decoding. It rejects unsupported
format or empty recordings, distinguishes EOF from read failures, and returns the
source frame index for every block. Integer sample counts determine timestamps.
The example reuses a 4,800-frame buffer (100 ms, 76,800 bytes of float samples).
A future onset analyzer can request smaller blocks without changing channel meaning.

A seek requires downstream analysis history to be reset. The example does not connect
the scene director: musical energy calibration, onset detection and confidence are
not established yet. File corruption is an error, not silence. A real-time replay uses
an absolute sample-derived deadline to avoid accumulating relative sleep error. It is
best-effort pacing and preserves all samples if delayed; it does not simulate USB
clock drift, capture overruns, device removal, crosstalk or live-room microphone bleed.
Those need explicit fault scenarios and later real-device tests.

## Validation evidence

Measured on the AArch64 development host, not Pi 1:

- Punk: 5,472,768 frames; metal: 14,156,995 frames.
- Every interleaved sample matched its corresponding prepared mono AUX sample.
  All eight channels contained nonzero audio; no prepared channel reached the signed
  16-bit clipping rails. The two guitar files in each song are distinct.
- Complete release-mode replay consumed exactly those frame counts, producing
  1,141 and 2,950 CSV rows. Host elapsed times were approximately 0.25 and 0.57 s;
  these are smoke-test observations, not Pi 1 benchmarks.
- A one-second real-time replay starting at 30 s produced ten blocks / 48,000 frames
  in about 1.001 s. Invalid time values/options and starts beyond EOF were rejected.
- All 19 normal Rust tests passed: five new replay contracts, nine show-policy tests,
  and the existing DMX range and UI checks.
- Six fast Python preparation tests passed (padding/alignment, truncation, explicit
  ZIP extraction, cache integrity and catalog source mapping). These run in normal CI
  without downloads, SoX or copyrighted recordings.
- Formatting, Clippy, release builds and q/Ctrl+C/Esc terminal restoration checks passed.

The full-song audit is opt-in because it needs local third-party media and performs
one-time evidence work. It writes `local/simulation-validation.json`. It is excluded
from the normal suite. Physical fixtures, actual input devices and Pi 1 timing tests
were intentionally not run. Nothing was published or redistributed.

Related: [audio design](0008-audio-and-musical-state.md),
[music research](0015-musical-direction.md), [Pi 1 plan](0014-pi1-feasibility.md).

[Notebook index](../reference/NOTEBOOK.md)
