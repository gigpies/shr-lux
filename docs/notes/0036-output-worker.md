# Device-independent output worker

`src/output.rs` is an opt-in internal LX-06 library boundary. The application and
private service still use null-disarmed output, with unchanged C-LIGHT snapshots.
No USB backend, physical profile, device opening, CLI arm action or deployment is
provided. [The task card](0027-gigpies-implementation.md#dev-pc-package--bounded-software-output-worker)
owns implementation state and acceptance evidence.

## Authority integration and encoding

`OutputWorker::configure(&authority, safe_policy)` captures a coherent authority
view. `publish(&authority)` is called after logical commands/ticks and also serves
as a freshness heartbeat. The caller serializes authority access as usual; capture
and bounded mailbox replacement may occur under its authority lock. The owned
worker never accesses that authority or holds the mailbox lock during sink I/O.
Commands remain available while a sink is busy. This is a regular-thread API, not
an audio-callback API; bounded snapshots and patch copies can allocate.

`Frame::synthetic` consumes the real authority's `final_intent`, retaining manual
Hold resolution, master levels and intensity-only blackout. It uses only the
existing explicitly invented `SyntheticMode` personalities: one byte per attribute
in `mode.attributes()` order at the validated fixture address. Each capability
range maps linearly to 0..255 using integer floor; a singleton range maps to zero.
Unpatched slots are zero in this synthetic encoding. These choices are test
encoding, **not real fixture profiles or a safety claim**. Existing `encode_range`
validates the full 512-slot request, without a DMX start code.

Frames carry show ID, owner epoch, patch revision, logical revision and the full
validated patch. Mailbox submissions additionally carry a monotonic generation
and a safe/desired discriminator. Comparing the full patch also rejects a changed
patch whose author reused its revision. Different show/epoch/patch or a regressed
logical revision is refused. Equal revision with different intent is refused.
Reconfiguration is explicit, disarmed and requires all pending/in-flight work to
drain, so an old configuration cannot enter the sink after a new one is installed.

## Bounds and cancellation

There is one thread, one replaceable pending frame and at most one in-flight
submission. Status retains only the latest desired, submitted and failed records;
there is no growing history or retry queue. Each frame is exactly 512 slots and a
patch bounded by the existing 32-fixture contract. A fast publisher replaces
pending desired state; it never queues intermediate fades or stale snapshots.

Defaults: submission starts at most every 25 ms, backend deadline 100 ms, source
freshness 500 ms. Configured intervals/deadlines must be 1..1000 ms; freshness must
be at least the interval and at most 10 seconds. These are software limits, not
qualified device timing. Rate limiting applies to safe frames too. No catch-up
burst or periodic frame retransmission occurs: a publication queues at most one
latest frame. A future device adapter must define any required refresh policy.

Reservation under the mailbox mutex is the submission linearization point. Once
reserved, **one** old submission may finish after disarm. Cancellation cannot undo
bytes already submitted; even a reservation paused by the OS may enter its sink
later. Pending desired state is removed, and only the explicit safe frame follows.
A sink must honor its supplied absolute deadline, including cleanup, and retain no
asynchronous operation after returning. Success after the deadline is a timeout.
The worker cannot safely kill an arbitrary blocked backend thread. Shutdown joins
its thread and, for a compliant backend, waits for at most an existing operation,
one safe operation and one rate interval, plus scheduling overhead. It does not
claim a hard realtime wall-clock bound or a bound for a noncompliant backend.

## Lifecycle and observations

- Creation/restart is disarmed and never transmits automatically.
- Arm requires fresh intent and an explicitly supplied, identity-matching safe
  frame. `Frame::explicit_safe` accepts the caller's reviewed full slot array.
  No generic all-zero safe policy or movement/blackout policy is inferred.
- Disarm drops pending desired state and queues the policy once. Repeated disarm
  is idempotent. Arm/configure refuse until that submission drains.
- Source expiry latches `Fault(StaleSource)` and queues the policy once. A new
  heartbeat cannot clear the fault. Expiry is checked before reservation and
  publication; an in-flight operation can finish first within its backend bound.
- Refusal, timeout or backend panic latches a fault and clears pending work. There
  is no automatic retry, including safe-frame retry after transport failure.
  Physical output is unknown; safe output is not promised over a failed transport.
  A late failure still latches after disarm and cannot be hidden by reconfiguration.
- Recovery requires explicit `recover()` after work drains, then `configure()`
  with a newly reviewed policy/fresh intent, then `arm()`. No automatic rearm.
- Shutdown sends the policy once if armed, preserves an already pending disarm/
  stale policy, then stops. A transport fault suppresses further writes. Drop
  follows the same joined shutdown contract.

`Status` distinguishes desired, in-flight, submitted and failed identity-tagged
records. Submitted is historical transport success only; it may refer to the
previous configuration and must not be presented as current physical observation.
Observed remains unknown. Late/failed operations may have partially affected a
future physical device; they never become a success record. Shutdown's stopped
state retains failed/submitted diagnostics. These internal statuses do not change
the service's null-output wire fields.

## Software checks and remaining gates

`tests/output.rs` uses the real authority and a channel-gated fake sink to exercise
Hold/blackout, mailbox replacement, blocked-I/O command progress, disarm races,
identity changes, refusal/timeout/panic, explicit recovery and joined shutdown.
Internal core tests inject time for exact rate/freshness boundaries and late
success classification. Tests use synthetic fixtures only and do not open devices.

Physical work still requires an authorized adapter/fixture session, exact profiles,
a fixture-specific reviewed safe policy, measured transfer/cancellation/refresh
behavior, wiring/electrical verification and independent observed-light acceptance.
Software submission cannot establish any of those facts.
