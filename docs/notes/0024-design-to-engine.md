# Turning design principles into a small engine

Date: 2026-09-29

Status: proposed implementation specification; no runtime changes in this study.
Tags: #architecture #design #pi1

## Design before extra detection

Our current DSP is already sufficient to test better composition. A larger model
would not automatically solve timer-driven scene changes, missing performer roles
or an unsuitable color mixer. Preserve the [current implementation](0019-composed-pad-show.md)
as a comparison baseline while changing one decision layer at a time.

### Code observations and proposed corrections

| Current behavior | Design consequence | Proposed correction |
| --- | --- | --- |
| `Preview` cycles motifs at each program dwell | Variety can arrive without musical motivation | Dwell makes a change eligible; sustained evidence or explicit intent selects it |
| All pads belong to one row | Geometry exists but coverage and depth do not | Small venue layout with roles, coordinates and performer association |
| `mix` removes the shared RGB component | Pale colors/white fail even identity blending | Color model with defined domains; saturation is an authored constraint |
| Base and accents exist, but intensity is embedded in RGB | Palette, level and flash can become entangled | Separate color, intensity, motion and accent intent |
| Kick pulse steers phase speed | Half/double ambiguity can change motion unexpectedly | Confidence, bounded steering and a free-running fallback; no beat claim yet |
| Main burst is regular paired flashes | Accepted storm texture remains outside music simulation | A selectable bounded accent texture, with policy owning permission and duration |
| Storm seed comes from wall time and is not logged | A liked realization is difficult to compare or repeat | Optional explicit seed and audition manifest |
| `show-audition` counts frames and looks | More activity can look like better performance in a report | Keep those as coverage checks; add human preference and musical-fit evaluation |

## Proposed logical model

```text
SourceFeatures
  timestamps, validity, activity, energy trends, kick events, pulse confidence
        |
PassageHypothesis
  stable / opening / thinning / sustained change / uncertain
        |
Director + ArtistProgram + RecentVisualHistory
  visual intention, reason, eligible changes, hold state, accent allowance
        |
SceneRenderer + VenueLayout
  performer layer + environment layer + optional accent layer
        |
Resolved fixture intent: color, intensity, transient expiry
        |
Capability adapter -> terminal / MiniLab / future validated fixture encoder
```

Keep the analyzer independent of artistic genre rules. Keep the renderer independent
of MIDI color codes and DMX channel maps. A venue layout maps logical positions to
actual fixtures once known; changing fixture count must not change the musical
meaning of a scene.

An initial layout can be a short fixed-capacity list: fixture ID, normalized x/y,
role flags, performer association and capabilities. A real four-fixture layout may
need a completely different composition from an eight-pad strip. Do not invent
missing front light by relabeling rear fixtures in software.

### A scene card should answer these questions

- What does the audience attend to?
- What remains stable for the duration?
- What is allowed to vary, and how much?
- Which sources influence those variables?
- What information justifies entry and exit?
- What happens when evidence becomes unreliable?
- What simpler version works when fixtures lack a capability?

**Illustrative authoring record, not a supported configuration schema:**

```yaml
name: contained_drive
intention: steady band picture with restrained rear motion
performer_layer: held
palette: cold_depth
motion:
  geometry: mirrored_pairs
  property: color_position
  clock: gently_pulse_steered
  amplitude: restrained
hold:
  change_requires: sustained_texture_change
accent:
  allowed: colored_pair
  storm: only_by_separate_qualified_request
fallback: held_environment
```

A UI should expose “hold,” movement amount, palette, performer balance and burst
permission before exposing dozens of internal thresholds. The artist should not
have to understand the detector implementation to direct the show.

## Proposed scene vocabulary

These are design studies to audition, not automatically recognized song labels.

| Scene | Stable identity | Development | Avoid |
| --- | --- | --- | --- |
| Open stage | Readable band, broad quiet environment | Slow breadth or color change | Turning every active instrument into motion |
| Contained drive | Compact spatial grouping, limited palette | Restrained continuous chase | Unrelated pattern rotation |
| Heavy space | Widely separated pools, long decay | Sparse large impacts | Treating sparse kicks as lack of intensity |
| Tension | Narrow picture with contrast in reserve | One parameter gradually grows | Raising speed, level and saturation together by default |
| Release | A deliberate opening of space or color | Hold the arrival long enough to read | Immediately spending the new look on another effect |
| Storm accent | Short local white texture over an owned base | Irregular bounded cluster | Running the standalone eight-second demo on every trigger |

Punk, metal and atmospheric programs would constrain this vocabulary and its
pacing. They do not determine one correct palette. “Heavy space” requires either
artist selection or stronger evidence than a low kick count; do not silently
pretend the present detector recognizes breakdowns.

## Minimum viable directing logic

**Proposal:** maintain one active intention and a small history of recent choices.
A candidate must be compatible with available fixture roles and current evidence.
When it becomes eligible, compare it to simply holding. Prefer a small adjustment
unless the evidence or operator calls for a stronger transformation. Do not use
random choice to conceal uncertainty.

Useful reason codes: `hold_stable`, `hold_uncertain`, `source_entry`, `sustained_rise`,
`sustained_thinning`, `manual_change`, `accent_allowed`, `accent_suppressed`.
Log a reason when a decision changes. This makes it possible to ask why a chase
started instead of tuning dozens of variables from memory.

A future history record might track the last few palettes, spatial families,
large contrasts and transient accents. It must not force novelty after a fixed
number of seconds. A long static look can remain the best choice.

## Implementation order and acceptance evidence

| Priority | Work | Effort estimate | Evidence before keeping it |
| --- | --- | --- | --- |
| 1 | Repeatable auditions: seed, exact passage, ready/go, decision log | Small | Same manifest reproduces the same intended frames; no output before go |
| 2 | Correct color representation and identity-preserving blends | Small–medium | Endpoint/identity tests, intentional neutral colors, inspected palette paths |
| 3 | Hold-first director and independent visual-change controls | Medium | Stable passage can hold; convincing changes respond; preferred in controlled A/B |
| 4 | Role-aware layout and six authored scene cards | Medium | No accidental loss of performer layer; graceful four/eight-fixture reduction |
| 5 | Bounded storm as an optional music accent | Small–medium | Permission, expiry, cooldown and base recovery; auditioned with music |
| 6 | Better passage evidence and rhythm phase | Medium–large | Annotated onset/boundary errors and false changes improve on baseline |
| Separate gate | ARMv6 build, capture and physical output | Hardware dependent | Actual Pi 1 measurements and known-fixture bench evidence |

Effort labels describe scope, not promised delivery dates. None of these proposals
require source separation or inference on a large neural network.

## What current generative research contributes

**Skip-BART (2025):** the paper learns lighting from livehouse video data covering
rock/punk/metal/core material. Its examined task is offline generation of one
principal hue and value stream, with saturation fixed. It processes the whole
audio track; that is not a drop-in live four-AUX controller. Its human study had
38 participants. The reported nonsignificant comparison with human lighting
(`p = 0.72`) does not by itself demonstrate statistical equivalence. Our takeaway
is to learn from authored examples and human judgments, not to copy its output
representation as a full stage model.
[Zhao et al., v1, §§3.1, 4.4 and discussion](https://arxiv.org/html/2506.01482v1).

**SeqLight (2026 preprint):** it separates music-to-color-distribution from allocating
that distribution among multiple lights. The authors explicitly describe a simplified
eight-point-light setup, omitted directionality and offline whole-sequence operation.
We can borrow the separation of artistic intent from venue allocation. Neither
its training hardware nor its reported results establish suitability for a Pi 1,
real fixture spectra or audience visibility.
[Zhao et al., v1, introduction and Appendix H](https://arxiv.org/html/2605.03660v1).

Both are research leads. No models, datasets or dependencies were downloaded, and
no claims of reproducing their results are made. Video-derived color labels also
need a critical reading: a camera view is not a direct record of DMX commands or
what every audience member saw.

## Pi 1 cost model

Use bounded histories and per-fixture arithmetic. As an illustrative storage budget,
eight precomputed paths of 256 RGB float samples require 24 KiB before metadata.
A small scene/history model can be evaluated at a lower rate than transient output.
These are arithmetic estimates, not target benchmarks. Keep sample-time decisions
separate from bounded I/O and discard late transient output.

The existing Pi 5 analysis measurement cannot certify ARMv6 performance. Measure
CPU, resident memory, capture overruns, update latency and shutdown on the actual
512 MB system before declaring the deployment target achieved.

Related: [study](0020-lighting-design-study.md), [color](0022-color-and-fixtures.md),
[timing](0023-musical-time-and-motion.md), [auditions](0025-design-audition-lab.md).

[Notebook index](../reference/NOTEBOOK.md)
