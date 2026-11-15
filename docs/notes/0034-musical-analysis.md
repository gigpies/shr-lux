# Bounded harmonic and arrangement estimates

The optional musical analyzer consumes normalized, finite 480-frame blocks at
48 kHz. The original kick/bass/guitar1/guitar2 ordering remains supported;
configured subscriptions can add vocal, synth and backing feeds as described below.
Tonal feeds contribute to harmony; kick remains in the transient/activity analysis. No hardware is
opened by this module. This is implemented software analysis, with synthetic
validation; live accuracy, Pi CPU budget and acoustic acceptance remain unverified.

`MusicalAnalyzer::new`, `reset`, `snapshot` and `process` provide a copyable
snapshot. A source timeline gap/backward jump, malformed block, non-finite or
out-of-range sample resets all estimates. More than two frames in a block with
any selected feed at absolute amplitude >=0.999 also reset the analyzer,
matching the base source reliability gate. This prevents ADC-clipped harmonics
from contaminating retained key/arrangement history. Two isolated saturated
frames and signals below the threshold remain accepted; distortion below ADC
saturation is not classified by this gate. `source_frame` is the exclusive end
of the latest spectral window, rather than the latest consumed block: while
ready it lags the source block end by strictly fewer than 6144 source frames.
No complete spectrum means `ready=false`. Silence can be ready with no estimates.
Reset preserves precomputed coefficients.

A fixed 96-tap Hann-windowed sinc filter with 2.5 kHz cutoff precedes sixfold
48-to-8 kHz decimation. A 2048-point Hann FFT runs every 1024 decimated frames
(256 ms window, 128 ms hop). Maximum note-bin power is folded into twelve pitch
classes over approximately 63–2100 Hz. A low E1 bass fundamental near 41 Hz is
outside this band; its upper partials can contribute, but this does not provide
full bass-register pitch tracking. Sources are normalized separately and
combined as powers, preventing opposite-phase guitars from cancelling; bass has
weight 0.35 and each guitar 1.0. All buffers have fixed bounds; `process` has no
allocation, lock, I/O or trigonometry. Sample validation remains bounded by the
480-frame input contract. FFT window delay and FIR delay are not compensated.

Major/minor triad cosine templates require score, runner-up margin and all three
tones' coverage, then two consecutive agreeing hops. Power chords and ambiguous
spectra remain unknown. This does not identify seventh/suspended chords,
enharmonic spelling or arbitrary polyphonic harmony. Distorted overtones, feed
bleed, unequal notes, tuning and noise can reduce confidence or mislead templates.
A present estimate's confidence is heuristic support, not calibrated probability.

Key uses a rolling 96-hop chroma history (12.288 seconds), requiring at least 48
active hops (6.144 seconds) and three confirmed chord roots in that same history.
Weighted major/minor scale profiles need a winner margin; relative-mode ambiguity
can remain unknown. A sustained triad cannot establish song key, and expired
root diversity cannot keep a key alive. Silence clears current harmonic estimates and
partial arrangement descriptors immediately; after sixteen quiet hops it also
clears retained arrangement/key context. There is no universal key accuracy claim.

Camelot numbers use integer circle-of-fifths arithmetic: C major is 8B, A minor
8A, G major 9B and E minor 9A. Harmonic movement uses the shortest signed fifths
displacement, with antipodes assigned +6. A confirmed change can publish movement
for up to eight hops (1.024 seconds); up to two ambiguous overlap hops preserve the prior confirmed root for a
transition, with no movement published during those ambiguous hops. Longer
ambiguity or silence breaks continuity. A confirmation hop can have movement but no current chord. Tension
and dominant-to-tonic resolution need independently supported current key context.
Retained movement clears when supported key identity changes.

Arrangement descriptors aggregate sixteen hops (2.048 seconds), combining chroma,
source balance and energy. Sixteen fixed prototypes track local recurring IDs.
A change requires two similar novel descriptors and at least two descriptors of
dwell. Harmonic difference alone cannot meet the novelty threshold. Repeated
contrasting arrangements can receive `VerseCandidate`/`ChorusCandidate` only
when both prototypes recur and their energy differs by at least 1.7x. These are
local arrangement candidates; the analyzer has no lyric, vocal or formal song
structure model. IDs start at one, reset with context, and unknown labels carry
zero confidence. Coarse block boundaries limit transition timing.

Focused default regressions use synthetic triads, opposite-phase sources,
fifth transitions/expiry, power-chord ambiguity, silence, invalid PCM, timeline
gaps, broadband noise, tonic-weighted progressions with rolling key expiry,
minor-key progressions with a major dominant, relative-mode ambiguity, repeated
arrangement contrast, transition overlap/noise gaps, reset and an allocation
counter around post-construction processing. They are device-free correctness tests,
not performance qualification or a representative recorded-music accuracy study.
Run `CARGO_INCREMENTAL=0 cargo test --locked -j1 --test musical`.

Primary mathematical references (concepts only; no copied implementation):

- [AudioLabs FMP template chord recognition](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C5/C5S2_ChordRec_Templates.html): chroma and template similarity.
- [AudioLabs FMP structure analysis](https://www.audiolabs-erlangen.de/resources/MIR/FMP/C4/C4.html): novelty, similarity and recurrence.
- [Mixed In Key harmonic mixing](https://mixedinkey.com/harmonic-mixing-guide/): Camelot labels and neighboring relationships.

See [analysis subscription](0032-analysis-subscription.md),
[configured analysis](0033-configured-analysis.md), and the
[owning integration plan](0027-gigpies-implementation.md).

## Extended musical context and configured feeds

`SourceMusicalAnalyzer<N>::with_weights` admits 1–16 sources with finite harmonic
weights in 0..=1. Zero excludes a source from spectral analysis while retaining it
in fast ensemble activity. `MusicalAnalyzer` remains the compatible four-source
alias. The local C-ANALYSIS:3 adapter admits 4–8 named feeds, including vocal,
synth and backs after the original four roles. Its eight-feed bound comes from
the existing 1200-byte packet budget; it is not a product input ceiling. Selected
extra feeds receive harmonic weight 1.0; kick remains excluded and bass retains
0.35. Silence on unused storage slots does no FIR/FFT work. Constructor/control
storage is heap-owned by the service to keep transaction clones off the stack;
processing/reset still allocate nothing.

Fast expression uses every selected source, independently of harmonic confidence.
Maximum source RMS avoids phase cancellation and prevents one playing voice being
mistaken for an ensemble pause. A slowly decaying recent level establishes a 2%
relative quiet threshold with a 0.0001 absolute floor. After a 120 ms initial
settling period, 120 ms quiet is a brief gap, 350 ms a pause and 3 s silence.
Resumption is held for 100 ms. These thresholds are detection heuristics: bleed,
noise and long effect tails still need representative rehearsal evaluation.
Missing/stale/malformed audio invalidates estimates, never fabricates a pause.

Energy direction compares combined source-power RMS with roughly 250 ms and 2 s envelopes with 25%/20%
direction thresholds. Register direction compares independent-source spectral pitch-power
centroids before octave folding, requiring two agreeing spectral hops and a
0.75-semitone deadband. A source needs at least 12% of pitch power in its strongest pitch bin; noise or absent tonal support yields unknown register direction. Register is explicitly **not a monophonic melody estimate**:
voicing, changing instruments and harmonics can move it. It is independent of
energy direction and signed circle-of-fifths chord-root motion.

A fixed 24-by-24 table learns confirmed major/minor chord transitions. Prediction
requires at least three outgoing observations and 75% agreement; counts halve
before saturation. No training songs, downloads, new dependency or unbounded
history are required. The next-chord candidate has no predicted onset time and
its support is an observed fraction, not calibrated probability. Ambiguity hides
predictions; source discontinuity and prolonged quiet erase learned context.

A novel arrangement after at least two recurring other arrangements, sustained
for two descriptor observations, can receive `BridgeCandidate` at support 0.35.
This can also describe a solo, breakdown or outro. Formal bridge/chorus/verse
recognition remains tentative; lyrics and a trained structural model are absent.

The rich wire generations are `lx05-music-v3/v4` and `lx06-scenes-v3/v4`
(legacy/configured mapping respectively). Musical metadata adds nullable
`expression` and `prediction`; unavailable states clear both. Expression's
canonical decimal `source_frame` is the latest consumed 10 ms block end, while
musical `source_frame` remains the latest 128 ms spectral-hop end. `lx05-v3`
identifies configured variable-feed operation without the optional music layer.
RMS array length matches the configured source count. Old corpus schemas remain
readable in the updated Lightdesk. New schemas intentionally require an updated
consumer; there is no silent downgrade.

No extra audio delay or relaxed 100 ms acquisition-age budget is introduced.
The 256 ms spectral window and multi-second key/section confirmation are analysis
history, not a claim of usable acoustic lookahead. Venue audience distance and
fixture latency must be measured before scheduling lights against acoustic arrival.
The existing four-role intensity director and all manual/Hold/grant rules remain
unchanged; new cues are read-only musical context for the desk.

Focused checks: `cargo test --locked --test musical --test expression`.
Bounded offline cost (explicit opt-in):
`cargo test --locked --release --test expression seven_feed_worker_cost -- --ignored --nocapture`.
Shared acceptance and measured results belong to
[GP-MUSIC-2](../../../gigpies/docs/development/MODULE_IMPLEMENTATION_PLAN.md#gp-music-2--seven-feed-musical-context-for-lightdesk).
