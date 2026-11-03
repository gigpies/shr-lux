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
Next: establish the real profile and loss/blackout/recovery policy, then prepare
software contracts/tests before any output. Do not invent fixture maps, movement
limits or generic all-zero safety. `src/dmx.rs` currently encodes ranges, not USB writes.
Acceptance: encoding/worker bounds, unknown versus submitted versus observed output,
fault/restart/rearm and fixture-specific physical checks. No automatic rearming.
Progress/evidence: no physical output implementation or qualification.

### Dev-PC package — bounded software output worker

Selected 2026-11-03 to run independently of the two-Pi audio transport work.
State: ready for implementation; no worker has been launched. Own SHR Lux only,
starting from the synchronized `main` after a fast-forward pull. Record that exact
base commit before editing. GigPies, Desk, Lightdesk, PA, FX and REC are read-only.
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

Validation follow-up from source synchronization: the 2026-11-03 parallel normal
suite observed `WouldBlock` when reopening the checkpoint owner lock in
`configured_service::locked_config_identity_and_checkpoint_recovery_preserve_show_holds_without_rearm`
(line 199). The isolated case passed. Investigate the lifetime of locks on refused
startup paths alongside concurrent CLI spawning before claiming parallel-suite
reliability; the cause is not yet established. The complete serial all-target
suite passed 148 tests, with one opt-in ignored, using `-- --test-threads=1`.
No assertion, deadline or runtime source was changed. Preserve exclusive ownership and
the refusal assertions. Record this separately from the new output worker.

Acceptance checklist:

- [ ] Bounded authority/worker boundary and explicit lifecycle implemented.
- [ ] Deterministic output/fault/recovery/race tests pass without devices.
- [ ] Default null-disarmed behavior and existing wire contract preserved.
- [ ] Required normal validation and publication checks pass.
- [ ] Software completion and remaining physical limitations documented.

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
