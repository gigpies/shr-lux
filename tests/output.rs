use shr_lux::{
    authority::{Authority, Command},
    fixture::{Attribute, FixtureSpec, Id, Patch, PatchSpec, SyntheticMode, VERSION, Value},
    output::*,
};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};
fn authority(epoch: u64) -> Authority {
    Authority::new(
        Patch::validate(PatchSpec {
            version: VERSION,
            patch_revision: 1,
            fixtures: vec![FixtureSpec::synthetic(
                Id::new("fixture").unwrap(),
                "Synthetic",
                SyntheticMode::RgbPosition,
                1,
            )],
        })
        .unwrap(),
        "11111111-1111-4111-8111-111111111111",
        epoch,
    )
    .unwrap()
}
fn touch(a: &mut Authority, value: i32) {
    a.execute(Command::Touch(vec![Value {
        fixture: Id::new("fixture").unwrap(),
        attribute: Attribute::Intensity,
        value,
    }]))
    .unwrap();
}
struct Gate {
    sent: Sender<Submission>,
    replies: Receiver<Result<(), Fault>>,
}
impl Sink for Gate {
    fn submit(&mut self, s: &Submission, deadline: Instant) -> Result<(), Fault> {
        self.sent.send(s.clone()).unwrap();
        self.replies
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Err(Fault::Timeout))
    }
}
fn worker() -> (
    OutputWorker,
    Receiver<Submission>,
    Sender<Result<(), Fault>>,
) {
    let (tx, rx) = mpsc::channel();
    let (reply, replies) = mpsc::channel();
    (
        OutputWorker::spawn(
            Gate { sent: tx, replies },
            Limits {
                timeout: Duration::from_secs(1),
                stale_after: Duration::from_secs(10),
                ..Limits::default()
            },
        )
        .unwrap(),
        rx,
        reply,
    )
}
fn receive(rx: &Receiver<Submission>) -> Submission {
    rx.recv_timeout(Duration::from_secs(2)).unwrap()
}
fn wait(w: &OutputWorker, predicate: impl Fn(&Status) -> bool) -> Status {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let s = w.status();
        if predicate(&s) {
            return s;
        }
        assert!(Instant::now() < deadline, "status timeout: {s:?}");
        std::thread::yield_now();
    }
}
fn configure(w: &OutputWorker, a: &Authority) {
    w.configure(a, Some(Frame::explicit_safe(a, [17; 512])))
        .unwrap();
}
#[test]
fn real_authority_hold_blackout_and_unrelated_attributes_reach_fake_sink() {
    let mut a = authority(1);
    touch(&mut a, 800);
    a.execute(Command::Touch(vec![
        Value {
            fixture: Id::new("fixture").unwrap(),
            attribute: Attribute::Pan,
            value: 900,
        },
        Value {
            fixture: Id::new("fixture").unwrap(),
            attribute: Attribute::Tilt,
            value: -450,
        },
    ]))
    .unwrap();
    a.execute(Command::ClearToHold).unwrap();
    let before = Frame::synthetic(&a);
    assert_eq!(before.slots()[0], 204);
    assert_eq!(before.slots()[4], 170);
    assert_eq!(before.slots()[5], 85);
    a.execute(Command::Blackout(true)).unwrap();
    let (mut w, rx, reply) = worker();
    configure(&w, &a);
    w.arm().unwrap();
    let sent = receive(&rx);
    assert_eq!(sent.frame.slots()[0], 0);
    assert_eq!(&sent.frame.slots()[1..], &before.slots()[1..]);
    assert_eq!(a.snapshot().fixtures[0].attributes[0].hold, Some(800));
    assert!(
        a.snapshot().fixtures[0]
            .attributes
            .iter()
            .all(|v| v.submitted.is_none() && v.observed.is_none())
    );
    assert_eq!(sent.frame.request().data.len(), 512);
    reply.send(Ok(())).unwrap();
    let status = wait(&w, |s| s.submitted.is_some());
    assert_eq!(status.submitted.unwrap(), sent);
    assert_eq!(status.observed, None);
    w.disarm().unwrap();
    let safe = receive(&rx);
    assert!(safe.safe);
    assert_eq!(safe.frame.slots(), &[17; 512]);
    reply.send(Ok(())).unwrap();
    w.shutdown();
}
#[test]
fn blocked_sink_does_not_block_authority_and_mailbox_keeps_latest_only() {
    let mut a = authority(1);
    let (mut w, rx, reply) = worker();
    configure(&w, &a);
    w.arm().unwrap();
    let first = receive(&rx);
    for value in 1..=1000 {
        touch(&mut a, value);
        w.publish(&a).unwrap();
    }
    assert_eq!(w.status().in_flight, Some(first));
    assert!(rx.try_recv().is_err());
    reply.send(Ok(())).unwrap();
    let latest = receive(&rx);
    assert_eq!(latest.frame.slots()[0], 255);
    assert_eq!(latest.frame.revision, a.snapshot().revision);
    assert_eq!(latest.generation, w.status().generation);
    // Disarm invalidates pending state, but cannot undo the one reserved submission.
    w.disarm().unwrap();
    assert_eq!(w.configure(&authority(2), None), Err(Refusal::Busy));
    reply.send(Ok(())).unwrap();
    let safe = receive(&rx);
    assert!(safe.safe);
    reply.send(Ok(())).unwrap();
    wait(&w, |s| {
        s.in_flight.is_none() && s.submitted.as_ref().is_some_and(|s| s.safe)
    });
    assert!(rx.try_recv().is_err());
    configure(&w, &authority(2));
    assert_eq!(w.publish(&a), Err(Refusal::Identity));
    assert_eq!(w.status().state, State::Disarmed);
    w.shutdown();
}
#[test]
fn failed_or_timed_out_submission_latches_until_explicit_recovery() {
    for fault in [Fault::Refused, Fault::Timeout] {
        let a = authority(1);
        let (mut w, rx, reply) = worker();
        configure(&w, &a);
        w.arm().unwrap();
        let job = receive(&rx);
        w.disarm().unwrap(); // Late failure must not be hidden by disarm.
        reply.send(Err(fault)).unwrap();
        let s = wait(&w, |s| matches!(s.state, State::Fault(_)));
        assert_eq!(s.failed, Some((job, fault)));
        assert!(s.submitted.is_none());
        assert_eq!(w.arm(), Err(Refusal::State));
        assert_eq!(w.publish(&a), Err(Refusal::State));
        assert_eq!(w.configure(&a, None), Err(Refusal::State));
        assert!(rx.try_recv().is_err()); // No retry, even safe output.
        w.recover().unwrap();
        assert_eq!(w.arm(), Err(Refusal::Policy));
        configure(&w, &a);
        w.arm().unwrap();
        receive(&rx);
        reply.send(Ok(())).unwrap();
        wait(&w, |s| s.submitted.is_some());
        // Queue both bounded shutdown completions before joining.
        reply.send(Ok(())).unwrap();
        assert_eq!(w.shutdown().state, State::Stopped);
    }
}
#[test]
fn policy_identity_revision_and_restart_refusals() {
    let mut a = authority(1);
    let (mut w, rx, _reply) = worker();
    assert_eq!(w.status().state, State::Disarmed);
    w.configure(&a, None).unwrap();
    assert_eq!(w.arm(), Err(Refusal::Policy));
    assert_eq!(
        w.configure(&a, Some(Frame::explicit_safe(&authority(2), [0; 512]))),
        Err(Refusal::Identity)
    );
    let old = a.clone();
    touch(&mut a, 700);
    w.publish(&a).unwrap();
    assert_eq!(w.publish(&old), Err(Refusal::Revision));
    assert!(rx.try_recv().is_err());
    w.shutdown();
    assert_eq!(w.arm(), Err(Refusal::State));
    let (mut fresh, rx, _) = worker();
    assert_eq!(fresh.status().state, State::Disarmed);
    assert!(fresh.status().submitted.is_none());
    assert!(rx.try_recv().is_err());
    fresh.shutdown();
}
#[test]
fn shutdown_drains_at_most_reserved_plus_safe_and_never_arms() {
    let a = authority(1);
    let (mut w, rx, reply) = worker();
    configure(&w, &a);
    w.arm().unwrap();
    receive(&rx);
    reply.send(Ok(())).unwrap();
    reply.send(Ok(())).unwrap();
    let s = w.shutdown();
    assert_eq!(s.state, State::Stopped);
    assert!(s.submitted.unwrap().safe);
    assert!(receive(&rx).safe);
    assert!(rx.try_recv().is_err());
}
#[test]
fn backend_panic_becomes_fault_and_does_not_poison_control_lock() {
    struct Panic;
    impl Sink for Panic {
        fn submit(&mut self, _: &Submission, _: Instant) -> Result<(), Fault> {
            panic!("synthetic backend panic")
        }
    }
    let a = authority(1);
    let mut w = OutputWorker::spawn(Panic, Limits::default()).unwrap();
    configure(&w, &a);
    w.arm().unwrap();
    wait(&w, |s| s.state == State::Fault(Fault::Panicked));
    w.recover().unwrap();
    w.shutdown();
}

#[test]
fn changed_patch_or_show_is_refused_even_with_same_patch_revision() {
    let a = authority(1);
    let (mut w, _rx, _reply) = worker();
    configure(&w, &a);
    let mut spec = PatchSpec {
        version: VERSION,
        patch_revision: 1,
        fixtures: a.patch().fixtures().to_vec(),
    };
    spec.fixtures[0].address = 22;
    let changed = Authority::new(Patch::validate(spec).unwrap(), &a.snapshot().show_id, 1).unwrap();
    assert_eq!(w.publish(&changed), Err(Refusal::Identity));
    let other =
        Authority::new(a.patch().clone(), "22222222-2222-4222-8222-222222222222", 1).unwrap();
    assert_eq!(w.publish(&other), Err(Refusal::Identity));
    configure(&w, &changed);
    assert_eq!(w.publish(&a), Err(Refusal::Identity));
    w.shutdown();
}

#[test]
fn bounded_fake_sink_deadline_faults_without_retry() {
    let (tx, rx) = mpsc::channel();
    let (_reply, replies) = mpsc::channel();
    let mut w = OutputWorker::spawn(
        Gate { sent: tx, replies },
        Limits {
            timeout: Duration::from_millis(25),
            ..Limits::default()
        },
    )
    .unwrap();
    configure(&w, &authority(1));
    w.arm().unwrap();
    receive(&rx);
    let s = wait(&w, |s| s.state == State::Fault(Fault::Timeout));
    assert!(s.submitted.is_none());
    assert!(s.failed.is_some());
    assert!(rx.try_recv().is_err());
    w.shutdown();
}
