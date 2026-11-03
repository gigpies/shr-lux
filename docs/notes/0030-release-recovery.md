# Timed logical release and private restart

Task0008 LX04A source accepted and25focused/78normal passed. LX04B implemented
and pass3 offline validation completed:89normal, fmt, Clippy, release,8PTY and
realCLI E05/two-crash restart demonstration passed. Final root artifact review
remains independent. [CLI integration evidence and usage](0031-local-provider-integration.md).

LX03 accepted corpus bytes remain unchanged. `Service::new` is the accepted static
wire capability. `Service::timed` advertises `wire_schema:lx04-v1` and `release`
capability with500ms transition/2000ms preview validity. Contract C-LIGHT:1 remains;
this is a new explicitly identified owner wire schema for LD04. Moving/color/effects
and AUTO remain unavailable. New fixture corpus belongs in tests/fixtures/lx04/v1.

Authority time is injected monotonic10ms ticks; Unix service uses Instant elapsed
without wall-clock assumptions. Reads/UI redraw do not grant authority. Late update
computes current interpolation once, never replays ticks/flashes. State revision
advances on actual changed values/ownership, not mere elapsedtime; observations have
independent sequence. Commit increments revision; same-tick duplicate does not.
A cached old command reply cannot roll back the engine or a newer client snapshot.

Preview validates actual source Hold and capabilities, targets<=64/intensityonly,
then computes the real engine destination by removing only those Hold masks in a
private candidate. It is valid strictly before issue_tick+200, bound to show/epoch/
patch/state revision and targets/current/destination. Wire token additionally binds
writer/lease/scope and includes an opaque128-bit OS-random nonce. Caller cannot
construct/deserialise a trusted ReleasePreview: commit looks up its private stored
preview, requires exact canonical parsed payload, and revalidates real sources.
No file/device opens for randomness; bounded nonblocking getrandom failure refuses.
One unused preview per active writer; cancel/expiry/retire/conflict changes no Hold.
Normal state mutation invalidates unused preview. Renewal does not extend a preview.

Commit atomically removes reviewed Hold masks and installs a bounded named logical
release layer, with current/target/start/end/progress reported. Snapshot attributes
have optional `release` current value and source/contributor `release`, distinct
from human Hold. This optional field is absent from unchanged staticLX03 outputs.
Release layer masks ordinary playback, preserves every underlying contributor,
and is removed at exact end, revealing the unchanged computed destination. Programmer
wins over release. Non-intensity and unrelated programmer/Hold remain untouched.
Interpolation is integer nearest/half-away from zero. No physical/submitted/observed
claim; null output stays disarmed, physicalunknown.

Ordinary Touch first cancels only touched transition targets from their actual
current values, preserving that position in Hold beneath the explicit programmer.
Other ordinary state commands conservatively cancel remaining transition to its
current Hold before changing state; no destination shift or late automatic jump.
Failed commands keep the original complete state/transition. Losing lease stops
future writes and removes unused preview; an already applied transition continues
against authority time. New writer takeover cannot reuse the previous preview.

Validation cases: exactE05 0Hold->700playback, midpoint350/end700;700->500;
nearest half case; latejump directend with one revision; invalid multitar get,
stale revision/expiry/cancel/forged payload/wronglease; scopedmanualtouch retaining
other transition; committed fade after lease expiry; exact cachedcommit no rollback.
Physical/historical/exhaustive/live/load remain skipped.

[Owning plan](0027-gigpies-implementation.md) · [Wire](0029-lighting-wire.md)

Explicit CLI `lux-service --synthetic-private-dir /private/canonical/path --timed` selects lx04-v1. Without --timed it retains accepted LX03 static capability.

## LX04B private durability

Implemented/offline-validated; final root artifact review pending. Explicit CLI option `--durable`
selects lx04-durable-v1 and reserves a new epoch durably before binding. Files in
already verified private0700 directory: lighting-owner.lock (exclusive nonblocking
lifetime lock), lighting-epoch.json (strict reserved epoch), lighting-checkpoint.json.
The acquired owner lock has its own lifetime guard before startup validation.
Both refused initialization and normal shutdown explicitly unlock it, even if a
concurrent fork temporarily retains a descriptor until exec. Closing only the
parent descriptor is insufficient in that case. Failed acquisition never creates
the guard and cannot release another owner's lock. Refusals still preserve the
stored identity, checkpoint and epoch; a later valid startup reserves a fresh epoch.
Every existing file must be regular, sameeffectiveUID and0600, O_NOFOLLOW; no
symlink or unowned replacement. Missing/corrupt registry after prior ownership
refuses; it is never reset to1. Two crashes with no new checkpoint still reserve
successive epochs. Missing first checkpoint is explicit None/volatile defaults,
not recovered state; corrupt/future/wrongshow checkpoint refuses startup intact.

Wire `{action:"checkpoint"}` is an explicit bounded owner command, same scoped
lease/replay/revision rules, with no client path. It persists versioned validated
patch, independent copied cue/palette stores, original manualHold mask, actual
current pre-master intended look, masters and blackout. Programmer, active playback,
release/unusedpreview/lease/cache are transient. On restart all actual checkpointed
current intent (including programmer or midrelease) is frozen into Hold, Manual
mode, no active playback/transition; stores remain stored and do not GO. Human
held/current look and blackout remain intact, with no transientjump/replay.
Fresh epoch/revision0 requires fresh snapshot/grant. Physical/output stayunknown/
disarmed; checkpoint does not arm or recall any scene.

Checkpoint data uses strict bounded JSON<=1MiB/depth12, denyunknown/duplicate fields,
canonical u64strings and real Patch/mask/source validation before adoption. Existing
mismatched/future files refuse overwrite. Writes create_new0600 owned temp, write/
flush/filefsync, rename, directoryfsync. Error before rename leaves prior file intact;
error after rename reports uncertainty rather than claiming old file still installed.
Only successfully created own scratch is removed, never a filename collision.
No source state or revision changes on failed checkpoint; service durabilityerror is
honest diagnostic state. Successful sync reports checkpointed; future logical edits
or timed changes reportvolatile. Applied does not itself imply durability.

A crash leaves a socket endpoint, and binding intentionally refuses preexisting
endpoints. An operator must explicitly remove a known stale owned socket after
verifying no owner remains; there is no automatic arbitrary unlink. Controlled CLI
restart tests remove only their own known created socket after killing/waiting their
own child. They do not remove other endpoints, services or sessions.

Final review repair: every executable listener (static/timed/durable) reserves and
retains a fresh durable epoch owner, even if scene checkpointing is unavailable.
Static/timed modes keep their prior scene/schema behavior and do not recall saved
scenes; they now fail closed on incompatible epoch/checkpoint ownership. Separate
schema-aware Snapshot validation allows checkpointed/error only for identified
lx04-durable-v1; LX03 retains its strict volatile-only validation. Real provider
checkpointed/error page roundtrips and all-mode repeated CLI crash cases are added.
