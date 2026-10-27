# Composed eight-fixture show

Date: 2026-09-29

Status: implemented; user approved the composed show and revised storm texture. Tags: #show #midi #validation

The first [pad preview](0018-analysis-and-pad-preview.md) assigned individual LEDs to
source activity. The user confirmed the white marker was visible, but asked for an
actual coordinated light show. This implementation replaces that diagnostic layout.
All eight pads now represent fixtures, ordered left to right; partners are 1/8,
2/7, 3/6 and 4/5. Both MIDI banks still mirror the same show. The TUI uses one row
of eight fixtures and keeps source measurements in a separate panel.

## Composition

A held base look, moving color and restrained accents are rendered separately.
The analyzer and scene director are unchanged. The renderer uses every source:

| Input | Artistic role |
| --- | --- |
| Kick | 120 ms colored accent on a mirrored pair, at most once per 300 ms; sustained dense kicks can qualify for the director's burst |
| Bass | Smooth overall level and the width of the moving color |
| Guitar 1 / 2 | Their smoothed level difference biases opposing sides during Guitar conversation |
| Combined energy / regular kick pulse | Motion speed; pulse steers speed over two seconds without resetting phase |

Four motifs repeat, held for 16 seconds in punk and 24 in metal:

1. **Mirror sweep:** color travels through symmetric pairs.
2. **Crosscurrent:** the two halves move in opposing directions.
3. **Guitar conversation:** slower movement, biased by the two guitar sources.
4. **Center bloom:** color expands/contracts around the central pairs.

These are elapsed-time motifs, not detected bars or song sections. The musical
scene director still requires sustained energy changes and its own dwell. Its
Calm/Drive/Intense decisions recolor the running composition without resetting
motion. Transitions use the program's 2/4-second fades. A source becoming active
does not force a new motif. Silence fades the base out; unavailable input clears it.

Punk uses green/yellow, blue/cyan, and red/yellow for intense scenes. Metal uses
blue/purple and purple/red, with red/purple for intense scenes. RGB mixes remove
the shared white component so ordinary color transitions cannot accidentally
become white flashes. Atmospheric uses a slow blue/cyan tidal wash and no accents
or strobes. Its underlying dwell remains 48 seconds and fade eight seconds.

The controller exposes a small discrete palette and no demonstrated brightness
control. Smooth RGB transitions become color steps on it. The rendered geometry
and moving colors survive this conversion; pad brightness does not measure what
a dimmable fixture would look like. These are initial designs awaiting musical
judgment, not a claim of a finished stage aesthetic.

## Strobe treatment

`--bursts` or the `b` key enables the director's existing dense-kick policy. Each
qualified request now produces **three 80 ms white pulses**, starting 240 ms apart,
on successive mirrored pairs. Only two of eight fixtures flash white at a time;
the other six retain the base show. The renderer bounds the sequence to 720 ms,
within the director's 750 ms request and 12-second cooldown. No continuous or
whole-stage white strobe is generated. Atmospheric rejects requests independently.

Disabling bursts restores the base on all pads, including when the key arrives
while the loop waits for its next frame. Blackout overrides the whole result.
The existing queue expiry, nonblocking MIDI I/O, exit cleanup and input-loss
behavior remain in place. This is pad output; no DMX transport or fixture patch
has been added.

## Run and validation

```sh
target/release/shr-lux simulate metal --midi --headless --start 240 --seconds 55 --bursts
target/release/shr-lux simulate punk --midi
target/release/shr-lux simulate atmospheric --midi
# Opt-in full-song audit: files required, no MIDI writes.
cargo run --locked --release --example show-audition
```

Normal regression tests cover visible motion after device color conversion,
mirror geometry, palette changes without unrequested white, three bounded paired
flashes, preserved base color, kick accents without phase resets, motif dwell,
source influence, silence, input loss and atmospheric inhibition. The synthetic
WAV integration test now verifies actual white-pulse frames and burst permission.
The normal terminal suite verifies recovery on exit, EOF and SIGTERM.

Recorded validation: `sh scripts/check.sh` passed formatting, Clippy, all 41 Rust
production tests, six Python preparation tests, release build and eight PTY cases.
The opt-in full-song audition also passed:

| Source | Distinct device frames | Changed frames | Motifs visited | White pulses |
| --- | ---: | ---: | ---: | ---: |
| Punk | 106 | 555 | 4 | 0 |
| Metal | 124 | 1,522 | 4 | 3 |

A physical MIDI audition from metal 240 s to EOF (about 55 seconds) completed
successfully. The log records Mirror sweep, Crosscurrent and Guitar conversation,
with the qualified burst in the dense-kick passage. All-off cleanup was attempted
on normal exit. After a subsequent explicitly requested run, the user said the composed show
looked good. This is positive feedback on the pads, not validation of real fixtures. Log remains ignored at
`local/analysis/composed-metal-midi.log`.

These counts include the entire song at 25 Hz, including startup and silence.
The unchanged media-extraction audit, exhaustive algorithm comparisons and physical
DMX tests were intentionally skipped.

The full-song audition is deliberately opt-in because it needs private recordings.
It examines frames at the actual 25 Hz MIDI update rate, counts distinct device
patterns and motif visits, and rejects white outside qualified bursts or on more
than two fixtures at once. It is evidence of integrated behavior, not subjective
proof that the show looks good. Media preparation and onset thresholds are unchanged.

Related: [analysis and original hardware evidence](0018-analysis-and-pad-preview.md),
[show policy](0016-show-policy-validation.md), [validation](0012-development-and-validation.md).

[Notebook index](../reference/NOTEBOOK.md)

## Manual irregular-white audition

The user requested an irregular white-strobe demonstration. The first manual
sample used scripted alternating halves and mirrored pairs. After an explicitly
requested run, the user described it as a “soldier squad”: visibly too ordered.
That sample has been replaced by a seeded storm generator.

`target/release/shr-lux strobe-test` prepares a fresh eight-second timeline for
each run. Clusters have random sizes, gaps and pad masks. Individual pads within
a strike have independently jittered onset and 30–109 ms duration, so overlapping
flashes have ragged edges. Occasional broad strikes interrupt scattered flashes;
unequal dark pauses separate clusters. There is no mirrored progression, alternating
half pattern, or common beat grid. A fixed seed reproduces a timeline for tests;
the live command seeds it from the current time.

This is a manual visual sample, not driven by song analysis, and does not change
automatic burst policy. A fixed array bounds storage, and elapsed-time lookup
skips missed flashes after a stall. Output ends at eight seconds and existing
worker expiry and cleanup still apply.

The user explicitly requires that builds finish first and that physical auditions
wait for their subsequent “go.” Do not start a demonstration automatically after
compilation. The revised storm was then run on the user's explicit “go”; they confirmed it
matched the intended scattered storm effect.
Normal tests check bounded pulse lengths, visible asymmetry, variable durations,
dark rests, seed repeatability, different seeds, random-access playback and all-off
at the deadline. Full-song render audits are skipped because the music renderer
is unchanged. Its placement within a musical show remains an open audition question.
The [design study](0020-lighting-design-study.md) develops the next steps.
