# Configured synthetic authority and named analysis

Implemented software integration, with physical output still null/disarmed and
unknown. [LX05 baseline](0032-analysis-subscription.md) and
[private restart](0030-release-recovery.md) retain their legacy semantics.

`lux-service --synthetic-private-dir ABS_PRIVATE_DIRECTORY --config ABS_JSON`
loads an explicit startup identity. The version1 JSON has `show_id` (canonical
UUID), `patch` (the existing complete `PatchSpec` with canonical decimal-string
`patch_revision`) and optional `analysis`. Only existing synthetic personalities
are admitted. Config input is a regular nonsymlink file, at most65536bytes;
duplicate keys, unknown fields, nested malformed patch data and excess depth
refuse before persistence or socket creation. The existing fixture and wire
resource limits still apply. No physical personality, discovery or output driver
is introduced.

The optional analysis binding is exactly
`{"version":2,"inputs":["input-17","input-03","input-09","input-01"]}`.
The four distinct canonical logical IDs identify the ordered semantic sources
kick, bass, guitar-1 and guitar-2, independently of physical input slots. Their
unsigned16bit identity domain is not a qualified channel-count claim. A binding
requires an explicit `--analysis ABS_PRIVATE_PROVIDER_SOCKET`. Without a binding,
`--analysis` retains its legacy v1 meaning, even with a configured show/patch.
`--timed` and `--durable` retain their existing independent meanings; durable
includes timed behavior.

## Consumer contract

Configured analysis sends only
`{"subscription":"lux.aux.v2","version":2}` and validates every descriptor on
every connection against the exact ordered configured IDs. It admits only
C-ANALYSIS:2/lux.aux.v2, retaining all existing stream3/48kHz/raw-pre-fader,
canonical-counter, frame-origin, same-host acquisition-age, loss, sequence and
calibration checks. Wrong mapping, version or order never downgrades to v1.
`Descriptor::decode` and `Client::start` remain strict legacy entrypoints; the
configured entrypoints require an independently validated `Binding`.

Configured service inventory/capabilities always declare `wire_schema:lx05-v2`,
including when the source is absent. Inventory adds `analysis_binding`, and the
inner `analysis` status adds the same `analysis_binding`. A standalone
`analysis_status` reply retains body shape `{wire_schema,analysis}` with that
binding inside `analysis`. The binding has exactly `version` and `inputs`.
All previous LX05 command shapes remain unchanged. The estimator identity remains
`lux.analysis.v1`; this change adds no musical estimator or DSP.

Consumers must validate both the current source descriptor and each retained
AUTO contribution's `provenance.source_identity` against the explicit binding,
even with an absent source. Retained contributions may legitimately name an older
epoch/map than the current live descriptor; equality to current epoch/map is not
required. Reattachment invalidates calibration and grants, and missing samples
never become synthetic silence. Human control/Holds remain independent.

The owner-produced [configured corpus](../../tests/fixtures/lx05/v2/README.md)
covers cold absent configuration, active analysis, retained provenance after
source loss and manual control. The producer descriptor is copied unchanged from
GigPies configured analysis commit527e320e778155597be4668344d0b4fde7e2550b.
Actual physical acquisition timestamps are not established by this software path;
GigPies rejects configured publication with its physical provider option.

## Private startup identity and recovery

Use a fresh owned0700 canonical private directory for first configured startup.
Under the existing exclusive owner lock, Lux persists `lighting-startup.json`
with the complete normalized startup identity (show, patch and optional analysis
binding). Existing legacy storage cannot be adopted implicitly. Legacy startup
cannot open configured storage. Later configured starts must match exactly;
changes require a separately chosen fresh directory, not overwriting saved state.

The identity and any checkpoint patch are checked before consuming a new epoch or
modifying checkpoint/socket state. Invalid/mismatched files are preserved. A
missing epoch registry after previous ownership fails closed, including a crash
between initial identity and epoch writes; use fresh storage after review rather
than guessing a never-used epoch. Failed first startup may retain an owner lock
file, as existing recovery policy already does.

Durable recovery uses the configured show and patch, keeps intended values/Holds,
and resumes neither AUTO grants nor active transients. A runtime replacement patch
can still be reviewed/applied through existing commands, but checkpointing a patch
that differs from the startup identity refuses; select a reviewed new startup
configuration and fresh private directory for a different persistent patch.

## Reproduction and limits

Normal gates are the repository's pinned-toolchain default tests, warning-denied
all-target Clippy, formatting, release build, terminal recovery and fast simulation
checks. `cargo test --locked --test configured_service` covers startup admission,
identity preservation, recovery and real private socket reconnection. The normal
`configured_owner_corpus_active_absent_retained_and_manual` unit test checks exact
owner output against the accepted fixture. Explicit fixture refresh is
`LUX_GENERATE_LX05_V2_CORPUS=1 cargo test --locked --lib configured_owner_corpus_active_absent_retained_and_manual`;
review the diff before accepting any change. No automatic fixture rewrites occur.
Hardware, audible playback, MIDI/DMX output and deadline qualification remain
separate; no service starts automatically on boot.

## Recorded software validation — 2026-10-08

The complete normal all-target suite passed114tests, including eight configured
startup/recovery/socket regressions and the exact v2 owner corpus. Warning-denied
all-target Clippy, formatting, release, six fast simulation checks, the legacy
E04 corpus and eight Linux PTY checks passed. The actual legacy null-service
checkpoint check passed three unique epochs with disarmed recovery.

An actual configured GigPies/Lux private process episode passed with seventeen
logical inputs, reversed capture slots, explicit [17,03,09,01] selection, a custom
show and two fixtures. It exercised explicit calibration, fresh ready analysis,
a scoped AUTO grant, source-child loss with retained provenance, and independent
human intensity900. Processes were bounded and joined; no physical endpoint was
opened. Exact executable hashes and private command/evidence logs are retained in
the task handoff, not treated as hardware qualification.

## Optional musical estimator

Add `--musical-analysis` alongside `--analysis ABS_PRIVATE_PROVIDER_SOCKET` to
select the additive `lx05-music-v1` (legacy input binding) or `lx05-music-v2`
(configured binding) schema and `lux.analysis.v2` estimator. Without the flag,
legacy LX05 output and exact corpora are unchanged. This is a startup choice;
configuration identity, provider subscription and raw source timing are unchanged.

```sh
lux-service --synthetic-private-dir ABS_PRIVATE_DIRECTORY --analysis ABS_PRIVATE_PROVIDER_SOCKET --musical-analysis
lux-service --synthetic-private-dir ABS_PRIVATE_DIRECTORY --config ABS_JSON --analysis ABS_PRIVATE_PROVIDER_SOCKET --musical-analysis
```

The new `analysis.musical` object contains state (`unknown`, `tracking`, `stale`),
nullable decimal-string exclusive-end `source_frame`, twelve integer chroma bins
(C through B), and nullable chord, key, movement and section estimates. Normalized
values/confidences use 0..1000. Chord has root (C=0), quality (major/minor), confidence.
Key has tonic, mode, Camelot number (1..12), letter (A minor/B major), confidence.
Movement has signed fifths_steps (-6..6), kind (stationary/fifths_forward/
fifths_backward/other), tension and resolution. Section has id, novelty, recurrence,
label (unknown/verse_candidate/chorus_candidate) and confidence. Candidate section
names are acoustic hypotheses, not known lyrics or formal song annotations.

Tracking requires both calibrated source readiness and musical readiness; its frame
names the last completed spectral hop's exclusive source-frame end, at most
6143 frames behind the current source window end. It does not advance between
spectral hops. Every invalidation, source
loss, reattachment and fresh calibration resets musical history. Unknown/stale
output clears estimates, frame and chroma. Beat/downbeat/harmony legacy slots remain
null; music lives exclusively in the additive object. AUTO provenance uses the
advertised estimator version. Harmonic tension/resolution can nudge the existing
intensity proposal by at most 50/1000; existing grant caps, steps, expiry, Hold,
manual priority and loss freeze still govern application. No output backend or
arming authority changes. Algorithm and limitations: [musical analysis](0034-musical-analysis.md).

The spectral window spans about 256ms, with a 128ms hop. The reported frame is
analysis evidence time, not capture or sound-card synchronization qualification.
Normal protocol/reset coverage is `cargo test --locked --lib
musical_opt_in_schema_source_frames_and_stale_reset`. Explicit owner fixture
refresh is `LUX_GENERATE_MUSICAL_CORPUS=1 cargo test --locked --lib
musical_opt_in_schema_source_frames_and_stale_reset`; review the JSON changes.
Fixture generation is opt-in and no private PCM is written.

The longer synthetic owner estimate corpus is an opt-in evidence renderer:
`cargo test --locked --lib musical_owner_estimate_corpus -- --ignored`. It retains
real owner-encoded active, stale and unknown snapshots under `tests/fixtures/musical/`;
its progression includes C/F/G chords and the final active snapshot contains all
four estimates. Section estimates are informational; only key-supported harmonic
movement adjusts AUTO intensity in this implementation.

The owner drains at most two queued PCM windows into local ownership under the
transport mutex, then releases that mutex before DSP. The receiver retains its
two-window queue and the 100ms acquisition-age limit. Loss, disconnect or a new
source descriptor arriving during DSP invalidates the analyzed batch before AUTO
can use it; pending transport events are handled in order on the next drain. This
reduces transport lock contention; it does not establish the cause of any observed
process reconnection or qualify realtime throughput.

## Variable-feed binding

C-ANALYSIS:3 adds configured 4–8-feed local attachments. Lux startup `analysis`
uses `version:3` with matching ordered `inputs` and `sources`; seven sources can
be `kick`, `bass`, `guitar-1`, `guitar-2`, `vocal`, `synth`, `backs`. Every feed
must be observed during fresh calibration. The consumer checks exact names,
input IDs, source epoch, revisions and channel-dependent packet/window lengths.
The first four roles preserve the established director; extra feeds contribute
to musical context and have individual RMS observations. ADC clipping on an
extra feed also inhibits readiness. No legacy endpoint silently changes shape.

See the [producer mapping and packet budget](../../../gigpies/docs/reference/CONFIGURED_ANALYSIS.md#seven-feed-configuration-c-analysis3)
and [rich music contract](0034-musical-analysis.md#extended-musical-context-and-configured-feeds).
The budget and synthetic software validation do not qualify physical capture or
lighting. A selected source remains a named input, not an inferred isolated stem.
