//! Independent GP04-local:1 client. No device discovery, capture or provider DSP.
use crate::analysis::{Analyzer, Calibrator, Snapshot, WINDOW};
use crate::replay::AuxFrame;
use crate::show::{Director, Program, Scene};
use crate::wire::{counter, strict_json};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const PAYLOAD: usize = 6280;
pub const MAX_SOURCES: usize = 8;
pub const MAX_PAYLOAD: usize = 12040;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub contract: String,
    pub version: u32,
    pub subscription: String,
    pub source_epoch: String,
    pub stream: u32,
    pub sample_rate: u32,
    pub first_frame: String,
    pub sources: Vec<String>,
    pub inputs: Vec<String>,
    pub tap: String,
    pub map_revision: String,
    pub calibration_revision: String,
    pub clock: String,
}
/// Exact configured semantic-source binding, independently validated by the consumer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    pub inputs: Vec<String>,
}
impl Binding {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !matches!(self.version, 2 | 3)
            || !(4..=MAX_SOURCES).contains(&self.inputs.len())
            || (self.version == 2 && (self.inputs.len() != 4 || !self.sources.is_empty()))
            || (self.version == 3
                && (self.sources.len() != self.inputs.len()
                    || self.sources[..4] != ["kick", "bass", "guitar-1", "guitar-2"]
                    || self.sources.iter().enumerate().any(|(i, s)| {
                        s.is_empty()
                            || s.len() > 32
                            || !s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                            || self.sources[..i].contains(s)
                    })))
        {
            return Err("analysis binding version");
        }
        for (i, id) in self.inputs.iter().enumerate() {
            let index = id
                .strip_prefix("input-")
                .and_then(|s| s.parse::<u16>().ok());
            if !index.is_some_and(|n| n > 0 && *id == format!("input-{n:02}"))
                || self.inputs[..i].contains(id)
            {
                return Err("analysis binding inputs");
            }
        }
        Ok(())
    }
}
impl Descriptor {
    pub fn decode(bytes: &[u8]) -> Result<Self, &'static str> {
        Self::decode_expected(bytes, None)
    }
    pub fn decode_configured(bytes: &[u8], binding: &Binding) -> Result<Self, &'static str> {
        binding.validate()?;
        Self::decode_expected(bytes, Some(binding))
    }
    fn decode_expected(bytes: &[u8], binding: Option<&Binding>) -> Result<Self, &'static str> {
        let d: Self = serde_json::from_value(strict_json(bytes)?).map_err(|_| "descriptor")?;
        let version = binding.map_or(1, |b| b.version);
        let subscription = if version == 3 {
            "lux.aux.v3"
        } else if binding.is_some() {
            "lux.aux.v2"
        } else {
            "lux.aux.v1"
        };
        let inputs = binding.map(|b| b.inputs.clone()).unwrap_or_else(|| {
            ["input-01", "input-02", "input-03", "input-04"]
                .map(String::from)
                .to_vec()
        });
        if d.contract != "C-ANALYSIS"
            || d.version != version
            || d.subscription != subscription
            || d.stream != 3
            || d.sample_rate != 48000
            || d.tap != "raw-pre-fader"
            || d.clock != "linux-clock-monotonic-ms"
            || d.sources
                != binding
                    .filter(|b| b.version == 3)
                    .map(|b| b.sources.clone())
                    .unwrap_or_else(|| {
                        ["kick", "bass", "guitar-1", "guitar-2"]
                            .map(String::from)
                            .to_vec()
                    })
            || d.inputs != inputs
        {
            return Err("descriptor_identity");
        }
        for s in [
            &d.source_epoch,
            &d.first_frame,
            &d.map_revision,
            &d.calibration_revision,
        ] {
            counter::parse(s)?;
        }
        if counter::parse(&d.source_epoch)? == 0
            || counter::parse(&d.map_revision)? == 0
            || counter::parse(&d.calibration_revision)? == 0
            || !counter::parse(&d.first_frame)?.is_multiple_of(48)
        {
            return Err("descriptor_origin");
        }
        Ok(d)
    }
}
fn u64be(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().expect("checked fixed slice"))
}
fn u32be(b: &[u8]) -> u32 {
    u32::from_be_bytes(b.try_into().expect("checked fixed slice"))
}
pub struct Window {
    pub first: u64,
    pub sequence: u32,
    pub oldest_ms: u64,
    pub losses: u64,
    pub pcm: [AuxFrame; WINDOW],
    pub all_pcm: [[f32; MAX_SOURCES]; WINDOW],
}
pub fn decode_window(d: &Descriptor, b: &[u8], now_ms: u64) -> Result<Window, &'static str> {
    let channels = d.inputs.len();
    if !(4..=MAX_SOURCES).contains(&channels) {
        return Err("source budget");
    }
    let packet_bytes = 48 + channels * 144;
    if b.len() != 40 + packet_bytes * 10 || &b[..4] != b"GAW1" || b[36..40] != [0; 4] {
        return Err("window_header");
    }
    let oldest_ms = u64be(&b[28..36]);
    if now_ms.checked_sub(oldest_ms).is_none_or(|age| age > 100) {
        return Err("age");
    }
    if u64be(&b[12..20]) != counter::parse(&d.map_revision)?
        || u64be(&b[20..28]) != counter::parse(&d.calibration_revision)?
    {
        return Err("mapping");
    }
    let mut w = Window {
        first: 0,
        sequence: 0,
        oldest_ms,
        losses: u64be(&b[4..12]),
        pcm: [[0.; 4]; WINDOW],
        all_pcm: [[0.; MAX_SOURCES]; WINDOW],
    };
    for (n, p) in b[40..].chunks_exact(packet_bytes).enumerate() {
        if &p[..4] != b"GPA1"
            || p[4..12] != [1, 1, 0, channels as u8, 0, 48, 0, 0]
            || u32be(&p[12..16]) != 3
            || u64be(&p[16..24]) != counter::parse(&d.source_epoch)?
            || u32be(&p[36..40]) != 48000
            || p[40..48] != [0; 8]
        {
            return Err("packet_identity");
        }
        let first = u64be(&p[24..32]);
        let seq = u32be(&p[32..36]);
        if n == 0 {
            w.first = first;
            w.sequence = seq;
        }
        if first != w.first.checked_add(n as u64 * 48).ok_or("frame_overflow")?
            || seq
                != w.sequence
                    .checked_add(n as u32)
                    .ok_or("sequence_overflow")?
        {
            return Err("packet_order");
        }
        for (i, s) in p[48..].chunks_exact(3).enumerate() {
            let v =
                ((i32::from(s[0]) << 24) | (i32::from(s[1]) << 16) | (i32::from(s[2]) << 8)) >> 8;
            let value = v as f32 / 8388608.;
            w.all_pcm[n * 48 + i / channels][i % channels] = value;
            if i % channels < 4 {
                w.pcm[n * 48 + i / channels][i % channels] = value;
            }
        }
    }
    let offset = w
        .first
        .checked_sub(counter::parse(&d.first_frame)?)
        .ok_or("frame_origin")?;
    if !offset.is_multiple_of(480) || offset / 48 != u64::from(w.sequence) {
        return Err("frame_origin");
    }
    w.first.checked_add(480).ok_or("frame_overflow")?;
    w.sequence.checked_add(10).ok_or("sequence_overflow")?;
    Ok(w)
}
/// Linux same-host acquisition clock; no audio endpoint access.
pub fn monotonic_ms() -> io::Result<u64> {
    let mut t = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: writable timespec and a supported clock ID.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut t) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(t.tv_sec as u64 * 1000 + t.tv_nsec as u64 / 1_000_000)
}
/// Cloneable owner-side state; temporal interpretation is never shared with the transport.
#[derive(Clone)]
pub struct AnalysisState {
    pub descriptor: Option<Descriptor>,
    pub state: &'static str,
    pub reason: Option<&'static str>,
    pub losses: u64,
    last_range: Option<(u64, u64)>,
    expected: Option<(u64, u32)>,
    oldest: Option<u64>,
    calibration: Option<Calibrator>,
    calibration_windows: u32,
    calibration_seen: [bool; MAX_SOURCES],
    all_rms: Option<[f32; MAX_SOURCES]>,
    calibration_start: u64,
    analyzer: Option<Analyzer>,
    analyzed_frames: u32,
    snapshot: Option<Snapshot>,
    music: Option<Box<crate::musical::SourceMusicalAnalyzer<MAX_SOURCES>>>,
    director: Director,
    pub proposal: Option<i32>,
    pub generation: u64,
    admitted: bool,
    last_now: Option<u64>,
}
impl Default for AnalysisState {
    fn default() -> Self {
        Self {
            descriptor: None,
            state: "absent",
            reason: Some("provider_absent"),
            losses: 0,
            last_range: None,
            expected: None,
            oldest: None,
            calibration: None,
            calibration_windows: 0,
            calibration_seen: [false; MAX_SOURCES],
            all_rms: None,
            calibration_start: 0,
            analyzer: None,
            analyzed_frames: 0,
            snapshot: None,
            music: None,
            director: Director::new(Program::Atmospheric),
            proposal: None,
            generation: 0,
            admitted: false,
            last_now: None,
        }
    }
}
impl AnalysisState {
    pub fn enable_music(&mut self) {
        let count = self.descriptor.as_ref().map_or(4, |d| d.inputs.len());
        let weights = std::array::from_fn(|i| {
            if i >= count || i == 0 {
                0.0
            } else if i == 1 {
                0.35
            } else {
                1.0
            }
        });
        self.music = Some(Box::new(
            crate::musical::SourceMusicalAnalyzer::with_weights(weights).expect("bounded roles"),
        ));
    }
    pub fn music_enabled(&self) -> bool {
        self.music.is_some()
    }
    pub fn attach(&mut self, d: Descriptor) {
        self.invalidate("reattach");
        self.descriptor = Some(d);
        if self.music.is_some() {
            self.enable_music();
        }
        self.admitted = true;
        self.last_now = None;
        self.state = "invalid";
        self.expected = None;
        self.oldest = None;
        self.last_range = None;
        self.calibration_windows = 0;
    }
    pub fn invalidate(&mut self, reason: &'static str) {
        self.state = "invalid";
        self.admitted = false;
        self.reason = Some(reason);
        self.calibration = None;
        self.analyzer = None;
        self.snapshot = None;
        self.all_rms = None;
        if let Some(music) = &mut self.music {
            music.reset();
        }
        self.proposal = None;
        self.generation = self.generation.saturating_add(1);
        self.director = Director::new(Program::Atmospheric);
    }
    pub fn absent(&mut self) {
        self.invalidate("provider_absent");
        self.state = "absent";
        self.descriptor = None;
        self.expected = None;
        self.oldest = None;
        self.last_range = None;
        self.calibration_windows = 0;
    }
    pub fn calibrate(&mut self, phase: &str, now: u64) -> Result<(), &'static str> {
        if !self.admitted
            || self.descriptor.is_none()
            || self
                .oldest
                .is_none_or(|t| now.checked_sub(t).is_none_or(|a| a > 100))
        {
            return Err("analysis_unavailable");
        }
        match phase {
            "start" => {
                self.invalidate("fresh_calibration");
                self.admitted = true;
                self.calibration = Some(Calibrator::default());
                self.calibration_windows = 0;
                self.calibration_seen = [false; MAX_SOURCES];
                self.calibration_start = now;
                self.state = "calibrating";
            }
            "finish" => {
                if self.state != "calibrating"
                    || self.calibration_windows < 50
                    || !self.calibration_seen[..self.descriptor.as_ref().unwrap().inputs.len()]
                        .iter()
                        .all(|v| *v)
                    || now.saturating_sub(self.calibration_start) > 10000
                {
                    return Err("calibration_not_ready");
                }
                let c = self.calibration.take().ok_or("calibration")?.finish();
                self.analyzer = Some(Analyzer::new(c));
                self.analyzed_frames = 0;
                self.state = "settling";
                self.reason = None;
            }
            _ => return Err("calibration_phase"),
        }
        Ok(())
    }
    pub fn ingest(&mut self, b: &[u8], now: u64) -> Result<(), &'static str> {
        let result = self.ingest_inner(b, now);
        if let Err(reason) = result {
            self.invalidate(reason);
        }
        result
    }
    fn ingest_inner(&mut self, b: &[u8], now: u64) -> Result<(), &'static str> {
        if !self.admitted {
            return Err("reattach_required");
        }
        if self.last_now.is_some_and(|last| now < last) {
            return Err("clock_regression");
        }
        self.last_now = Some(now);
        let d = self.descriptor.as_ref().ok_or("not_attached")?;
        let w = decode_window(d, b, now)?;
        if self
            .expected
            .is_some_and(|(f, s)| f != w.first || s != w.sequence)
        {
            return Err("gap_overlap_reorder");
        }
        if self.oldest.is_some_and(|t| w.oldest_ms < t) {
            return Err("clock_regression");
        }
        if w.losses != self.losses && self.expected.is_some() {
            self.losses = w.losses;
            return Err("lost_windows");
        }
        self.losses = w.losses;
        self.expected = Some((w.first + 480, w.sequence + 10));
        self.oldest = Some(w.oldest_ms);
        self.last_range = Some((w.first, w.first + 480));
        if let Some(c) = &mut self.calibration {
            if now.saturating_sub(self.calibration_start) > 10000 {
                return Err("calibration_expired");
            }
            c.observe(&w.pcm);
            self.calibration_windows += 1;
            for (s, seen) in self.calibration_seen.iter_mut().enumerate() {
                *seen |= w.all_pcm.iter().any(|f| f[s].abs() >= 0.0001);
            }
        }
        if let Some(a) = &mut self.analyzer {
            let mut snap = a.process(self.analyzed_frames, &w.pcm)?;
            snap.reliable &= w
                .all_pcm
                .iter()
                .filter(|f| f.iter().any(|s| s.abs() >= 0.999))
                .take(3)
                .count()
                <= 2;
            self.all_rms = Some(std::array::from_fn(|c| {
                (w.all_pcm.iter().map(|f| f[c] * f[c]).sum::<f32>() / WINDOW as f32).sqrt()
            }));
            self.analyzed_frames = self
                .analyzed_frames
                .checked_add(480)
                .ok_or("analysis_frame_overflow")?;
            let musical = self.music.as_mut().map(|m| m.process(&w.all_pcm, w.first));
            let decision = self.director.update(snap.at, snap.features(), false);
            self.proposal = decision.input_available.then_some(match decision.scene {
                Scene::Calm => 250,
                Scene::Drive => 500,
                Scene::Intense => 750,
            });
            // Harmonic evidence only nudges the existing bounded intensity proposal.
            // It grants no authority and bypasses no Hold, cap, freshness or manual rule.
            if snap.reliable {
                let nudge = musical.map(musical_intensity_nudge).unwrap_or(0);
                self.proposal = self.proposal.map(|value| (value + nudge).clamp(0, 1000));
            }
            self.state = if snap.reliable { "ready" } else { "settling" };
            self.snapshot = Some(snap);
        }
        Ok(())
    }
    pub fn expire(&mut self, now: u64) {
        if self
            .oldest
            .is_some_and(|t| now.checked_sub(t).is_none_or(|a| a > 100))
            && self.state != "stale"
        {
            self.invalidate("age");
            self.state = "stale";
        }
    }
    pub fn ready(&self) -> bool {
        self.state == "ready" && self.proposal.is_some()
    }
    pub fn scene_evidence(&self) -> crate::scenes::Evidence {
        let snap = self.snapshot;
        let music = self.music.as_ref().map(|m| m.snapshot());
        crate::scenes::Evidence {
            available: self.ready(),
            energy: snap.map_or(0, |s| (s.energy.clamp(0., 1.) * 1000.).round() as i32),
            density: snap.map_or(0, |s| {
                (s.kick_density / 8. * 1000.).clamp(0., 1000.).round() as i32
            }),
            confidence: if self.ready() { 1000 } else { 0 },
            section_id: music.and_then(crate::scenes::section_identity),
            harmonic_relation: music
                .and_then(|m| m.movement)
                .map(|m| format!("{:?}", m.kind).to_lowercase()),
        }
    }
    pub fn status(&self, now: u64) -> Value {
        let mut status = json!({"analysis_version":"lux.analysis.v1","calibration_generation":self.analyzer.as_ref().map(|_|self.generation.to_string()),"calibration_windows":self.calibration_windows,"source_epoch":self.descriptor.as_ref().map(|d|&d.source_epoch),"map_revision":self.descriptor.as_ref().map(|d|&d.map_revision),"calibration_revision":self.descriptor.as_ref().map(|d|&d.calibration_revision),"sources":self.descriptor.as_ref().map(|d|&d.sources),"state":self.state,"reason":self.reason,"source_identity":self.descriptor,"source_window_range":self.last_range.map(|(a,b)|json!({"first_frame":a.to_string(),"end_frame_exclusive":b.to_string()})),"source_age_ms":self.oldest.and_then(|t|now.checked_sub(t)),"losses":self.losses.to_string(),"confidence":if self.ready(){1000}else{0},"rms_millionths":self.all_rms.map(|r|r[..self.descriptor.as_ref().map_or(4, |d| d.inputs.len())].iter().map(|v|(v*1000000.).round() as u32).collect::<Vec<_>>()),"energy_millionths":self.snapshot.map(|s|(s.energy*1000000.).round() as u32),"beat":null,"downbeat":null,"harmony":null,"proposal":self.proposal,"generation":self.generation.to_string()});
        if let Some(music) = &self.music {
            status["analysis_version"] = json!("lux.analysis.v2");
            status["musical"] = musical_status(music.snapshot(), self.state);
        }
        status
    }
}
/// Key-dependent harmonic evidence cannot exceed the owner's modest ±50 proposal nudge.
fn musical_intensity_nudge(snapshot: crate::musical::MusicalSnapshot) -> i32 {
    if !snapshot.ready || snapshot.key.is_none() {
        return 0;
    }
    snapshot.movement.map_or(0, |movement| {
        ((movement.tension.clamp(0., 1.) - movement.resolution.clamp(0., 1.)) * 50.).round() as i32
    })
}

#[cfg(test)]
mod musical_policy_tests {
    use super::musical_intensity_nudge;
    use crate::musical::{HarmonicMovement, KeyEstimate, Mode, MovementKind, MusicalSnapshot};
    #[test]
    fn harmonic_nudge_is_bounded_and_requires_ready_key_evidence() {
        let mut snapshot = MusicalSnapshot {
            ready: true,
            key: Some(KeyEstimate {
                tonic: 0,
                mode: Mode::Major,
                camelot_number: 8,
                camelot_letter: 'B',
                confidence: 0.8,
            }),
            movement: Some(HarmonicMovement {
                fifths_steps: 1,
                kind: MovementKind::FifthsForward,
                tension: 1.,
                resolution: 0.,
            }),
            ..MusicalSnapshot::default()
        };
        assert_eq!(musical_intensity_nudge(snapshot), 50);
        snapshot.movement.as_mut().unwrap().resolution = 1.;
        snapshot.movement.as_mut().unwrap().tension = 0.;
        assert_eq!(musical_intensity_nudge(snapshot), -50);
        snapshot.key = None;
        assert_eq!(musical_intensity_nudge(snapshot), 0);
        snapshot.ready = false;
        assert_eq!(musical_intensity_nudge(snapshot), 0);
    }
}

/// Integer, bounded public estimates. An unavailable source never exposes retained music.
fn musical_status(m: crate::musical::MusicalSnapshot, source_state: &str) -> Value {
    use crate::musical::{Mode, MovementKind, SectionLabel};
    let tracking = source_state == "ready" && m.ready;
    let state = if source_state == "stale" {
        "stale"
    } else if tracking {
        "tracking"
    } else {
        "unknown"
    };
    let unit = |v: f32| (v.clamp(0., 1.) * 1000.).round() as u32;
    let mode = |v: Mode| match v {
        Mode::Major => "major",
        Mode::Minor => "minor",
    };
    json!({
        "state":state,
        "source_frame":tracking.then(||m.source_frame.to_string()),
        "expression":tracking.then(|| { let mut e = serde_json::to_value(m.expression).expect("expression"); e["source_frame"] = json!(m.expression.source_frame.to_string()); e }),
        "prediction":m.prediction.filter(|_| tracking).map(|c|json!({"root":c.root,"quality":mode(c.quality),"confidence":unit(c.confidence)})),
        "chroma":if tracking {m.chroma.map(unit)} else {[0;12]},
        "chord":m.chord.filter(|_|tracking).map(|c|json!({"root":c.root,"quality":mode(c.quality),"confidence":unit(c.confidence)})),
        "key":m.key.filter(|_|tracking).map(|k|json!({"tonic":k.tonic,"mode":mode(k.mode),"camelot_number":k.camelot_number,"camelot_letter":k.camelot_letter.to_string(),"confidence":unit(k.confidence)})),
        "movement":m.movement.filter(|_|tracking).map(|v|json!({"fifths_steps":v.fifths_steps,"kind":match v.kind {MovementKind::Stationary=>"stationary",MovementKind::FifthsForward=>"fifths_forward",MovementKind::FifthsBackward=>"fifths_backward",MovementKind::Other=>"other"},"tension":unit(v.tension),"resolution":unit(v.resolution)})),
        "section":m.section.filter(|_|tracking).map(|v|json!({"id":v.id,"novelty":unit(v.novelty),"recurrence":unit(v.recurrence),"label":match v.label {SectionLabel::Unknown=>"unknown",SectionLabel::VerseCandidate=>"verse_candidate",SectionLabel::ChorusCandidate=>"chorus_candidate",SectionLabel::BridgeCandidate=>"bridge_candidate"},"confidence":unit(v.confidence)}))
    })
}
struct Inbox {
    descriptor: Option<Descriptor>,
    windows: std::collections::VecDeque<Vec<u8>>,
    disconnected: bool,
    dropped: bool,
}
pub struct Client {
    reconnect: Arc<AtomicBool>,
    inbox: Arc<Mutex<Inbox>>,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Client {
    pub fn start(socket: &Path) -> io::Result<Self> {
        Self::start_expected(socket, None)
    }
    pub fn start_configured(socket: &Path, binding: Binding) -> io::Result<Self> {
        binding.validate().map_err(io::Error::other)?;
        Self::start_expected(socket, Some(binding))
    }
    fn start_expected(socket: &Path, binding: Option<Binding>) -> io::Result<Self> {
        if !socket.is_absolute() {
            return Err(io::Error::other("absolute analysis socket required"));
        }
        crate::local_service::private_directory(
            socket.parent().ok_or_else(|| io::Error::other("parent"))?,
        )?;
        let path = socket.to_owned();
        let reconnect = Arc::new(AtomicBool::new(false));
        let reset = reconnect.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let inbox = Arc::new(Mutex::new(Inbox {
            descriptor: None,
            windows: std::collections::VecDeque::with_capacity(2),
            disconnected: false,
            dropped: false,
        }));
        let queue = inbox.clone();
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                let connection = (|| -> io::Result<()> {
                    reset.store(false, Ordering::Release);
                    let mut stream = bounded_connect(&path, &worker_stop)?;
                    crate::local_service::same_uid(&stream)?;
                    stream.set_read_timeout(Some(Duration::from_millis(100)))?;
                    stream.set_write_timeout(Some(Duration::from_millis(100)))?;
                    let request: &[u8] = if binding.as_ref().is_some_and(|b| b.version == 3) {
                        br#"{"subscription":"lux.aux.v3","version":3}"#
                    } else if binding.is_some() {
                        br#"{"subscription":"lux.aux.v2","version":2}"#
                    } else {
                        br#"{"subscription":"lux.aux.v1","version":1}"#
                    };
                    stream.write_all(&(request.len() as u32).to_be_bytes())?;
                    stream.write_all(request)?;
                    let bytes = client_frame(&mut stream, 4096)?;
                    let d = if let Some(binding) = &binding {
                        Descriptor::decode_configured(&bytes, binding)
                    } else {
                        Descriptor::decode(&bytes)
                    }
                    .map_err(io::Error::other)?;
                    {
                        let mut q = queue.lock().map_err(|_| io::Error::other("poison"))?;
                        q.descriptor = Some(d);
                        q.windows.clear();
                        q.disconnected = false;
                    }
                    while !worker_stop.load(Ordering::Acquire) && !reset.load(Ordering::Acquire) {
                        let frame = client_frame(&mut stream, MAX_PAYLOAD)?;
                        let mut q = queue.lock().map_err(|_| io::Error::other("poison"))?;
                        if q.windows.len() == 2 {
                            q.dropped = true;
                            return Err(io::Error::other("client_queue_loss"));
                        }
                        q.windows.push_back(frame);
                    }
                    Ok(())
                })();
                if connection.is_err()
                    && let Ok(mut q) = queue.lock()
                {
                    q.disconnected = true;
                    q.descriptor = None;
                    q.windows.clear();
                }
                for _ in 0..25 {
                    if worker_stop.load(Ordering::Acquire) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });
        Ok(Self {
            reconnect,
            inbox,
            stop,
            worker: Some(worker),
        })
    }
    pub fn drain(&self, state: &mut AnalysisState, now: u64) {
        self.drain_with(state, now, AnalysisState::ingest);
    }
    fn drain_with(
        &self,
        state: &mut AnalysisState,
        now: u64,
        mut ingest: impl FnMut(&mut AnalysisState, &[u8], u64) -> Result<(), &'static str>,
    ) {
        // Move at most two owned windows; expensive analysis never holds the
        // transport mutex. The reader can fill its existing bounded queue while
        // this batch is analyzed; neither the queue nor acquisition age grows.
        let batch = self.inbox.lock().ok().map(|mut q| {
            (
                std::mem::take(&mut q.disconnected),
                std::mem::take(&mut q.dropped),
                q.descriptor.take(),
                [q.windows.pop_front(), q.windows.pop_front()],
            )
        });
        if let Some((disconnected, dropped, descriptor, windows)) = batch {
            if disconnected {
                state.absent();
            }
            if dropped {
                state.invalidate("client_queue_loss");
            }
            if let Some(d) = descriptor {
                state.attach(d);
            }
            for bytes in windows.into_iter().flatten() {
                if ingest(state, &bytes, now).is_err() {
                    self.reconnect.store(true, Ordering::Release);
                    break;
                }
            }
            // A receiver loss or new connection during DSP invalidates the
            // processed batch before the owner can apply an AUTO proposal. Keep
            // those events queued for their normal ordered handling next drain.
            if let Ok(q) = self.inbox.lock() {
                if q.disconnected {
                    state.absent();
                } else if q.dropped {
                    state.invalidate("client_queue_loss");
                } else if q.descriptor.is_some() {
                    state.invalidate("reattach");
                }
            }
        }
        if state.state == "stale" {
            self.reconnect.store(true, Ordering::Release);
        }
        if self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            state.absent();
        }
        state.expire(now);
    }
}
#[cfg(test)]
mod inbox_tests {
    use super::*;
    fn descriptor() -> Descriptor {
        let corpus: Value =
            serde_json::from_str(include_str!("../tests/fixtures/lx05/v1/e09.json")).unwrap();
        Descriptor::decode(&serde_json::to_vec(&corpus["descriptor"]).unwrap()).unwrap()
    }
    fn client() -> Client {
        Client {
            reconnect: Arc::new(AtomicBool::new(false)),
            inbox: Arc::new(Mutex::new(Inbox {
                descriptor: Some(descriptor()),
                windows: [vec![1], vec![2]].into_iter().collect(),
                disconnected: true,
                dropped: true,
            })),
            stop: Arc::new(AtomicBool::new(false)),
            worker: None,
        }
    }
    #[test]
    fn receiver_can_enqueue_while_bounded_batch_is_analyzed() {
        let client = client();
        let mut state = AnalysisState::default();
        let mut observed = Vec::new();
        client.drain_with(&mut state, 1000, |state, bytes, _| {
            // Pending reset events precede the descriptor, so attachment is admitted.
            assert!(state.admitted);
            assert_eq!(state.reason, Some("reattach"));
            let mut inbox = client
                .inbox
                .try_lock()
                .expect("DSP must release transport mutex");
            inbox.windows.push_back(vec![3]);
            observed.push(bytes[0]);
            Ok(())
        });
        assert_eq!(observed, [1, 2]);
        assert_eq!(client.inbox.lock().unwrap().windows.len(), 2);
    }
    #[test]
    fn loss_disconnect_or_reattach_during_analysis_cannot_leave_ready_proposal() {
        for event in ["loss", "disconnect", "reattach"] {
            let client = client();
            let mut state = AnalysisState::default();
            state.enable_music();
            client.drain_with(&mut state, 1000, |state, _, _| {
                state.state = "ready";
                state.proposal = Some(500);
                let mut inbox = client.inbox.try_lock().unwrap();
                match event {
                    "loss" => inbox.dropped = true,
                    "disconnect" => inbox.disconnected = true,
                    _ => inbox.descriptor = Some(descriptor()),
                }
                Ok(())
            });
            assert!(!state.ready());
            assert!(state.proposal.is_none());
            assert!(state.status(1000)["musical"]["source_frame"].is_null());
            let inbox = client.inbox.lock().unwrap();
            assert!(inbox.dropped || inbox.disconnected || inbox.descriptor.is_some());
        }
    }
}

fn bounded_connect(path: &Path, stop: &AtomicBool) -> io::Result<UnixStream> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::ffi::OsStrExt,
    };
    let bytes = path.as_os_str().as_bytes();
    // SAFETY: sockaddr_un consists of plain integer/byte fields, zero is valid.
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err(io::Error::other("analysis socket path"));
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    for (target, byte) in address.sun_path.iter_mut().zip(bytes) {
        *target = *byte as libc::c_char;
    }
    // SAFETY: socket has no pointer arguments; returned descriptor immediately receives RAII ownership.
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd is newly owned and is transferred exactly once.
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let len =
        (std::mem::offset_of!(libc::sockaddr_un, sun_path) + bytes.len() + 1) as libc::socklen_t;
    // SAFETY: address points to initialized sockaddr_un and length covers its bounded pathname.
    let result = unsafe {
        libc::connect(
            stream.as_raw_fd(),
            (&address as *const libc::sockaddr_un).cast(),
            len,
        )
    };
    if result != 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error);
        }
        let deadline = std::time::Instant::now() + Duration::from_millis(100);
        loop {
            if stop.load(Ordering::Acquire) || std::time::Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "analysis connect deadline",
                ));
            }
            let mut poll = libc::pollfd {
                fd: stream.as_raw_fd(),
                events: libc::POLLOUT,
                revents: 0,
            };
            // SAFETY: one valid writable pollfd; finite10ms cancellation interval.
            let ready = unsafe { libc::poll(&mut poll, 1, 10) };
            if ready < 0 {
                return Err(io::Error::last_os_error());
            }
            if ready > 0 {
                let mut error = 0i32;
                let mut len = std::mem::size_of::<i32>() as libc::socklen_t;
                // SAFETY: valid owned fd and correctly sized writable integer storage.
                if unsafe {
                    libc::getsockopt(
                        stream.as_raw_fd(),
                        libc::SOL_SOCKET,
                        libc::SO_ERROR,
                        (&mut error as *mut i32).cast(),
                        &mut len,
                    )
                } != 0
                {
                    return Err(io::Error::last_os_error());
                }
                if error != 0 {
                    return Err(io::Error::from_raw_os_error(error));
                }
                break;
            }
        }
    }
    stream.set_nonblocking(false)?;
    Ok(stream)
}
fn client_frame(stream: &mut UnixStream, max: usize) -> io::Result<Vec<u8>> {
    let start = std::time::Instant::now();
    fn read(stream: &mut UnixStream, part: &mut [u8], start: std::time::Instant) -> io::Result<()> {
        let mut offset = 0;
        while offset < part.len() {
            let remaining = Duration::from_millis(100)
                .checked_sub(start.elapsed())
                .ok_or_else(|| io::Error::other("analysis frame age"))?;
            stream.set_read_timeout(Some(remaining))?;
            let n = stream.read(&mut part[offset..])?;
            if n == 0 {
                return Err(io::Error::other("analysis eof"));
            }
            offset += n;
        }
        Ok(())
    }
    let mut h = [0; 4];
    read(stream, &mut h, start)?;
    let len = u32be(&h) as usize;
    if len == 0 || len > max {
        return Err(io::Error::other("analysis frame capacity"));
    }
    let mut b = vec![0; len];
    read(stream, &mut b, start)?;
    Ok(b)
}
impl Drop for Client {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
