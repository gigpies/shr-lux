//! Opt-in, device-independent LX-06 worker. No USB backend or service activation.
use crate::{authority::Authority, dmx, fixture::Patch};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub show: String,
    pub epoch: u64,
    pub patch_revision: u64,
}

/// Full synthetic universe; private construction preserves validated identity/patch.
/// Synthetic attribute order and linear scaling are NOT real fixture personalities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub identity: Identity,
    pub revision: u64,
    patch: Patch,
    slots: [u8; dmx::CHANNELS],
}
impl Frame {
    pub fn synthetic(authority: &Authority) -> Self {
        let snapshot = authority.snapshot();
        let mut slots = [0; dmx::CHANNELS];
        for fixture in authority.patch().fixtures() {
            let values = snapshot
                .fixtures
                .iter()
                .find(|f| f.fixture == fixture.id)
                .unwrap();
            for (offset, attribute) in fixture.mode.attributes().iter().enumerate() {
                let cap = fixture.capability(*attribute).unwrap();
                let value = values
                    .attributes
                    .iter()
                    .find(|v| v.attribute == *attribute)
                    .unwrap()
                    .final_intent;
                let range = i64::from(cap.max) - i64::from(cap.min);
                slots[usize::from(fixture.address - 1) + offset] = if range == 0 {
                    0
                } else {
                    ((i64::from(value) - i64::from(cap.min)) * 255 / range) as u8
                };
            }
        }
        Self {
            identity: Identity {
                show: snapshot.show_id,
                epoch: snapshot.epoch,
                patch_revision: snapshot.patch_revision,
            },
            revision: snapshot.revision,
            patch: authority.patch().clone(),
            slots,
        }
    }
    /// Caller-reviewed synthetic safe output; never inferred from defaults or blackout.
    pub fn explicit_safe(authority: &Authority, slots: [u8; dmx::CHANNELS]) -> Self {
        Self {
            slots,
            ..Self::synthetic(authority)
        }
    }
    pub fn slots(&self) -> &[u8; dmx::CHANNELS] {
        &self.slots
    }
    pub fn request(&self) -> dmx::RangeRequest<'_> {
        dmx::encode_range(dmx::Address::new(1).unwrap(), &self.slots).unwrap()
    }
    fn same_configuration(&self, other: &Self) -> bool {
        self.identity == other.identity && self.patch == other.patch
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Refused,
    Timeout,
    StaleSource,
    Panicked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Disarmed,
    Armed,
    Fault(Fault),
    Stopped,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Policy,
    Identity,
    Revision,
    Busy,
    State,
    Stale,
    Exhausted,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submission {
    pub generation: u64,
    pub safe: bool,
    pub frame: Frame,
}
#[derive(Clone, Debug)]
pub struct Status {
    pub state: State,
    pub desired: Option<Frame>,
    pub in_flight: Option<Submission>,
    /// Historical transport success, identity tagged; never physical observation.
    pub submitted: Option<Submission>,
    pub failed: Option<(Submission, Fault)>,
    /// Always unknown in this package.
    pub observed: Option<()>,
    pub generation: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub interval: Duration,
    pub timeout: Duration,
    pub stale_after: Duration,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            interval: Duration::from_millis(25),
            timeout: Duration::from_millis(100),
            stale_after: Duration::from_millis(500),
        }
    }
}
impl Limits {
    fn valid(self) -> bool {
        (Duration::from_millis(1)..=Duration::from_secs(1)).contains(&self.interval)
            && (Duration::from_millis(1)..=Duration::from_secs(1)).contains(&self.timeout)
            && self.stale_after >= self.interval
            && self.stale_after <= Duration::from_secs(10)
    }
}
/// A backend MUST return by the supplied deadline, including cleanup, and MUST NOT
/// retain work after returning. Rust cannot safely kill an arbitrary blocked thread.
/// Success means transport submission only. This package supplies no physical backend.
pub trait Sink: Send + 'static {
    fn submit(&mut self, submission: &Submission, deadline: Instant) -> Result<(), Fault>;
}

fn completion_result(
    result: Result<(), Fault>,
    completed: Instant,
    deadline: Instant,
) -> Result<(), Fault> {
    if completed >= deadline {
        Err(Fault::Timeout)
    } else {
        result
    }
}

struct Core {
    status: Status,
    safe: Option<Frame>,
    pending: Option<Submission>,
    fresh: Option<Instant>,
    next: Instant,
    stop: bool,
    limits: Limits,
}
impl Core {
    fn new(limits: Limits, now: Instant) -> Self {
        Self {
            status: Status {
                state: State::Disarmed,
                desired: None,
                in_flight: None,
                submitted: None,
                failed: None,
                observed: None,
                generation: 0,
            },
            safe: None,
            pending: None,
            fresh: None,
            next: now,
            stop: false,
            limits,
        }
    }
    fn advance(&mut self) -> Result<(), Refusal> {
        self.status.generation = self
            .status
            .generation
            .checked_add(1)
            .ok_or(Refusal::Exhausted)?;
        Ok(())
    }
    fn queue_safe(&mut self) {
        self.pending = self.safe.clone().map(|frame| Submission {
            generation: self.status.generation,
            safe: true,
            frame,
        });
    }
    fn expire(&mut self, now: Instant) {
        if self.status.state == State::Armed
            && self
                .fresh
                .is_none_or(|t| now >= t + self.limits.stale_after)
        {
            self.status.state = State::Fault(Fault::StaleSource);
            self.queue_safe();
        }
    }
    // Reservation under the control lock is the submission linearization point.
    fn take(&mut self, now: Instant) -> Option<Submission> {
        self.expire(now);
        if now < self.next || self.status.in_flight.is_some() {
            return None;
        }
        let job = self.pending.take()?;
        self.status.in_flight = Some(job.clone());
        self.next = now + self.limits.interval;
        Some(job)
    }
    fn complete(&mut self, job: Submission, result: Result<(), Fault>) {
        self.status.in_flight = None;
        match result {
            Ok(()) => self.status.submitted = Some(job),
            Err(fault) => {
                self.status.failed = Some((job, fault));
                self.status.state = State::Fault(fault);
                self.pending = None; // No retries, including a queued safe frame after I/O failure.
            }
        }
    }
}
struct Shared {
    core: Mutex<Core>,
    wake: Condvar,
}

/// Owns exactly one thread and a latest-frame mailbox. Not attached to default service.
pub struct OutputWorker {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}
impl OutputWorker {
    pub fn spawn(mut sink: impl Sink, limits: Limits) -> std::io::Result<Self> {
        if !limits.valid() {
            return Err(std::io::Error::other("invalid output limits"));
        }
        let shared = Arc::new(Shared {
            core: Mutex::new(Core::new(limits, Instant::now())),
            wake: Condvar::new(),
        });
        let worker = shared.clone();
        let thread = thread::Builder::new()
            .name("lux-output".into())
            .spawn(move || {
                let mut core = worker.core.lock().unwrap();
                loop {
                    let now = Instant::now();
                    if let Some(job) = core.take(now) {
                        let deadline = now + core.limits.timeout;
                        drop(core);
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            sink.submit(&job, deadline)
                        }))
                        .unwrap_or(Err(Fault::Panicked));
                        let result = completion_result(result, Instant::now(), deadline);
                        core = worker.core.lock().unwrap();
                        core.complete(job, result);
                        worker.wake.notify_all();
                    } else if core.stop && core.pending.is_none() {
                        core.status.state = State::Stopped;
                        worker.wake.notify_all();
                        break;
                    } else {
                        let mut wait = Duration::from_secs(10);
                        if core.pending.is_some() {
                            wait = core.next.saturating_duration_since(now);
                        }
                        if core.status.state == State::Armed
                            && let Some(fresh) = core.fresh
                        {
                            wait = wait.min(
                                (fresh + core.limits.stale_after).saturating_duration_since(now),
                            );
                        }
                        core = worker.wake.wait_timeout(core, wait).unwrap().0;
                    }
                }
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
    pub fn status(&self) -> Status {
        self.shared.core.lock().unwrap().status.clone()
    }
    /// Configuration is allowed only after disarm/recovery and all prior work drains.
    pub fn configure(&self, authority: &Authority, safe: Option<Frame>) -> Result<(), Refusal> {
        let desired = Frame::synthetic(authority);
        if safe
            .as_ref()
            .is_some_and(|s| !s.same_configuration(&desired))
        {
            return Err(Refusal::Identity);
        }
        let mut c = self.shared.core.lock().unwrap();
        if c.status.state != State::Disarmed || c.stop {
            return Err(Refusal::State);
        }
        if c.status.in_flight.is_some() || c.pending.is_some() {
            return Err(Refusal::Busy);
        }
        c.advance()?;
        c.safe = safe;
        c.status.desired = Some(desired);
        c.fresh = Some(Instant::now());
        Ok(())
    }
    /// Called with a coherent authority view, after any logical command/tick.
    /// Only bounded copies occur under the output mutex; the sink never holds it.
    pub fn publish(&self, authority: &Authority) -> Result<(), Refusal> {
        let frame = Frame::synthetic(authority);
        let mut c = self.shared.core.lock().unwrap();
        c.expire(Instant::now());
        self.shared.wake.notify_one();
        if c.stop || matches!(c.status.state, State::Fault(_) | State::Stopped) {
            return Err(Refusal::State);
        }
        let previous = c.status.desired.as_ref().ok_or(Refusal::Identity)?;
        if !frame.same_configuration(previous) {
            return Err(Refusal::Identity);
        }
        if frame.revision < previous.revision
            || (frame.revision == previous.revision && frame != *previous)
        {
            return Err(Refusal::Revision);
        }
        c.advance()?;
        c.status.desired = Some(frame.clone());
        c.fresh = Some(Instant::now());
        if c.status.state == State::Armed {
            c.pending = Some(Submission {
                generation: c.status.generation,
                safe: false,
                frame,
            });
        }
        self.shared.wake.notify_one();
        Ok(())
    }
    pub fn arm(&self) -> Result<(), Refusal> {
        let mut c = self.shared.core.lock().unwrap();
        if c.stop || c.status.state != State::Disarmed {
            return Err(Refusal::State);
        }
        if c.safe.is_none() {
            return Err(Refusal::Policy);
        }
        if c.pending.is_some() || c.status.in_flight.is_some() {
            return Err(Refusal::Busy);
        }
        if c.fresh
            .is_none_or(|t| Instant::now() >= t + c.limits.stale_after)
        {
            return Err(Refusal::Stale);
        }
        c.advance()?;
        c.status.state = State::Armed;
        c.pending = Some(Submission {
            generation: c.status.generation,
            safe: false,
            frame: c.status.desired.clone().ok_or(Refusal::Identity)?,
        });
        self.shared.wake.notify_one();
        Ok(())
    }
    pub fn disarm(&self) -> Result<(), Refusal> {
        let mut c = self.shared.core.lock().unwrap();
        if c.stop || matches!(c.status.state, State::Fault(_) | State::Stopped) {
            return Err(Refusal::State);
        }
        if c.status.state == State::Armed {
            c.advance()?;
            c.status.state = State::Disarmed;
            c.queue_safe();
            self.shared.wake.notify_one();
        }
        Ok(())
    }
    /// Explicit acknowledgment only after the backend has returned. Drops policy and
    /// freshness: configure with a reviewed policy, then arm explicitly again.
    pub fn recover(&self) -> Result<(), Refusal> {
        let mut c = self.shared.core.lock().unwrap();
        if c.stop || !matches!(c.status.state, State::Fault(_)) {
            return Err(Refusal::State);
        }
        if c.status.in_flight.is_some() || c.pending.is_some() {
            return Err(Refusal::Busy);
        }
        c.advance()?;
        c.status.state = State::Disarmed;
        c.safe = None;
        c.fresh = None;
        c.status.desired = None;
        Ok(())
    }
    /// Joins the owned thread. At most one existing operation plus one safe operation
    /// and one rate interval, provided the sink honors its deadline. No thread killing.
    pub fn shutdown(&mut self) -> Status {
        {
            let mut c = self.shared.core.lock().unwrap();
            if !c.stop {
                if c.status.state == State::Armed {
                    c.status.state = State::Disarmed;
                    c.queue_safe();
                }
                c.stop = true;
                self.shared.wake.notify_one();
            }
        }
        if let Some(thread) = self.thread.take() {
            thread.join().expect("output worker internal panic");
        }
        self.status()
    }
}
impl Drop for OutputWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{FixtureSpec, Id, PatchSpec, SyntheticMode, VERSION};
    fn frame() -> Frame {
        Frame::synthetic(
            &Authority::new(
                Patch::validate(PatchSpec {
                    version: VERSION,
                    patch_revision: 1,
                    fixtures: vec![FixtureSpec::synthetic(
                        Id::new("f").unwrap(),
                        "",
                        SyntheticMode::Dimmer,
                        1,
                    )],
                })
                .unwrap(),
                "11111111-1111-4111-8111-111111111111",
                1,
            )
            .unwrap(),
        )
    }
    #[test]
    fn deterministic_rate_stale_and_safe_once() {
        let now = Instant::now();
        let mut c = Core::new(Limits::default(), now);
        let frame = frame();
        c.safe = Some(frame.clone());
        c.status.desired = Some(frame.clone());
        c.fresh = Some(now);
        c.status.state = State::Armed;
        c.pending = Some(Submission {
            generation: 1,
            safe: false,
            frame,
        });
        let job = c.take(now).unwrap();
        c.complete(job.clone(), Ok(()));
        c.pending = Some(job);
        assert!(c.take(now + Duration::from_millis(24)).is_none());
        let job = c.take(now + Duration::from_millis(25)).unwrap();
        c.complete(job, Ok(()));
        assert!(c.take(now + Duration::from_millis(499)).is_none());
        let safe = c.take(now + Duration::from_millis(500)).unwrap();
        assert!(safe.safe);
        assert_eq!(c.status.state, State::Fault(Fault::StaleSource));
        c.complete(safe, Ok(()));
        assert!(c.take(now + Duration::from_secs(20)).is_none());
        assert_eq!(c.status.state, State::Fault(Fault::StaleSource));
    }
    #[test]
    fn late_success_is_timeout_not_submission() {
        let now = Instant::now();
        assert_eq!(completion_result(Ok(()), now, now), Err(Fault::Timeout));
        assert_eq!(
            completion_result(Ok(()), now, now + Duration::from_millis(1)),
            Ok(())
        );
        let mut c = Core::new(Limits::default(), now);
        let job = Submission {
            generation: 1,
            safe: false,
            frame: frame(),
        };
        c.complete(job, completion_result(Ok(()), now, now));
        assert_eq!(c.status.state, State::Fault(Fault::Timeout));
        assert!(c.status.submitted.is_none());
    }
    #[test]
    fn malformed_limits_refused_and_generation_does_not_wrap() {
        assert!(
            !Limits {
                interval: Duration::ZERO,
                ..Limits::default()
            }
            .valid()
        );
        assert!(
            !Limits {
                timeout: Duration::from_secs(2),
                ..Limits::default()
            }
            .valid()
        );
        assert!(
            !Limits {
                stale_after: Duration::from_secs(11),
                ..Limits::default()
            }
            .valid()
        );
        let mut c = Core::new(Limits::default(), Instant::now());
        c.status.generation = u64::MAX;
        assert_eq!(c.advance(), Err(Refusal::Exhausted));
    }
}
