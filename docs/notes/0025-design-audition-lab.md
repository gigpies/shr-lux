# A laboratory for learning lighting taste

Date: 2026-09-29

Status: proposed study exercises and reusable observation template. Tags: #learning #evaluation

## What counts as evidence

A unit test can prove that a burst expires, that a mirror is symmetric or that
output is reproducible. It cannot prove that a show is moving, tasteful or appropriate
for the artist. A high count of distinct frames is coverage evidence, not an
artistic score. Keep those two kinds of evaluation separate.

Our observations so far:

| Experiment | User feedback | Supported conclusion | Not established |
| --- | --- | --- | --- |
| Source LEDs and white marker | Mostly static; a short blink was not a burst | Diagnostic indicators did not meet the desired experience | Detector accuracy |
| Composed eight-pad metal show | “looks good” | Direction worth preserving for further study | Real-stage composition or audience comfort |
| Scripted irregular-white sample | “soldier squad” | Obvious group order defeated the intended storm character | That every regular chase is undesirable |
| Seeded scattered storm | “yes” | This texture better matched the requested effect | Where it belongs in music, or physical strobe limits |

This is one user's feedback on a MiniLab, not a population study. It is still more
relevant to this project's artistic direction than an unexamined generic preset.

## The audition contract

1. Complete implementation and normal checks first.
2. Prepare a specific duration, source passage, variant and expected observation.
3. Tell the user it is ready. Wait for their **go**.
4. Run that audition only. End and clear output.
5. Record what they saw, separately from what the software logged.

Do not build, start output later and assume the user stayed watching. A “go” for
one demo is not authorization to surprise them with the next variant. Read-only
analysis and document work can continue independently.

The current `--listen` path lacks a working default playback destination on this
host and is not sample-locked to analysis. Before judging synchronization or cue
placement, establish a usable listening setup and record its offset. A silent pad
run can judge texture/order, but not whether an accent lands musically.

## An eight-session study plan

These are exercises to schedule, not demonstrations launched by this document.

| Session | Exercise | What to learn | Evidence to save |
| --- | --- | --- | --- |
| 1. Listening | Listen without lights; mark exposed entries, sustained passages, rises, thinning, silence | Distinguish observed events from guessed section names | Time-stamped annotations with uncertainty |
| 2. One hue | Produce three still compositions using one hue and distribution/level changes | Separate composition from palette novelty | Sketches and audience-focus descriptions |
| 3. Context | Place the same color beside different surrounds, then compare on a real surface later | Color is experienced in context | Display/fixture conditions and observations |
| 4. Long hold | Compare a stable look with fixed-period motif rotation | Whether changes are musically needed | A/B preference and needless-change timestamps |
| 5. Chase anatomy | Hold order and palette; vary envelope, width and overlap separately | Why two chases with the same speed feel different | One changed variable per trial |
| 6. Storm | Compare regular pairs and seeded storm at matched musical locations | Texture versus timing, and the value of pauses | Seed, requested pulses and visual feedback |
| 7. Passage development | Design a quiet entry, sustained middle and return using a limited palette | Preserve a contrast for an arrival | A cue sketch with reasons and what stays unchanged |
| 8. Reduction | Reduce an eight-fixture concept to four available roles | Preserve intention when capabilities shrink | Accepted omissions and coverage limitations |

The context exercise is consistent with the kind of color studies described in
Yale's catalogue for Albers's *Interaction of Color*. The book itself has not been
read in this pass; it is a recommended next study resource, not evidence for a
completed exercise.
[Publisher description](https://yalebooks.yale.edu/book/9780300179354/interaction-of-color/).

## First three controlled comparisons

### A. Holding versus rotating

Choose a 45–60 second sustained passage. Keep program palette, source calibration,
base level and accents constant. Variant A preserves one composition; B uses the
current timed motif cycle. Reverse presentation order on a later session. Ask
which feels intentional and where either becomes monotonous or distracting.

Do not force a winner. A held look may need subtle internal development; a rotating
look may happen to align with this one passage. Retest on another source.

### B. Color routes

Compare direct RGB-byte interpolation with an authored perceptual path using the
same endpoints. Inspect the middle of the transition and any gamut adjustment.
Include a pale neutral endpoint so the current white-stripping limitation cannot
hide behind saturated examples. First use a display study; MiniLab quantization
cannot validate the continuous route.

### C. Storm in context

Use the already analyzed metal passage around 258–264 seconds to find the known
candidate burst, but include enough lead-in and recovery to judge contrast.
Compare: no transient, regular paired burst, bounded storm. Keep the same underlying
scene. Do not insert the full eight-second standalone sample as a substitute for
an integrated short accent.

The recorded time identifies a **detector event**, not a human-verified ideal cue.
A reviewer may prefer no burst there. That result should be allowed to change the
policy rather than being dismissed as a failed demonstration.

## What to ask and measure

Ask for a short reaction before displaying technical logs:

- Did the light support the musical passage or distract from it?
- Where did a change feel earned, late, early or unnecessary?
- Was there a readable focal area?
- Did the palette feel coherent?
- Did the quiet periods make the accents stronger?
- Was it comfortable to keep watching?

Then inspect logs: scene changes and reasons, large palette/spatial changes,
accent requests/suppression, output expiry and missing input. Do not combine these
into a beauty score. For a small set of personal trials, report preferences and
specific comments rather than claiming statistical significance.

For detector evaluation separately, label actual kick onsets and count false/missed
hits with a stated timing tolerance. For passage evidence, label acceptable change
regions rather than pretending every boundary has one exact millisecond. Keep a
few excerpts out of tuning to expose overfitting.

## Reference viewing without cargo-cult copying

Use the [firsthand case studies](0026-lighting-study-sources.md) as a viewing syllabus:
American Utopia for continuity and staged revelation; Mitski for visibility,
sculptural space and return to stillness; Metallica for performer presence inside
a large energetic picture. Interview pages include photographs or linked media.
No concert film was watched or shot-by-shot analyzed during this research pass.

When viewing later, log the camera shot: a cut or exposure change can mimic a
lighting change. Prefer a stable wide view for spatial judgments. Do not copy a
specific cue merely because it looks impressive in a close-up or a heavily edited
film. Translate its intention into the actual small rig.

## Record and preserve

Use [the audition template](../research/audition-template.md). Save private audio,
full traces and captures under ignored `local/` or `recordings/`. Keep concise
conclusions in linked notes. A rejected version is useful evidence when it explains
why the accepted one works; it need not remain in the normal test suite forever.

Default tests should protect current behavior. Full-song auditions, parameter
sweeps, render studies and performance runs remain opt-in. Documentation-only
research does not justify rerunning hardware or rebuilding the application.

Related: [study map](0020-lighting-design-study.md), [implementation](0024-design-to-engine.md).

[Notebook index](../reference/NOTEBOOK.md)
