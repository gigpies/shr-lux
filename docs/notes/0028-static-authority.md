# Null-output static lighting authority

Date: 2026-10-04. Task 0008 / LX-02. **Implemented; offline validated; final handoff pending coordinator
acceptance**. This engine is separate from the recorded-audio simulation, terminal
preview and MIDI pad output. It neither imports a surface simulator nor opens
hardware. [Integration plan](0027-gigpies-implementation.md) and
[C-LIGHT:1](../../../gigpies/docs/reference/MODULE_CONTRACTS.md) own the integration scope.

## Manual use without analysis or MIDI

`authority::Authority` owns one validated `fixture::Patch`, show UUID and epoch.
`execute(Command)` returns a complete typed snapshot. The caller may read the
patch/snapshot but cannot mutate internal source masks or arbitration order.
A new engine starts with fixture defaults, global master 1000, no blackout and
Manual mode. `with_stores` validates optional volatile initial cue/palette copies;
it is not durable restart loading. Applications can supply authored synthetic
looks without first creating a Hold that would require timed release.

Commands implement the static manual workflow:

- `Touch` accepts at most 64 values in a transaction, requiring whole RGB and
  pan/tilt groups. Zero is an intentional programmer value.
- `Record` creates a cue or palette copy from programmer masks only. `Update`
  merges programmer masks into an existing stored copy. Hold is not recordable.
- `Go` copies a cue into a playback and gives it an engine activation order.
  Updating the stored cue leaves the running copy unchanged until another GO.
  `ApplyPalette` copies a palette into programmer; there are no live references.
- `ClearToHold` copies programmer values to Hold and clears programmer masks,
  preserving the pre-master look. Mode changes to Manual/Assist freeze the
  resolved pre-master look into Hold without removing existing manual ownership.
- `Level` scales playback intensity. Level zero retains color and position;
  `Off` removes the playback. All tied HTP intensities are winners. Non-intensity
  coherent groups use latest engine GO activation, independent of text IDs.
- `Master` and `FixtureMaster` accept 0..1000; final intensity is the product
  of resolved intensity and both masters, rounded once to nearest integer with
  halves up. `Blackout` inhibits intensity while preserving color and position.

Every command applies to a private candidate state, validates it, then swaps the
whole state and increments revision. Refusal retains masks, stored/running copies,
activation order, patch and state revisions. Replacement patches must advance
patch revision and validate every retained mask against the new capabilities;
orphaned or out-of-range stored state is refused rather than silently discarded.

Cue and palette namespaces each hold at most 32 IDs, including distinct objects
with identical ID text. At most eight playback IDs may be active. Accumulated
masks may cover the full validated patch; the 64-target limit applies to an edit,
not to the total state accumulated over several edits. Coherent group masks stay
complete in programmer, Hold, saved objects and copied running playbacks.

## Provenance and output limits

Snapshots report every applied contributor even when hidden by programmer or
Hold. Fixture default is the fallback only. Priority is programmer > Hold >
ordinary playback > fixture default. Applied AUTO is unavailable until LX-05
provides a validated active vocabulary; an ASSIST proposal does not own output.
The static engine currently produces no analysis proposals.

`NullSink` retains a local copy of engine intent for inspection. It is not a
submission acknowledgement. Snapshots stay `volatile`, `null_disarmed`, submitted
and observed values `None`, and physical state `unknown`. No physical map,
encoder, DMX worker, controller feedback or restart-safe persistence is provided.

Engine-created release previews expose current/destination values for held
intensity targets and bind show/epoch/patch/state revision. They declare the E05
500 ms transition and 2000 ms validity interval, but this static milestone has
no authority clock/expiry or timed release. `release_commit` returns Unavailable
without changing state. LX-04 must implement and review timing before any release
is enabled. Non-intensity release/movement paths remain unavailable.

## Validation scope

The fast production authority suite executes E04 commands from named prior
states and compares masks, provenance, resolved/final intent and retained
color/position. Additional regressions exercise transaction/patch atomicity,
HTP ties and winner removal, coherent activation, GO retrigger, copied update
isolation, palette masks, mode/Hold continuity, masters/blackout, independent
capacities and preview invalidation. Normal all-target tests (61 passed), Clippy, release and all eight
Linux PTY checks passed. Focused tests (19), six preparation contracts and corpus
checks also passed. Measured results are recorded in the
[owning plan](0027-gigpies-implementation.md).

Historical auditions, exhaustive research matrices, long benchmarks and physical
fixture/load tests are outside this offline change. Hardware and persistence
claims retain their original limits.

[Notebook index](../reference/NOTEBOOK.md)
