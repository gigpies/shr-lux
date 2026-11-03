# SHR Lux: plan and implementation

Lux owns lighting authority, fixture capabilities, programmer/Hold/cue/playback,
analysis, timing, effects and physical output. Lightdesk presents that authority;
GigPies supplies named source analysis and final system integration.

## Delivered contract and evidence

LX-01..05 have software acceptance: fixture/patch contracts, static authority,
private service, timed release/durable disarmed restart and named PCM analysis with
bounded ASSIST/AUTO. Actual provider/consumer execution uses null output.
[Authority](0028-static-authority.md), [wire](0029-lighting-wire.md),
[release/recovery](0030-release-recovery.md), [provider](0031-local-provider-integration.md)
and [analysis](0032-analysis-subscription.md) own exact behavior/evidence.
Optional music analysis adds chord/key/Camelot, movement and tentative recurring
sections; [LX-07 synthetic software acceptance](archive/2026-10-11-musical-understanding.md)
records its validation. Beat/downbeat and physical light remain unavailable.

## LX-06 — physical fixture output

State: PLANNED; physical acceptance requires a separate authorized session and known
fixture/adapter/patch facts. Owner: SHR Lux. Outcome: one bounded output worker with
explicit arm/rearm, exact fixture encoding and visible submission/fault state.
Next: establish the real profile and fixture-specific loss/blackout/recovery policy,
then implement and qualify the adapter in a separately authorized session. Do not
invent fixture maps, movement limits or generic all-zero safety. `src/dmx.rs` currently encodes ranges, not USB writes.
Acceptance: encoding/worker bounds, unknown versus submitted versus observed output,
fault/restart/rearm and fixture-specific physical checks. No automatic rearming.
Progress/evidence: the dev-PC worker below implements the software boundary only;
no physical output implementation or qualification.

### Dev-PC package — bounded software output worker

Selected 2026-11-03 to run independently of the two-Pi audio transport work.
State: software-complete and offline-validated. Own SHR Lux only,
starting from the synchronized `main` after a fast-forward pull. Implementation
base: `e0f50404eac688ffc2c12b7dfc19ce287a31c546`. GigPies, Desk, Lightdesk, PA, FX
and REC are read-only.
Keep this card as the sole plan/status owner; do not create another handoff forest.

Implement the device-independent part of LX-06 with an explicitly injected fake
sink. Real USB/DMX access, fixture profiles, wiring, timing qualification and
service deployment remain deferred. This is useful before adapter facts are known:
it establishes the ownership and failure behavior a future adapter must satisfy.

1. Inspect existing authority, fixture/patch validation, null sink and restart
   contracts. Specify a small internal boundary carrying validated encoded frames,
   show/epoch/patch identity and submission generation. Keep C-LIGHT wire snapshots
   and default null-disarmed service behavior compatible; no sibling changes.
2. Implement one bounded latest-frame mailbox and one owned output worker. I/O
   cannot hold the authority lock or block command processing. A slow sink must not
   create an unbounded queue; stale generations must not submit after reconfiguration.
   Bound request size, update rate, pending state, retry policy and shutdown.
3. Define explicit arm/disarm/rearm, stale-source and fault transitions. Restart
   remains disarmed. Transport success is submission only, never observed physical
   output. A failed/timed-out sink requires explicit recovery; never auto-rearm.
   Fixture-specific safe output must be supplied explicitly; missing policy refuses
   arming. Never assume all-zero DMX is a safe movement/blackout command.
4. Use deterministic fake sinks and synthetic patch fixtures for success, refusal,
   timeout, delayed completion, latest-state replacement, identity/epoch changes,
   disarm races and shutdown. State the maximum in-flight submission semantics:
   cancellation cannot undo bytes already submitted. A backend must implement a
   bounded operation; do not claim an arbitrary blocked thread can be killed safely.
5. Exercise the real authority-to-worker software boundary with synthetic intent,
   preserving manual Hold, blackout intensity semantics and unrelated attributes.
   Distinguish desired, submitted, failed and unknown observations. Reuse existing
   encoding where applicable, without inventing a physical fixture channel map.
6. Run focused production tests while implementing, then the complete normal
   all-target suite, formatting, warnings-denied Clippy, release build and Linux
   PTY checks required by AGENTS.md. Use the pinned toolchain, committed lockfile,
   normal target directory and CARGO_INCREMENTAL=0; coordinate the local build lock.
   Hardware, exhaustive timing/load campaigns and media downloads stay excluded.

Completion: tested worker and internal integration, concise contract/limits in the
owning reference, updated checklist/evidence here, clean reviewed source commit
and publication-policy checks. Push the scoped SHR Lux commit when validation
passes so the Pis can fast-forward it after their test reservation. Do not change
shared protocol versions or activate a device to make the task appear complete.
Report exact commit, commands/results and the remaining adapter/physical gates.

The synchronization lock regression is handled separately from this output-worker
package. The 2026-11-03 parallel run originally refused checkpoint reopening with
`WouldBlock` at tests/configured_service.rs:199; isolated and serial runs passed.
Investigation reproduced the refusal deterministically by keeping a duplicated
open-file description alive through the actual rejected legacy-startup path.
The prior explicit unlock existed only on a fully constructed CheckpointStore,
so an initialization error could leave its flock held by a descriptor inherited
during concurrent process creation until that child closed it or executed.

The acquired lock now owns an immediate RAII guard, covering all initialization
errors and normal store destruction. Failed acquisition never constructs the
guard. The regression failed before the fix and passed afterward, checks refused
startup leaves identity/epoch files unchanged, permits immediate fresh recovery,
and verifies the new owner's exclusion survives dropping the old duplicate.
No retry, sleep, deadline or assertion was loosened. See the
[recovery contract](0030-release-recovery.md#lx04b-private-durability).
Validation: deterministic regression red before the fix, green afterward;
configured-service target 8/8; full normal parallel all-target suite 149 passed,
0 failed, 1 opt-in ignored. Formatting, warnings-denied all-target Clippy, six
synthetic preparation tests, release build and all eight Linux PTY checks passed.
Historical/exhaustive and physical tests were intentionally skipped. This repair
is complete and independent of the dev-PC output-worker package.

Acceptance checklist:

- [x] Bounded authority/worker boundary and explicit lifecycle implemented.
- [x] Deterministic output/fault/recovery/race tests pass without devices.
- [x] Default null-disarmed behavior and existing wire contract preserved.
- [x] Required normal validation and publication checks pass.
- [x] Software completion and remaining physical limitations documented.

### Dev-PC completion evidence — 2026-11-03

Implemented `src/output.rs` with direct real-authority capture, synthetic frame
encoding, one latest pending frame and one owned sink worker. Explicit safe-policy
arming, disarm, stale-source fault, failed/late completion, recovery/rearm and joined
shutdown are bounded as described in the [output contract](0036-output-worker.md).
No physical backend, wire schema change or default-service activation was added.

Eight fake-sink integration tests and three deterministic core tests cover the new
boundary. The complete `sh scripts/check.sh` passed with Rust 1.97.1 and the committed
lockfile: formatting; all-target Clippy with `-D warnings`; all-target normal tests
**160 passed, 0 failed, 1 opt-in ignored**; six synthetic preparation tests; release
build; eight Linux PTY checks. Commands ran serially with `CARGO_INCREMENTAL=0`,
`CARGO_BUILD_JOBS=1`, nice 15, idle I/O priority and the shared
`/home/shome/p/.gigpies-build.lock`. Focused commands were
`cargo test --locked output` and `cargo test --locked --test output`.
The initial Clippy nested-if finding was fixed before the complete successful run.

Publication review covers the complete index for private/generated paths, media,
archives and credential markers, plus the named staged source/documentation diff
and whitespace checks. Lux has no versioned publication-policy script/hooks; no
scripts, binary assets, private state or generated evidence are added by this change.
Historical/exhaustive timing/load campaigns, private-media auditions, downloads and
all physical tests remain intentionally excluded. All work stayed in SHR Lux; the
external knowledge notebook remains unchanged under this task's repository-only scope.

Next: after the Pis' independent reservation, they may fast-forward the published
source. Actual adapter/profile mapping, device deadlines/refresh/cancellation,
fixture-specific safe output, electrical/wiring and observed-light acceptance remain
separate authorized gates. A sink success proves submission only, and an arbitrary
blocked backend cannot safely be killed to manufacture a shutdown guarantee.

## Later engine expansion

Cue-list/fade/effect/movement and venue patch authoring need explicit scope and one
card here when selected. Preserve manual Hold and per-attribute source explanation.
These are owner algorithms; Lightdesk must not substitute its simulator. Shared
whole-show qualification lives in GP-H3, not a duplicate Lux load tracker.

## Tracking and verification

This is the owning plan for module-only GigPies work. Keep each new task's plan,
implementation state, checklist, evidence and next action together here. Shared
integration tasks live only in the [GigPies integration plan](https://github.com/gigpies/gigpies/blob/main/docs/development/MODULE_IMPLEMENTATION_PLAN.md);
link to their cards instead of copying status. Follow its task lifecycle and this
repository's AGENTS.md. STATUS/acceptance files hold dated evidence, not another queue.
Preserve closed milestone details in the linked archive; do not reopen old launch cards.
A documentation reconciliation does not rerun tests or qualify hardware.

[Historical plan and milestone evidence](archive/tracking-before-2026-10-09/0027-gigpies-implementation.md). Its launch instructions are retired.

## Shared scene integration

[GP-SCENES](../../../gigpies/docs/archive/records/2026-10-11-gp-scenes.md) is the archived shared acceptance card for Lux director and Lightdesk editor work. Owner behavior and protocol are documented in [0035](0035-scene-map.md); task state, acceptance and next action stay in GigPies.
