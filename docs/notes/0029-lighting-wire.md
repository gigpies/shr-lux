# C-LIGHT:1 encoded local authority

Task 0008 / LX03. Implemented synthetic null-output service; focused/full/Clippy validated; release/PTY completing.
This is the single owner schema consumed by LD03. Original E04 is unchanged.
No physical output, device, audio, MIDI, TCP or native display is opened.

## Binding and startup

`lux-service --synthetic-private-dir /absolute/canonical/owned-directory` is an
explicit opt-in executable. Directory must already exist, belong to the effective
user, have mode 0700 and contain no preexisting `lux.sock` (including symlinks).
Socket mode is 0600. Linux SO_PEERCRED requires same UID. The standalone simulator
is unchanged. The executable uses invented fixture-11, RGB/position profile,
show 11111111-1111-4111-8111-111111111111. Every CLI mode now reserves/retains a
fresh durable epoch before listening; see [current CLI usage](0031-local-provider-integration.md).

Each UTF-8 JSON request/reply has a 4-byte unsigned big-endian byte length, 1..65536.
Up to four connected clients, one global lighting-control writer, 3s idle first-byte timeout (exceeds healthy500ms renew interval),
500ms absolute in-progress frame/read/write deadline starting first byte, 30s connection lifetime, 1024 requests per connection.
Own worker threads are joined and only the matching created socket inode removed.
No subscriptions exist: snapshots are requested; thus no stale telemetry queue.
Replies are synchronous, maximum 16 pages (<32 queue bound), no new requests are
processed for a connection while its replies are pending. A slow peer is dropped.
Reconnect needs fresh snapshot and distinct writer identity, never queue replay.
This is trusted same-UID local IPC, not remote authentication.

## Request envelope

Exactly eleven fields: `contract:"C-LIGHT"`, `version:1`, canonical `show_id`,
`module:"lighting"`, `epoch`, `writer`, `lease`, `request_id`,
`expected_revision`, `kind`, `body`. All u64 values are canonical decimal strings:
"0" or nonzero first digit with no leading zeros, no signs or numeric JSON.
Domain IDs are validated ASCII. Unknown/duplicate keys, floats, invalid UTF-8,
trailing data, >64KiB and nesting >12 refuse. Empty-body reads require all four
writer authority fields null. Malformed read refusal leaves real state intact;
no unavailable observation is converted to zero.

`kind:snapshot` and `capabilities` both return complete coherent inventory and
never grant authority. `grant` names writer and `{scope:"lighting-control"}`, lease null,
request_id:"1", expected_revision matching the current state. Grant is part of
that writer's request sequence; first command normally uses ID2. Exact lost-grant
ACK retry while that session is live returns its cached original issued lease,
before expected_revision checking. Changed ID1 payload refuses reused_id. An
expired/retired successful session refuses lease, even for the original grant.
Stale or busy well-formed grants cache their original refusal and permanently
retire that identity; exact retry repeats refusal, changed retry refuses reused_id.
Grant refusals do not alter look/revision. Only one writer owns lighting-control,
including global commands. All attempted valid grant identities count toward the
1024-entry epoch history; capacity exhaustion refuses without forgetting history.
Lease expiry preserves programmer/Hold/look. Remaining duration is2000ms; renew
explicitly, recommended500ms. Granted lease is in reply body; reply envelope echoes
null lease from the grant request.

`command`, `renew`, `retire` require writer/lease and canonical request/revision.
Fresh writer sequence starts with grant1 and strictly increases thereafter. Lease identity is
validated before cache; cache before expected_revision. 64 original outcomes are
retained; identical parsed JSON envelope returns the original response, changed
payload reused_id, uncached IDs <=highwater expired_id. Refused well-formed
requests also consume/cache their IDs. An old cached reply reports its original
revision/tick: consumers must never roll a newer snapshot backward. Renew/retire
body is `{scope:"lighting-control"}`; renew is explicit, recommended every500ms.

Command body has exactly `scope` and `command`; command is tagged by `action`:
`touch {values:[{fixture,attribute,value}]}`, `clear_to_hold`,
`record/update {kind:"cue"|"palette",id}`, `apply_palette {id}`,
`go {cue,playback}`, `level {playback,level}`, `off {playback}`,
`master {level}`, `fixture_master {fixture,level}`, `blackout {enabled}`,
`mode {mode:"manual"|"assist"}`, `replace_patch {patch}`.
Attribute names intensity/red/green/blue/pan/tilt/zoom. Groups and ranges validate
against actual owner patch. AUTO unavailable. Replace patch uses owner PatchSpec,
including decimal-string patch_revision. No caller constructs trusted engine state.

Replies have contract/version/show/module/epoch, writer/lease/request_id/
expected_revision, decimal sequence/revision/effective_tick, kind/reason/body.
Mutation replies echo their exact valid request/session identity, including
refusals and cached original outcomes. Malformed unidentified requests carry null
identity; valid parsed wrong-show/epoch/version refusals retain correlation fields.
Snapshot observation replies have null writer/lease/request/expected_revision. Applied means actual logical static
commit, never physical delivery. Refusals preserve authoritative engine revision
and all applied state. Counter arithmetic fails closed. Tick basis: injected
monotonic 10ms logical ticks, local service derives elapsed Instant, not wall time.

## Snapshot pages and consumer algorithm

A response contains <=16 `kind:snapshot` pages sharing show/epoch/revision/sequence.
Body: zero-based integer page, integer page_count, encoding:"json_utf8_chunks",
chunk:string. Chunks are consecutive pieces of one JSON UTF-8 text, **not separate
JSON documents**. Join by page index, then parse once. Accept every page exactly
once within 200 ticks / 2s of first receipt; reject mixed identities/counts, duplicate
pages and incomplete timeout. Drop whole pending set on failure. The owner has a **private test-only** assembly checker, not a public trusted
inventory decoder. It preflights aggregate <=16x65536 bytes/depth12, rejects
duplicates, validates inner show/epoch/revision against page headers and validates
typed Snapshot against typed validated Patch. Consumers must additionally validate
the entire owner inventory schema/capabilities before trust; joining chunks alone
is not semantic validation. Newest complete revision wins, never a partial view. Wire corpus includes actual page replies produced by real commands.

Inventory contains full snapshot (programmer/Hold/stores/playing/proposal,
contributors/all winners/resolved/final intent/submitted/observed/inhibit), real
synthetic capabilities/ranges/defaults/units via attribute vocabulary, stable fixture
IDs and all-group, patch_revision, mode/master/blackout, complete cue/palette/
playback inventories including empty objects, levels/activation orders and fixture
masters. Output null_disarmed, physical unknown, submitted/observed null. No
analysis proposal is fabricated. Volatile durability is explicit.

Negotiated bounds: 32fixtures,8playbacks,32cues,32palettes,64targets,1writer,
1024 epoch writer identities,64cached outcomes,16pages,65536encoded bytes/page.
**The complete encoded page budget is an additional state capacity limit.**
Long identities/repeated masks can reach it before object-count limits. Service
construction validates pageability; commands clone/execute/validate pages/commit,
refusing capacity atomically. No accepted edit can create an unreadable snapshot,
and no inventory is silently dropped. Max valid-state regression uses all32
fixtures and64-byte identities, filling actual stores until this honest budget
refusal, then reassembles and compares complete owner inventory.

## Corpus and evidence

`tests/fixtures/lx03/v1/commands.json` is generated by the real Service over the
real LX02 Authority. Default test repeats every command and compares exact replies;
reproduce deliberately with `LUX_GENERATE_CORPUS=1 cargo +1.97.1 test --locked -j1
--test wire reads_never_grant_and_actual_commands_produce_corpus` under host build
lock. Corpus is provider data, not handwritten simulated expected results.
Focused production tests cover malformed/version/show/epoch/scope/range refusal,
replay/eviction/reorder/first-ID, expiry/retired history exhaustion, coherent pages/
deadline and maximum-state atomicity, private framed fragmentation/oversize/
stalled input/reconnect/cleanup. Physical/historical/exhaustive/load tests excluded.

[Notebook](../reference/NOTEBOOK.md) · [Owning plan](0027-gigpies-implementation.md)

LX03 measured validation: focused 8 passed; full normal suite 69 passed; warning-denied all-target Clippy passed. Release and post-release PTY completing in the private checkpoint evidence. No ignored normal tests.

R2 repairs reviewed grant retry/correlation and separates idle/in-progress socket deadlines. Public arbitrary-JSON page consumer seam removed; test-only checker validates bounded depth and header/payload/patch identities. Lost grant ACK, stale grant and healthy650ms idle regressions added.
