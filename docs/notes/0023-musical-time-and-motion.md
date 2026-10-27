# Musical time, chases and storm texture

Date: 2026-09-29

Status: research synthesis and proposed directing behavior. Tags: #music #motion #strobe

## Separate what repeats from what develops

A show has several simultaneous timescales:

| Scale | Our usable evidence | Design question |
| --- | --- | --- |
| Transient | Kick candidate and strength | Does this hit deserve an accent? |
| Groove | Recent intervals, density and pulse regularity | How quickly can motion develop without fighting the feel? |
| Passage | Sustained energy/source-activity change | Should the composition open, contract, hold or release? |
| Song/set | Currently mostly unknown | What have we already shown, and what should remain in reserve? |

These are proposed responsibilities, not a claim that our analyzer identifies
phrases. It currently provides causal feature estimates after file-level
calibration. A regular double-kick stream can dominate its pulse estimate without
being the tactus a listener would tap. A half-time heavy passage can need slower,
larger gestures than a lighter passage with more onsets.

## Holds are active decisions

Our renderer advances motifs after 16/24 seconds even if the music is steady.
That produces variety but gives the clock authority over the performance. A better
proposal is: minimum dwell permits a change; it does not require one. Hold until
there is a convincing musical change, an operator request, or a deliberately
chosen slow development that preserves the same visual identity.

Sinclair's account of Queen Extravaganza describes revealing capabilities over
time. His American Utopia account describes transitions that preserve a longer
narrative. Our inference: remember recent visual choices and avoid spending every
available contrast at once.
[Queen interview](https://www.livedesignonline.com/business-people-news/rob-sinclair-lighting-queen-extravaganza-tour),
[American Utopia interview](https://www.livedesignonline.com/theatre/burning-down-house-american-utopia-broadway).

An initial “change budget” can simply track palette changes, large spatial changes
and accents separately. It is an artistic control to tune by audition, not a
medical exposure measure. Do not reduce everything to one scalar called intensity:
width, saturation, contrast, speed and light level can move independently.

## Understand the vocabulary of an effect engine

MA's phaser introduction separates the speed of a repeating sequence from the
phase positions of fixtures inside it. ETC's effects editor separately exposes
cycle time, grouping, trail, random group/rate, and entry/exit behavior. These are
independent design dimensions, not synonyms for speed.
[MA phasers](https://help.malighting.com/grandMA3/2.0/HTML/qsg_phasers.html),
[ETC effects editor](https://www.etcconnect.com/WebDocs/Controls/EosFamilyOnlineHelp/en/Content/14_Effects/01_Effects_Editor/About_the_Effects_Editor.htm).

**Our parameterization proposal:**

| Parameter | A useful distinction |
| --- | --- |
| Geometry | Linear order, mirrored pairs, center distance, performer region |
| Phase | Where each fixture is within the cycle |
| Envelope | Snap, decay, swell, triangle, smooth plateau |
| Width/duty | How much of the cycle is active |
| Overlap | Is there a trail, a gap, or a continuous pool? |
| Amplitude | How far the effect departs from the held base |
| Entry/exit | How it arrives and how the previous composition returns |
| Clock | Free-running, pulse-steered, or verified beat-synchronized |
| Variation | Fixed, seeded order variation, or clustered asynchronous events |

Start with one clear geometry and one moving property. Change its envelope before
adding another simultaneous effect. A slow soft chase and a hard narrow chase can
share the same order yet read very differently.

Our current `period` is a phase-step interval; the color waveform spans four phase
units. The complete cycle is therefore roughly four times that interval. Name and
log these quantities explicitly before calling an effect “one beat long.”

## Musical alignment without pretending to know the future

Novelty-based music segmentation uses changes in feature relationships to find
candidate boundaries. The FMP teaching example uses a self-similarity matrix and
checkerboard comparison around a time point. Directly transferring it to a live
system would require discussing future context, delay and bounded history.
[FMP novelty segmentation](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C4/C4S4_NoveltySegmentation.html).

**Proposal:** first add a small passage-change hypothesis from existing fast/slow
energy, activity masks and onset density. Require persistence and expose confidence.
Call it “texture changed,” not “chorus.” When rhythm is uncertain, fade freely or
wait for a strong observed event; do not claim a downbeat prediction. If a future
phase tracker is added, lock slowly, bound phase corrections and define unlock
behavior. Seeking and input gaps must reset its transient history.

A live causal system cannot reliably anticipate an unannounced drop. A prepared
recording, a known setlist, MIDI cue or operator can provide anticipation, but that
would be a different information source and must be shown as such.

## Storm is a texture, not a shuffled march

The user's accepted storm varies fixture selection, onset offsets, widths, overlap
and cluster gaps. The rejected version varied some times but retained obvious
ordered groups. This is project evidence of why changing order alone was inadequate.

Keep three forms distinct:

- **Chase:** legible progression. It benefits from a consistent spatial relationship.
- **Rhythmic burst:** repeated punctuation tied to a musical clock or selected hits.
- **Storm:** correlated clusters with asynchronous local flashes and unequal quiet intervals.

The current storm generator is a useful starting texture, not a physical lightning
model. It currently seeds from wall-clock time for manual auditions. Future
comparisons should accept and log a seed, so two policy variants see the same
texture. A seed changes the realization; it should not change overall bounds.

### Proposed music integration

Do not run the eight-second manual demo every time the director qualifies a burst.
Use a short, bounded storm envelope on an accent group. Dense kick persistence may
permit the envelope to start; individual kicks do not restart it. End or cancel it
on an absolute deadline, release smoothly to the still-running base, and preserve
the existing cooldown. Separate permission, density, spatial coverage and brightness.

Where the material calls for slower weight, use one held impact or a spatial opening
instead. This is a cue-selection problem; making the storm generator more random
cannot solve it.

## Comfort, permission and physical limits

Epilepsy Action identifies flash frequency, brightness, contrast and extent of the
visual field among relevant factors for photosensitive triggers. A random sequence
or a short burst is not automatically exempt.
[Photosensitive epilepsy guidance](https://www.epilepsy.org.uk/info/seizure-triggers/photosensitive-epilepsy).

W3C's flash criterion is written for web content and includes threshold/context
conditions. Its “three flashes” wording must not be converted into a universal
stage-lighting safety certificate.
[W3C explanation](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html).

For the project, keep pad permission separate from future physical-fixture arming,
fixture-specific recovery and venue decisions. The renderer should expose bounds
and allow an equivalent nonflashing accent. No safe fixture exposure has been
established here. This is distinct from the user's artistic request for occasional
aggressive texture, which the pad demo can help us develop.

Related: [study](0020-lighting-design-study.md), [design specification](0024-design-to-engine.md),
[audition plan](0025-design-audition-lab.md).

[Notebook index](../reference/NOTEBOOK.md)
