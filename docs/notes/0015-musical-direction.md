# Musical understanding and automatic programs for punk and metal

Date: 2026-09-29

Status: research and proposed artistic behavior; a limited policy prototype exists.
Tags: #audio #show #research

**Historical scope:** this note records the initial research/prototype state. Audio
analysis and pad rendering were subsequently implemented; see the [implementation
record](0018-analysis-and-pad-preview.md) and [composed show](0019-composed-pad-show.md).
The [design study](0020-lighting-design-study.md) revisits artistic assumptions with
additional sources. Statements below about unimplemented renderer features describe
the earlier checkpoint.

## What the system needs to understand

It can make useful decisions with a few reliable observations. It does not need to
recognize a song or name every chord to hold a good wash through a verse or react to
an intense passage. Keep three different timescales:

| Layer | Useful information | Lighting purpose |
|---|---|---|
| Hits, tens of milliseconds | Kick onset time, strength, source and confidence | Brief accents and chase steps |
| Groove, roughly 0.5–4 seconds | Hit density, regularity, activity, energy trend, optional beat phase | Decide whether accents are appropriate |
| Passage, roughly 8–60 seconds | Sustained energy/activity change, optional timbral novelty | Choose and hold a scene family |

A kick onset is not necessarily a beat, a beat is not necessarily a downbeat, and a
regular kick run does not prove a blast beat. Tempo estimators can prefer half/double
rates; the FMP reference explicitly treats tempo harmonics and subharmonics.
[Tempo analysis](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C6/C6S2_TempogramAutocorrelation.html).
For the first version call the trigger **dense kick activity**. True blast-beat
classification would need more evidence, such as snare/cymbal rhythm.

## Features worth implementing first

All numeric windows below are proposals for testing with this band, not published
accuracy claims or selected capture settings.

1. **Calibrated source activity:** RMS/peak envelopes, clipping count and noise floor.
   Use different activity-on/off thresholds and a release hold so a guitar sustain
   does not flicker. Keep silence distinct from a missing or clipped input. Normalize
   over seconds, retaining headroom; instant auto-normalization erases musical dynamics.
2. **Isolated kick onsets:** filtered envelope increase against an adaptive baseline,
   with hysteresis and a short retrigger interval. Start around 25–40 ms retrigger
   exclusion and tune against ringing/bleed versus real rapid hits. A long exclusion
   loses double kick. Low-band energy alone may miss a click-heavy metal kick, so
   compare low/body and broadband attack cues on recordings.
3. **Density and persistence:** count accepted onsets over a bounded 1-second window,
   with a faster 250 ms view for entry. Retain intervals and strength variation.
   Ignore stale events, and reset after capture gaps. Density is events/s, not BPM.
4. **Energy/activity trend:** fast and slow envelopes (e.g. 0.5 and 4 seconds), and
   source activity masks. In compressed metal, level may stay high throughout;
   changes in density, timbre and active instruments may be more informative.
5. **Optional tempo:** compare an onset-envelope autocorrelation/phase tracker with
   aubio on local rehearsal excerpts. Maintain competing half/double tempo hypotheses
   and confidence. Let a tap/half/double correction help when available. Do not infer
   4/4, bar 1 or eight-bar phrases solely from periodic kicks.

Aubio exposes onset thresholds, silence rejection and minimum inter-onset intervals;
its beat tracker is causal. It is a useful baseline, not proof of performance on our
metal material. Its repository declares GPL terms; evaluate dependency licensing
before integrating it. No aubio dependency has been added.
[Aubio onset API](https://aubio.org/doc/0.4.7/onset_8h.html),
[causal beat tracker](https://aubio.org/manpages/latest/aubiotrack.1.html),
[upstream README](https://github.com/aubio/aubio/blob/master/README.md).

Later, test spectral-flux/timbral change for passage boundaries. Research commonly
uses novelty and self-similarity, but a whole-recording method may depend on future
audio. An online adaptation must specify its lookahead/delay and memory bound.
A boundary detector also does not establish semantic labels such as “chorus”.
[FMP novelty segmentation](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C4/C4S4_NoveltySegmentation.html).

Harmony from distorted polyphonic guitars, reliable solo identification, automatic
meter/downbeats and named sections are harder work. Preserve these aspirations;
show them as unavailable until validated. For an early performer-focus feature,
label “Guitar 1 active alone” honestly rather than claiming “solo detected”.

## How to make long scenes interesting

A long scene has a stable composition but may evolve gently. Proposed scene structure:

- Base: palette, front-light level, rear-light balance and spatial grouping.
- Slow motion: gradual color blend or asymmetric intensity breathing across 16–64 s.
- Accent layer: short kick emphasis or chase steps that do not replace the base.
- Transition: fade from current rendered values to the next composition.

Change only one or two visual properties at a time. Preserve enough front light to
see the band; let the rear wash carry most movement. Use a small curated palette per
program, avoid random RGB per hit, and avoid repeating the same composition every
transition. These are proposed artistic choices requiring venue rehearsal.

Hold a scene for a minimum duration. Require a candidate change to persist, with
separate enter/leave thresholds. A short fill should add an accent without restarting
the scene. Stable music can hold a look indefinitely; slow movement supplies variation.
If reliable bar phase is available later, take the next suitable boundary after dwell
expires, with a bounded wait. Until then use elapsed seconds and advertise bars as
unavailable. Unknown input holds the base and removes transient accents.

## Programs: choose the musical behavior before RUN

Programs should select artistic response, while venue patch and source calibration
stay separate. Genre labels are human choices, not automatic genre classification.
The same metal program must support a quiet introduction and a loud ending.

| Program | Proposed visual character | Implemented minimum scene | Implemented suggested fade | Burst policy |
|---|---|---:|---:|---|
| Punk | Direct contrasts, restrained palette, clear rhythmic rear accents | 16 s | 2 s | Optional dense-kick requests |
| Metal | Strong silhouettes, slow base composition, brief dense-rhythm accents | 24 s | 4 s | Optional dense-kick requests |
| Atmospheric / doom | Long evolving washes, gentle movement, sustained notes | 48 s | 8 s | Disabled |

Only dwell, fade suggestions and burst eligibility are implemented now. Colors,
fixture grouping, actual fades, movement and per-hit accents are future renderer work.
Punk and metal initially share detection thresholds; differ in composition timing.
Split into thrash/hardcore, groove/breakdown or doom variants only if rehearsals show
useful differences. Avoid a menu of arbitrary presets without listening evidence.

Future live controls: program, intensity, movement amount, scene hold, burst permission,
manual accent, blackout and tempo correction. Keep the starting workflow small:
assign sources → calibrate → select program → preview → arm a verified patch → RUN.
A program switch must clear candidate/transient state and must not bypass global
burst rest or blackout. Runtime switching/persistence are not implemented in this prototype.

## When to request “strobo”

Treat dense kicks as a candidate for a short accent, never as a command to flash once
for every hit. Proposed request conditions: fresh reliable input, sustained high energy,
sustained dense kick activity, appropriate program, permission, and available burst budget.
A breakdown with fewer heavy hits should use spacious impacts instead of being mistaken
for low musical intensity. A kick-only input cannot reliably make that distinction yet.

Current **text-only prototype** uses confidence ≥0.8, normalized energy ≥0.8 and density
≥8 hits/s sustained for 600 ms. It requests at most 750 ms, then waits at least 12 seconds
from the scheduled end before another request. Cancellation preserves that rest.
Continuous double kick therefore cannot stretch one request into an endless burst.
These values are rehearsal placeholders, not a validated detector or exposure limit.

Permission defaults off in the preview; `--bursts` enables printed requests only.
Atmospheric suppresses them. Missing/stale/invalid input, permission removal and caller
stalls cancel the request. There is no flash frequency, fixture strobe encoding or DMX
output in this implementation.

Before physical strobes: select a documented fixture mode, establish venue burst policy,
implement master/blackout priority, total exposure budgeting, and a bounded output
watchdog with tested shutter/dimmer recovery. The policy's 750 ms/12 s limits alone
cannot establish physical safety. A crashed process may leave a fixture running its
internal effect, so software tick cancellation is insufficient. Use a steady accent
when strobes are disabled or unsupported.

## Work sequence and difficulty

| Step | Relative effort | Completion evidence |
|---|---|---|
| Pure scene/burst policy and synthetic preview | Small; implemented | Normal regression suite and readable timeline |
| Level/activity + kick detector from replay PCM | Small–medium | Annotated hits including ringing, bleed, clipping and fast double kick |
| Two-input ALSA capture, freshness/recovery and Pi 1 packaging | Medium | Actual Pi measurements and disconnect/overrun recovery |
| Four inputs + feature fusion | Medium, hardware dependent | No memory growth or capture overruns on final USB arrangement |
| Scene renderer + known fixture transport | Medium; separate hardware gate | Patch validation, arming, blackout, bounded I/O, manual bench evidence |
| Beat phase + passage novelty | Medium–large research | Half/double tempo errors and false transitions measured on rehearsal data |
| Harmony, solos, meter, semantic sections | Large/uncertain | Separate labeled evaluation, likely optional on Pi 1 |

Record a small, private evaluation set covering punk straight rhythms, fills, metal
double kick, stop/start riffs, breakdowns, clean intros, sustained distortion, silence
and device loss. Keep audio outside Git. Label onset times, intended scene changes and
undesired bursts. Report onset precision/recall and latency, false burst requests/minute,
scene changes/minute, missed transitions and operator preference. Compare against a
static wash and simple per-hit accents. No research sweep belongs in normal tests.

Related: [Pi 1 feasibility](0014-pi1-feasibility.md),
[policy and validation](0016-show-policy-validation.md), [audio](0008-audio-and-musical-state.md).

[Notebook index](../reference/NOTEBOOK.md)
