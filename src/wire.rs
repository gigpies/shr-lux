//! Strict C-LIGHT:1 owner codec and scoped null-output service. No hardware I/O.
use crate::authority::{Authority, Command, ReleasePreview};
use crate::fixture::{Error, Id, PatchSpec, Value};
use crate::lighting_contract::{MAX_DEPTH, MAX_MESSAGE_BYTES, StoredKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SCOPE: &str = "lighting-control";
pub const WRITER_HISTORY: usize = 1024;
pub const LEASE_TICKS: u64 = 200;
pub const CACHE: usize = 64;
/// Canonical u64 decimal strings. Numeric JSON, leading zeros and overflow refuse.
pub mod counter {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        parse(&s).map_err(serde::de::Error::custom)
    }
    pub fn parse(s: &str) -> Result<u64, &'static str> {
        if s.is_empty()
            || (s.len() > 1 && s.starts_with('0'))
            || !s.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("range");
        }
        s.parse().map_err(|_| "range")
    }
}

/// A visitor rejects duplicate keys before serde_json can collapse them.
struct Strict(Json);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("bounded JSON")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(Json::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Strict, E> {
                Err(E::custom("integer required"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Json::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Strict, E> {
                self.visit_unit()
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = Vec::new();
                while let Some(Strict(v)) = a.next_element()? {
                    if values.len() >= 1024 {
                        return Err(serde::de::Error::custom("capacity"));
                    }
                    values.push(v);
                }
                Ok(Strict(Json::Array(values)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if values.len() >= 128 || values.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate/capacity"));
                    }
                    let Strict(v) = a.next_value()?;
                    values.insert(k, v);
                }
                Ok(Strict(Json::Object(values)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
pub fn strict_json(bytes: &[u8]) -> Result<Json, &'static str> {
    strict_json_bounded(bytes, MAX_MESSAGE_BYTES)
}
pub(crate) fn strict_json_bounded(bytes: &[u8], limit: usize) -> Result<Json, &'static str> {
    if bytes.len() > limit {
        return Err("capacity");
    }
    // Preflight nesting without allocating or recursing; ignore escaped quoted characters.
    let (mut depth, mut quoted, mut escape) = (0usize, false, false);
    for &b in bytes {
        if quoted {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > MAX_DEPTH {
                        return Err("depth");
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    serde_json::from_slice::<Strict>(bytes)
        .map(|v| v.0)
        .map_err(|_| "malformed")
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Envelope {
    pub contract: String,
    pub version: u16,
    pub show_id: String,
    pub module: String,
    pub epoch: String,
    pub writer: Option<Id>,
    pub lease: Option<String>,
    pub request_id: Option<String>,
    pub expected_revision: Option<String>,
    pub kind: String,
    pub body: Json,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Touch {
        values: Vec<Value>,
    },
    ClearToHold,
    Record {
        kind: Store,
        id: Id,
    },
    Update {
        kind: Store,
        id: Id,
    },
    ApplyPalette {
        id: Id,
    },
    Go {
        cue: Id,
        playback: Id,
    },
    Level {
        playback: Id,
        level: i32,
    },
    Off {
        playback: Id,
    },
    Master {
        level: i32,
    },
    FixtureMaster {
        fixture: Id,
        level: i32,
    },
    Blackout {
        enabled: bool,
    },
    Mode {
        mode: String,
    },
    ReplacePatch {
        patch: PatchSpec,
    },
    ReleasePreview {
        values: Vec<Value>,
    },
    ReleaseCommit {
        token: Json,
    },
    ReleaseCancel,
    Checkpoint,
    AnalysisCalibrate {
        phase: String,
    },
    SceneMapReplace {
        map: crate::scenes::ShowMap,
    },
    ScenePreview {
        scene_id: Id,
    },
    SceneGrant {
        fixtures: Vec<Id>,
        intensity_cap: i32,
        color_cap: i32,
        ttl_ms: u64,
    },
    AnalysisGrant {
        fixtures: Vec<Id>,
        cap: i32,
        ttl_ms: u64,
    },
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Store {
    Cue,
    Palette,
}
impl From<Store> for StoredKind {
    fn from(v: Store) -> Self {
        match v {
            Store::Cue => Self::Cue,
            Store::Palette => Self::Palette,
        }
    }
}
impl Action {
    fn command(self) -> Result<Command, &'static str> {
        Ok(match self {
            Self::Touch { values } => Command::Touch(values),
            Self::ClearToHold => Command::ClearToHold,
            Self::Record { kind, id } => Command::Record {
                kind: kind.into(),
                id,
            },
            Self::Update { kind, id } => Command::Update {
                kind: kind.into(),
                id,
            },
            Self::ApplyPalette { id } => Command::ApplyPalette(id),
            Self::Go { cue, playback } => Command::Go { cue, playback },
            Self::Level { playback, level } => Command::Level { playback, level },
            Self::Off { playback } => Command::Off(playback),
            Self::Master { level } => Command::Master(level),
            Self::FixtureMaster { fixture, level } => Command::FixtureMaster { fixture, level },
            Self::Blackout { enabled } => Command::Blackout(enabled),
            Self::Mode { mode } => Command::Mode(match mode.as_str() {
                "manual" => crate::authority::Mode::Manual,
                "assist" => crate::authority::Mode::Assist,
                "auto" => crate::authority::Mode::Auto,
                _ => return Err("unavailable"),
            }),
            Self::ReplacePatch { patch } => Command::ReplacePatch(patch),
            Self::ReleasePreview { .. }
            | Self::ReleaseCommit { .. }
            | Self::ReleaseCancel
            | Self::AnalysisCalibrate { .. }
            | Self::AnalysisGrant { .. }
            | Self::SceneMapReplace { .. }
            | Self::ScenePreview { .. }
            | Self::SceneGrant { .. }
            | Self::Checkpoint => return Err("unavailable"),
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scoped {
    scope: String,
}
#[derive(Clone)]
struct Session {
    lease: u64,
    expiry: u64,
    highwater: u64,
    cache: VecDeque<(u64, std::sync::Arc<Json>, std::sync::Arc<Json>)>,
}
#[derive(Clone)]
struct AnalysisGrant {
    writer: Id,
    lease: u64,
    issue_revision: u64,
    patch: u64,
    generation: u64,
    expiry: u64,
    fixtures: Vec<Id>,
    cap: i32,
    color_cap: Option<i32>,
}
/// One scope, one active writer, bounded never-forgotten identities per epoch.
#[derive(Clone)]
pub struct Service {
    engine: Authority,
    scenes: Option<crate::scenes::Director>,
    scene_generation: Option<u64>,
    analysis: Option<crate::analysis_subscription::AnalysisState>,
    analysis_binding: Option<crate::analysis_subscription::Binding>,
    analysis_grant: Option<AnalysisGrant>,
    last_analysis_step: Option<u64>,
    analysis_now: u64,
    auto_held_reason: Option<String>,
    analysis_emergency: bool,
    tick: u64,
    sequence: u64,
    next_lease: u64,
    sessions: BTreeMap<Id, Session>,
    retired: BTreeSet<Id>,
    denied_grants: BTreeMap<Id, (std::sync::Arc<Json>, std::sync::Arc<Json>)>,
    timed: bool,
    preview: Option<PreviewEntry>,
    store: Option<std::sync::Arc<crate::recovery::CheckpointStore>>,
    epoch_owner: Option<std::sync::Arc<crate::recovery::CheckpointStore>>,
    durability: crate::lighting_contract::Durability,
}
impl Service {
    pub fn new(engine: Authority) -> Result<Self, &'static str> {
        let service = Self {
            engine,
            scenes: None,
            scene_generation: None,
            analysis: None,
            analysis_binding: None,
            analysis_grant: None,
            last_analysis_step: None,
            analysis_now: 0,
            auto_held_reason: None,
            analysis_emergency: false,
            tick: 0,
            sequence: 0,
            next_lease: 0,
            sessions: BTreeMap::new(),
            retired: BTreeSet::new(),
            denied_grants: BTreeMap::new(),
            timed: false,
            preview: None,
            store: None,
            epoch_owner: None,
            durability: crate::lighting_contract::Durability::Volatile,
        };
        service.pages()?;
        Ok(service)
    }
    pub fn timed(engine: Authority) -> Result<Self, &'static str> {
        let mut service = Self::new(engine)?;
        service.timed = true;
        service.pages()?;
        Ok(service)
    }
    /// Epoch reservation is already durable and the owner lock remains retained.
    /// Volatile scenes still retain a durably reserved never-reused listener epoch.
    pub fn reserved(
        store: std::sync::Arc<crate::recovery::CheckpointStore>,
        patch: crate::fixture::Patch,
        timed: bool,
    ) -> Result<Self, &'static str> {
        let engine = Authority::new(patch, store.show_id(), store.epoch()).map_err(error_reason)?;
        let mut service = if timed {
            Self::timed(engine)?
        } else {
            Self::new(engine)?
        };
        service.epoch_owner = Some(store);
        Ok(service)
    }
    pub fn recover(
        store: std::sync::Arc<crate::recovery::CheckpointStore>,
        fallback_patch: crate::fixture::Patch,
    ) -> Result<Self, &'static str> {
        let saved = store.load().map_err(|_| "recovery")?;
        let engine = match &saved {
            Some(state) => Authority::restore_durable(state, store.show_id(), store.epoch())
                .map_err(error_reason)?,
            None => Authority::new(fallback_patch, store.show_id(), store.epoch())
                .map_err(error_reason)?,
        };
        let mut service = Self::timed(engine)?;
        service.durability = if saved.is_some() {
            crate::lighting_contract::Durability::Checkpointed
        } else {
            crate::lighting_contract::Durability::Volatile
        };
        service.store = Some(store);
        service.pages()?;
        Ok(service)
    }
    pub fn enable_analysis(&mut self) -> Result<(), &'static str> {
        if self.analysis_binding.is_some() {
            return Err("configured analysis identity pinned");
        }
        self.enable_analysis_inner(None)
    }
    pub fn enable_configured_analysis(
        &mut self,
        binding: crate::analysis_subscription::Binding,
    ) -> Result<(), &'static str> {
        binding.validate()?;
        if self.analysis.is_some() {
            return Err("analysis already enabled");
        }
        self.enable_analysis_inner(Some(binding))
    }
    /// Explicit additive estimator mode; existing LX05 output remains unchanged.
    pub fn enable_musical_analysis(&mut self) -> Result<(), &'static str> {
        let mut next = self.clone();
        let analysis = next.analysis.as_mut().ok_or("analysis required")?;
        if analysis.descriptor.is_some() || analysis.music_enabled() {
            return Err("musical mode is startup-only");
        }
        analysis.enable_music();
        next.pages()?;
        *self = next;
        Ok(())
    }
    pub fn enable_scene_director(&mut self) -> Result<(), &'static str> {
        if !self.analysis.as_ref().is_some_and(|a| a.music_enabled()) || self.scenes.is_some() {
            return Err("scenes require musical analysis startup");
        }
        let mut next = self.clone();
        let owner = next.store.as_ref().or(next.epoch_owner.as_ref());
        let map = owner
            .map(|s| s.load_scene_map(next.engine.patch()))
            .transpose()
            .map_err(|_| "scene map recovery")?
            .flatten()
            .unwrap_or_else(|| crate::scenes::ShowMap::defaults(next.engine.patch()));
        map.validate(next.engine.patch())?;
        next.engine.enable_scenes();
        next.scenes = Some(crate::scenes::Director::new(map));
        next.pages()?;
        *self = next;
        Ok(())
    }
    pub fn scene_status(&self) -> Json {
        self.scenes.as_ref().map_or(Json::Null,|director|{let mut s=serde_json::to_value(&director.status).expect("scene status");s["grant"]=self.analysis_grant.as_ref().filter(|g|g.color_cap.is_some()).map_or(Json::Null,|g|json!({"writer":g.writer,"lease":g.lease.to_string(),"issue_revision":g.issue_revision.to_string(),"patch_revision":g.patch.to_string(),"analysis_generation":g.generation.to_string(),"expiry_tick":g.expiry.to_string(),"fixtures":g.fixtures,"intensity_cap":g.cap,"color_cap":g.color_cap}));s["automatic_layer"]=self.engine.scene_inventory();s})
    }
    fn analysis_schema(&self) -> &'static str {
        if self.scenes.is_some() {
            if self.analysis_binding.is_some() {
                "lx06-scenes-v4"
            } else {
                "lx06-scenes-v3"
            }
        } else if self.analysis.as_ref().is_some_and(|a| a.music_enabled()) {
            if self.analysis_binding.is_some() {
                "lx05-music-v4"
            } else {
                "lx05-music-v3"
            }
        } else if self
            .analysis_binding
            .as_ref()
            .is_some_and(|b| b.version == 3)
        {
            "lx05-v3"
        } else if self.analysis_binding.is_some() {
            "lx05-v2"
        } else {
            "lx05-v1"
        }
    }
    fn enable_analysis_inner(
        &mut self,
        binding: Option<crate::analysis_subscription::Binding>,
    ) -> Result<(), &'static str> {
        let mut next = self.clone();
        next.analysis_binding = binding;
        next.engine.enable_analysis();
        next.analysis = Some(crate::analysis_subscription::AnalysisState::default());
        next.pages()?;
        *self = next;
        Ok(())
    }
    fn revoke_analysis(&mut self, reason: &str) -> Result<(), &'static str> {
        if self.analysis_grant.take().is_some() {
            self.engine.freeze_auto(reason).map_err(error_reason)?;
            self.auto_held_reason = Some(reason.into());
        }
        Ok(())
    }
    /// Called by the independent owner clock even with no control clients connected.
    pub fn poll_analysis(
        &mut self,
        client: Option<&crate::analysis_subscription::Client>,
        now_ms: u64,
        tick: u64,
    ) -> Result<(), &'static str> {
        self.poll_analysis_clock(
            client,
            now_ms,
            tick,
            crate::analysis_subscription::monotonic_ms,
        )
    }
    fn poll_analysis_clock(
        &mut self,
        client: Option<&crate::analysis_subscription::Client>,
        now_ms: u64,
        tick: u64,
        mut clock: impl FnMut() -> std::io::Result<u64>,
    ) -> Result<(), &'static str> {
        if self.analysis.is_none() {
            return self.advance(tick);
        }
        let mut baseline = self.clone();
        baseline.analysis_emergency = true;
        baseline.advance_inner(tick)?;
        let mut candidate = baseline.clone();
        candidate.analysis_emergency = false;
        candidate.poll_analysis_inner(client, now_ms, tick, &mut clock)?;
        if candidate.pages().is_err() {
            baseline.analysis_emergency = true;
            if let Some(a) = &mut baseline.analysis {
                a.invalidate("snapshot_capacity");
            }
            baseline.engine.set_analysis_proposal(None);
            baseline.revoke_analysis("snapshot_capacity")?;
            if let Some(d) = &mut baseline.scenes {
                d.reset_source();
            }
            baseline.last_analysis_step = Some(tick);
            baseline.analysis_now = clock().map_err(|_| "clock")?;
            baseline.pages()?;
            *self = baseline;
            return Ok(());
        }
        // Encoding a near-budget snapshot also takes time: never commit a newly stale step.
        let final_now = clock().map_err(|_| "clock")?;
        if final_now < candidate.analysis_now {
            candidate.engine = baseline.engine.clone();
            candidate.analysis_emergency = true;
            if let Some(a) = &mut candidate.analysis {
                a.invalidate("clock_regression");
            }
            candidate.engine.set_analysis_proposal(None);
            candidate.revoke_analysis("clock_regression")?;
            if let Some(d) = &mut candidate.scenes {
                d.reset_source();
            }
            candidate.pages()?;
            *self = candidate;
            return Ok(());
        }
        candidate.analysis_now = final_now;
        if let Some(a) = &mut candidate.analysis {
            a.expire(final_now);
        }
        candidate
            .engine
            .set_analysis_proposal(candidate.analysis.as_ref().and_then(|a| a.proposal));
        if candidate.analysis_grant.is_some()
            && candidate.analysis.as_ref().is_none_or(|a| !a.ready())
        {
            candidate.engine = baseline.engine;
            candidate.analysis_emergency = true;
            candidate.engine.set_analysis_proposal(None);
            candidate.revoke_analysis("analysis_age_after_processing")?;
            if let Some(d) = &mut candidate.scenes {
                d.reset_source();
            }
            candidate.pages()?;
        }
        *self = candidate;
        Ok(())
    }
    fn poll_analysis_inner(
        &mut self,
        client: Option<&crate::analysis_subscription::Client>,
        now_ms: u64,
        tick: u64,
        clock: &mut impl FnMut() -> std::io::Result<u64>,
    ) -> Result<(), &'static str> {
        self.tick = tick;
        self.analysis_now = now_ms;
        let Some(analysis) = &mut self.analysis else {
            return Ok(());
        };
        if let Some(client) = client {
            client.drain(analysis, now_ms);
        } else {
            analysis.expire(now_ms);
        }
        // DSP may consume the last milliseconds of freshness. Re-sample acquisition age now.
        let after = clock().map_err(|_| "clock")?;
        self.analysis_now = after;
        if after < now_ms {
            analysis.invalidate("clock_regression");
        }
        analysis.expire(self.analysis_now);
        self.engine.set_analysis_proposal(analysis.proposal);
        let invalid = self.analysis_grant.as_ref().and_then(|g| {
            if !analysis.ready() || g.generation != analysis.generation {
                Some("analysis_loss_or_change")
            } else if self.engine.mode() != crate::authority::Mode::Auto {
                Some("mode")
            } else if g.patch != self.engine.patch().revision() {
                Some("patch")
            } else if g.expiry <= tick {
                Some("expiry")
            } else if !self
                .sessions
                .get(&g.writer)
                .is_some_and(|s| s.lease == g.lease && s.expiry > tick)
            {
                Some("lighting_lease")
            } else {
                None
            }
        });
        if let Some(reason) = invalid {
            self.revoke_analysis(reason)?;
        }
        if let Some(director) = &mut self.scenes {
            let a = self.analysis.as_ref().ok_or("analysis_unavailable")?;
            if !a.ready() || self.scene_generation != Some(a.generation) {
                director.reset_source();
                self.scene_generation = Some(a.generation);
            }
            let actual = self
                .engine
                .snapshot()
                .fixtures
                .into_iter()
                .flat_map(|f| {
                    f.attributes.into_iter().map(move |a| Value {
                        fixture: f.fixture.clone(),
                        attribute: a.attribute,
                        value: a.resolved,
                    })
                })
                .collect::<Vec<_>>();
            director.step(
                a.scene_evidence(),
                self.analysis_now,
                self.engine.patch(),
                &actual,
            )?;
        }
        if self.last_analysis_step != Some(tick) {
            self.last_analysis_step = Some(tick);
            if let Some(g) = &self.analysis_grant {
                let proposal = self
                    .analysis
                    .as_ref()
                    .and_then(|a| a.proposal)
                    .ok_or("analysis_unavailable")?;
                let revision = self.engine.revision();
                let status = self
                    .analysis
                    .as_ref()
                    .ok_or("analysis_unavailable")?
                    .status(self.analysis_now);
                if let Some(color_cap) = g.color_cap {
                    let director = self.scenes.as_ref().ok_or("unavailable")?;
                    let provenance = json!({"map_revision":director.status.map.revision.to_string(),"scene_id":director.status.current,"analysis_generation":g.generation.to_string(),"section_id":director.status.evidence.section_id,"source_identity":status["source_identity"],"source_window_range":status["source_window_range"]});
                    self.engine
                        .step_scene(
                            &director.status.intended_values,
                            &g.fixtures,
                            g.cap,
                            color_cap,
                            &provenance,
                        )
                        .map_err(error_reason)?;
                } else {
                    self.engine
                        .step_auto(&g.fixtures, g.cap, proposal)
                        .map_err(error_reason)?;
                    self.engine.set_auto_provenance(&g.fixtures,&json!({"analysis_version":status["analysis_version"],"source_identity":status["source_identity"],"source_window_range":status["source_window_range"],"calibration_generation":status["calibration_generation"],"confidence":status["confidence"]}));
                }
                if self.engine.revision() != revision {
                    self.preview = None;
                    self.durability = crate::lighting_contract::Durability::Volatile;
                }
            }
        }
        Ok(())
    }
    pub fn analysis_state_mut(
        &mut self,
    ) -> Option<&mut crate::analysis_subscription::AnalysisState> {
        self.analysis.as_mut()
    }
    pub fn analysis_status(&self) -> Json {
        let mut status = self
            .analysis
            .as_ref()
            .map_or(Json::Null, |a| a.status(self.analysis_now));
        if !status.is_null() {
            status["grant"]=self.analysis_grant.as_ref().filter(|g|g.color_cap.is_none()).map_or(Json::Null,|g|json!({"writer":g.writer,"lease":g.lease.to_string(),"issue_revision":g.issue_revision.to_string(),"patch_revision":g.patch.to_string(),"analysis_generation":g.generation.to_string(),"expiry_tick":g.expiry.to_string(),"fixtures":g.fixtures,"cap":g.cap}));
            status["automatic_layer"] = self.engine.auto_inventory();
            status["held_reason"] = json!(self.auto_held_reason);
            if let Some(binding) = &self.analysis_binding {
                status["analysis_binding"] = json!(binding);
            }
        }
        status
    }
    pub fn engine(&self) -> &Authority {
        &self.engine
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn advance(&mut self, tick: u64) -> Result<(), &'static str> {
        let mut next = self.clone();
        next.advance_inner(tick)?;
        next.pages()?;
        *self = next;
        Ok(())
    }
    fn advance_inner(&mut self, tick: u64) -> Result<(), &'static str> {
        if tick < self.tick {
            return Err("clock");
        }
        let revision = self.engine.revision();
        self.tick = tick;
        if self.timed {
            self.engine.advance(tick).map_err(error_reason)?;
            if self.engine.revision() != revision {
                self.durability = crate::lighting_contract::Durability::Volatile;
            }
        }
        let expired = self
            .sessions
            .iter()
            .filter(|(_, s)| s.expiry <= tick)
            .map(|(id, _)| id.clone())
            .collect::<Vec<_>>();
        for id in expired {
            self.sessions.remove(&id);
            self.retired.insert(id);
        }
        if self.preview.as_ref().is_some_and(|p| {
            !self.engine.preview_is_current(&p.preview)
                || !self
                    .sessions
                    .get(&p.writer)
                    .is_some_and(|s| s.lease == p.lease)
        }) {
            self.preview = None;
        }
        Ok(())
    }
    fn reply(&self, kind: &str, reason: Option<&str>, request: Option<&str>, body: Json) -> Json {
        let s = self.engine.snapshot();
        json!({"contract":"C-LIGHT","version":1,"show_id":s.show_id,"module":"lighting","epoch":s.epoch.to_string(),"sequence":self.sequence.to_string(),"revision":s.revision.to_string(),"effective_tick":self.tick.to_string(),"writer":null,"lease":null,"expected_revision":null,"request_id":request,"kind":kind,"reason":reason,"body":body})
    }
    fn correlated(&self, mut reply: Json, request: &Envelope) -> Json {
        reply["writer"] = serde_json::to_value(&request.writer).unwrap_or(Json::Null);
        reply["lease"] = request
            .lease
            .as_ref()
            .filter(|s| counter::parse(s).is_ok())
            .map_or(Json::Null, |s| json!(s));
        reply["request_id"] = request
            .request_id
            .as_ref()
            .filter(|s| counter::parse(s).is_ok())
            .map_or(Json::Null, |s| json!(s));
        reply["expected_revision"] = request
            .expected_revision
            .as_ref()
            .filter(|s| counter::parse(s).is_ok())
            .map_or(Json::Null, |s| json!(s));
        reply
    }
    pub fn handle(&mut self, bytes: &[u8]) -> Vec<Json> {
        let context = strict_json(bytes)
            .ok()
            .and_then(|v| serde_json::from_value::<Envelope>(v).ok());
        match self.handle_inner(bytes) {
            Ok(v) => v,
            Err(reason) => {
                let response = self.reply(
                    if reason == "stale_revision" {
                        "conflict"
                    } else if reason == "busy" {
                        "busy"
                    } else {
                        "rejected"
                    },
                    Some(reason),
                    None,
                    Json::Null,
                );
                vec![
                    context
                        .as_ref()
                        .map_or(response.clone(), |e| self.correlated(response, e)),
                ]
            }
        }
    }
    fn handle_inner(&mut self, bytes: &[u8]) -> Result<Vec<Json>, &'static str> {
        let raw = strict_json(bytes)?;
        if raw.as_object().is_none_or(|o| o.len() != 11) {
            return Err("malformed");
        }
        let e: Envelope = serde_json::from_value(raw.clone()).map_err(|_| "malformed")?;
        let s = self.engine.snapshot();
        if e.contract != "C-LIGHT" || e.version != 1 {
            return Err("version");
        }
        if e.show_id != s.show_id {
            return Err("wrong_show");
        }
        if e.module != "lighting" {
            return Err("target");
        }
        if counter::parse(&e.epoch)? != s.epoch {
            return Err("epoch");
        }
        self.sequence = self.sequence.checked_add(1).ok_or("capacity")?;
        if matches!(
            e.kind.as_str(),
            "snapshot" | "capabilities" | "analysis_status"
        ) {
            if e.writer.is_some()
                || e.lease.is_some()
                || e.request_id.is_some()
                || e.expected_revision.is_some()
                || e.body != json!({})
            {
                return Err("malformed");
            }
            if e.kind == "analysis_status" {
                if self.analysis.is_none() {
                    return Err("unavailable");
                }
                let mut body =
                    json!({"wire_schema":self.analysis_schema(),"analysis":self.analysis_status()});
                if self.scenes.is_some() {
                    body["scenes"] = self.scene_status();
                }
                return Ok(vec![self.reply("analysis_status", None, None, body)]);
            }
            return self.pages();
        }
        let writer = e.writer.clone().ok_or("lease")?;
        if e.kind == "grant" {
            return self.grant_request(&e, writer, raw);
        }
        let lease = counter::parse(e.lease.as_deref().ok_or("lease")?)?;
        let session = self.sessions.get(&writer).ok_or("lease")?;
        if session.lease != lease || session.expiry <= self.tick {
            return Err("lease");
        }
        let id = counter::parse(e.request_id.as_deref().ok_or("malformed")?)?;
        if let Some((_, old, response)) = session.cache.iter().find(|(i, _, _)| *i == id) {
            return if old.as_ref() == &raw {
                Ok(vec![response.as_ref().clone()])
            } else {
                Err("reused_id")
            };
        }
        if id <= session.highwater {
            return Err("expired_id");
        }
        if session.highwater == 0 && id != 1 {
            return Err("reused_id");
        }
        let expected = counter::parse(e.expected_revision.as_deref().ok_or("malformed")?)?;
        let outcome = if expected != self.engine.revision() {
            Err("stale_revision")
        } else {
            self.apply_request(&e, &writer)
        };
        let response = match outcome {
            Ok(body) => self.reply("applied", None, e.request_id.as_deref(), body),
            Err(reason) => self.reply(
                if reason == "stale_revision" {
                    "conflict"
                } else {
                    "rejected"
                },
                Some(reason),
                e.request_id.as_deref(),
                Json::Null,
            ),
        };
        let response = self.correlated(response, &e);
        // Retire removes the session; its identity remains permanently unavailable this epoch.
        if let Some(session) = self.sessions.get_mut(&writer) {
            session.highwater = id;
            session
                .cache
                .push_back((id, raw.into(), response.clone().into()));
            if session.cache.len() > CACHE {
                session.cache.pop_front();
            }
        }
        Ok(vec![response])
    }
    fn grant_request(
        &mut self,
        e: &Envelope,
        writer: Id,
        raw: Json,
    ) -> Result<Vec<Json>, &'static str> {
        let body: Scoped = serde_json::from_value(e.body.clone()).map_err(|_| "malformed")?;
        if body.scope != SCOPE {
            return Err("scope");
        }
        if e.lease.is_some() {
            return Err("lease");
        }
        let id = counter::parse(e.request_id.as_deref().ok_or("malformed")?)?;
        let expected = counter::parse(e.expected_revision.as_deref().ok_or("malformed")?)?;
        if id != 1 {
            return Err("reused_id");
        }
        if let Some(session) = self.sessions.get(&writer) {
            if session.expiry <= self.tick {
                return Err("lease");
            }
            if let Some((_, old, response)) = session.cache.iter().find(|(i, _, _)| *i == 1) {
                return if old.as_ref() == &raw {
                    Ok(vec![response.as_ref().clone()])
                } else {
                    Err("reused_id")
                };
            }
            return Err("expired_id");
        }
        if let Some((old, response)) = self.denied_grants.get(&writer) {
            return if old.as_ref() == &raw {
                Ok(vec![response.as_ref().clone()])
            } else {
                Err("reused_id")
            };
        }
        if self.retired.contains(&writer) {
            return Err("lease");
        }
        if self.retired.len() + self.sessions.len() >= WRITER_HISTORY {
            return Err("capacity");
        }
        let outcome = (|| {
            if expected != self.engine.revision() {
                return Err("stale_revision");
            }
            if !self.sessions.is_empty() {
                return Err("busy");
            }
            let expiry = self.tick.checked_add(LEASE_TICKS).ok_or("capacity")?;
            let lease = self.next_lease.checked_add(1).ok_or("capacity")?;
            let mut candidate = self.clone();
            candidate.next_lease = lease;
            candidate.sessions.insert(
                writer.clone(),
                Session {
                    lease,
                    expiry,
                    highwater: 1,
                    cache: VecDeque::new(),
                },
            );
            candidate.pages()?;
            self.next_lease = candidate.next_lease;
            self.sessions = candidate.sessions;
            Ok(json!({"scope":SCOPE,"lease":lease.to_string(),"remaining_ms":2000}))
        })();
        let response = match outcome {
            Ok(body) => self.reply("applied", None, e.request_id.as_deref(), body),
            Err(reason) => self.reply(
                if reason == "stale_revision" {
                    "conflict"
                } else if reason == "busy" {
                    "busy"
                } else {
                    "rejected"
                },
                Some(reason),
                e.request_id.as_deref(),
                Json::Null,
            ),
        };
        let response = self.correlated(response, e);
        if let Some(session) = self.sessions.get_mut(&writer) {
            session
                .cache
                .push_back((1, raw.into(), response.clone().into()));
        } else {
            self.retired.insert(writer.clone());
            self.denied_grants
                .insert(writer, (raw.into(), response.clone().into()));
        }
        Ok(vec![response])
    }
    fn apply_request(&mut self, e: &Envelope, writer: &Id) -> Result<Json, &'static str> {
        if e.kind == "renew" || e.kind == "retire" {
            let body: Scoped = serde_json::from_value(e.body.clone()).map_err(|_| "malformed")?;
            if body.scope != SCOPE {
                return Err("scope");
            }
            if e.kind == "renew" {
                let expiry = self.tick.checked_add(LEASE_TICKS).ok_or("capacity")?;
                self.sessions.get_mut(writer).ok_or("lease")?.expiry = expiry;
                return Ok(json!({"remaining_ms":2000}));
            }
            if self.preview.as_ref().is_some_and(|p| &p.writer == writer) {
                self.preview = None;
            }
            self.sessions.remove(writer);
            self.retired.insert(writer.clone());
            return Ok(json!({"retired":true}));
        }
        if e.kind != "command" {
            return Err("unavailable");
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Body {
            scope: String,
            command: Json,
        }
        let body: Body = serde_json::from_value(e.body.clone()).map_err(|_| "malformed")?;
        if body.scope != SCOPE {
            return Err("scope");
        }
        let action: Action = serde_json::from_value(body.command).map_err(|_| "malformed")?;
        let mut candidate = self.clone();
        candidate.analysis_emergency = false;
        let mut result =
            json!({"application":"logical_static","output":"null_disarmed","physical":"unknown"});
        match action {
            Action::SceneMapReplace { map } => {
                let old = candidate.scenes.as_ref().ok_or("unavailable")?;
                if map.revision != old.status.map.revision.checked_add(1).ok_or("capacity")? {
                    return Err("scene_map_revision");
                }
                map.validate(candidate.engine.patch())?;
                candidate.revoke_analysis("scene_map_edit")?;
                candidate.scenes = Some(crate::scenes::Director::new(map.clone()));
                candidate.engine.bump_revision().map_err(error_reason)?;
                candidate.pages()?;
                let owner = candidate
                    .store
                    .as_ref()
                    .or(candidate.epoch_owner.as_ref())
                    .ok_or("durability")?;
                owner
                    .save_scene_map(&map, candidate.engine.patch())
                    .map_err(|_| "durability")?;
                result = json!({"application":"scene_map_replaced","map_revision":map.revision.to_string(),"physical":"unknown"});
            }
            Action::ScenePreview { scene_id } => {
                let d = candidate.scenes.as_ref().ok_or("unavailable")?;
                let values = d.status.map.values(&scene_id, candidate.engine.patch())?;
                let unsupported=candidate.engine.patch().fixtures().iter().map(|f|json!({"fixture":f.id,"attributes":if f.capability(crate::fixture::Attribute::Red).is_none(){vec!["red","green","blue","movement","strobe"]}else{vec!["movement","strobe"]}})).collect::<Vec<_>>();
                let snapshot = candidate.engine.snapshot();
                let actual=values.iter().map(|v|{let a=snapshot.fixtures.iter().find(|f|f.fixture==v.fixture).and_then(|f|f.attributes.iter().find(|a|a.attribute==v.attribute)).expect("validated preview target");json!({"fixture":v.fixture,"attribute":v.attribute,"value":a.resolved,"source":a.source})}).collect::<Vec<_>>();
                result = json!({"application":"scene_preview","scene_id":scene_id,"map_revision":d.status.map.revision.to_string(),"values":values,"actual":actual,"unsupported":unsupported,"physical":"unknown"});
            }
            Action::SceneGrant {
                fixtures,
                intensity_cap,
                color_cap,
                ttl_ms,
            } => {
                if candidate.scenes.is_none() {
                    return Err("unavailable");
                }
                let now = crate::analysis_subscription::monotonic_ms().map_err(|_| "clock")?;
                candidate.analysis_now = now;
                let a = candidate.analysis.as_mut().ok_or("unavailable")?;
                a.expire(now);
                if !a.ready() {
                    return Err("analysis_unavailable");
                }
                if ttl_ms == 0
                    || ttl_ms > 2000
                    || !ttl_ms.is_multiple_of(10)
                    || !(0..=1000).contains(&color_cap)
                {
                    return Err("range");
                }
                candidate
                    .engine
                    .validate_auto_targets(&fixtures, intensity_cap)
                    .map_err(error_reason)?;
                let generation = a.generation;
                let lease = candidate.sessions.get(writer).ok_or("lease")?.lease;
                candidate.revoke_analysis("replaced_grant")?;
                candidate.analysis_grant = Some(AnalysisGrant {
                    writer: writer.clone(),
                    lease,
                    issue_revision: candidate.engine.revision(),
                    patch: candidate.engine.patch().revision(),
                    generation,
                    expiry: candidate.tick.checked_add(ttl_ms / 10).ok_or("capacity")?,
                    fixtures,
                    cap: intensity_cap,
                    color_cap: Some(color_cap),
                });
                candidate.auto_held_reason = None;
                result = json!({"application":"scene_granted","map_revision":candidate.scenes.as_ref().ok_or("unavailable")?.status.map.revision.to_string(),"grant":candidate.scene_status()["grant"]});
            }
            Action::AnalysisCalibrate { phase } => {
                let now = crate::analysis_subscription::monotonic_ms().map_err(|_| "clock")?;
                candidate.analysis_now = now;
                candidate
                    .analysis
                    .as_mut()
                    .ok_or("unavailable")?
                    .calibrate(&phase, now)?;
                candidate.revoke_analysis("calibration")?;
                candidate.engine.set_analysis_proposal(None);
                result = json!({"analysis":candidate.analysis_status()});
            }
            Action::AnalysisGrant {
                fixtures,
                cap,
                ttl_ms,
            } => {
                let now = crate::analysis_subscription::monotonic_ms().map_err(|_| "clock")?;
                candidate.analysis_now = now;
                let a = candidate.analysis.as_mut().ok_or("unavailable")?;
                a.expire(now);
                if !a.ready() {
                    return Err("analysis_unavailable");
                }
                if ttl_ms == 0 || ttl_ms > 2000 || !ttl_ms.is_multiple_of(10) {
                    return Err("range");
                }
                candidate
                    .engine
                    .validate_auto_targets(&fixtures, cap)
                    .map_err(error_reason)?;
                let session = candidate.sessions.get(writer).ok_or("lease")?;
                let generation = a.generation;
                let lease = session.lease;
                candidate.revoke_analysis("replaced_grant")?;
                candidate
                    .engine
                    .replace_scene_intensity(&fixtures)
                    .map_err(error_reason)?;
                candidate.analysis_grant = Some(AnalysisGrant {
                    writer: writer.clone(),
                    lease,
                    issue_revision: candidate.engine.revision(),
                    patch: candidate.engine.patch().revision(),
                    generation,
                    expiry: candidate.tick.checked_add(ttl_ms / 10).ok_or("capacity")?,
                    fixtures,
                    cap,
                    color_cap: None,
                });
                candidate.auto_held_reason = None;
                result = json!({"analysis":candidate.analysis_status()});
            }
            Action::ReleasePreview { values } => {
                if !candidate.timed {
                    return Err("unavailable");
                }
                let preview = candidate
                    .engine
                    .preview_release(&values)
                    .map_err(error_reason)?;
                let lease = candidate.sessions.get(writer).ok_or("lease")?.lease;
                let nonce = preview_nonce()?;
                let payload = json!({"schema":"lx04-release-v1","nonce":nonce,"writer":writer,"lease":lease.to_string(),"scope":SCOPE,"preview":preview});
                result = json!({"token":payload,"application":"preview_only","physical":"unknown"});
                candidate.preview = Some(PreviewEntry {
                    writer: writer.clone(),
                    lease,
                    preview,
                    payload,
                });
            }
            Action::ReleaseCommit { token } => {
                if !candidate.timed {
                    return Err("unavailable");
                }
                let stored = candidate.preview.as_ref().ok_or("target")?;
                let lease = candidate.sessions.get(writer).ok_or("lease")?.lease;
                if &stored.writer != writer || stored.lease != lease || stored.payload != token {
                    return Err("lease");
                }
                candidate
                    .engine
                    .release_commit(&stored.preview)
                    .map_err(error_reason)?;
                candidate.preview = None;
                result = json!({"application":"logical_timed","transition":candidate.engine.transition_status(),"physical":"unknown"});
            }
            Action::ReleaseCancel => {
                if !candidate.timed {
                    return Err("unavailable");
                }
                candidate.preview = None;
                result = json!({"application":"preview_cancelled","physical":"unknown"});
            }
            Action::Checkpoint => {
                candidate.pages()?;
                let store = candidate.store.as_ref().ok_or("unavailable")?;
                let state = candidate.engine.durable_state();
                if store.checkpoint(&state).is_err() {
                    self.durability = crate::lighting_contract::Durability::Error;
                    return Err("durability");
                }
                candidate.durability = crate::lighting_contract::Durability::Checkpointed;
                result = json!({"application":"checkpointed","durability":"checkpointed","physical":"unknown"});
            }
            ordinary => {
                candidate
                    .engine
                    .execute(ordinary.command()?)
                    .map_err(error_reason)?;
                if candidate.engine.patch().revision() != self.engine.patch().revision()
                    && let Some(d) = &candidate.scenes
                {
                    d.status.map.validate(candidate.engine.patch())?;
                }
                candidate.preview = None;
                if candidate.analysis_grant.as_ref().is_some_and(|g| {
                    g.patch != candidate.engine.patch().revision()
                        || candidate.engine.mode() != crate::authority::Mode::Auto
                }) {
                    candidate.revoke_analysis("mode_or_patch")?;
                }
            }
        }
        if serde_json::to_vec(&result).map_err(|_| "capacity")?.len() > MAX_MESSAGE_BYTES - 4096 {
            return Err("capacity");
        }
        candidate.pages()?; // Coherent readable state is a transaction invariant.
        if candidate.engine.revision() != self.engine.revision() {
            candidate.durability = crate::lighting_contract::Durability::Volatile;
        }
        self.analysis_emergency = candidate.analysis_emergency;
        self.analysis = candidate.analysis;
        self.scenes = candidate.scenes;
        self.analysis_grant = candidate.analysis_grant;
        self.analysis_now = candidate.analysis_now;
        self.auto_held_reason = candidate.auto_held_reason;
        self.engine = candidate.engine;
        self.preview = candidate.preview;
        self.durability = candidate.durability;
        Ok(result)
    }
    pub fn inventory(&self) -> Result<Json, &'static str> {
        let snapshot = self.engine.snapshot();
        snapshot
            .validate_for_schema(
                self.engine.patch(),
                if self.analysis.is_some() {
                    self.analysis_schema()
                } else {
                    "lx03-v1"
                },
            )
            .map_err(error_reason)?;
        let mut inventory = json!({"snapshot":snapshot,"patch":{"version":1,"patch_revision":self.engine.patch().revision().to_string(),"fixtures":self.engine.patch().fixtures()},"groups":[{"id":"all","fixtures":self.engine.patch().fixtures().iter().map(|f|f.id.clone()).collect::<Vec<_>>()}],"mode":match self.engine.mode(){crate::authority::Mode::Manual=>"manual",crate::authority::Mode::Assist=>"assist",crate::authority::Mode::Auto=>"auto"},"master":self.engine.master(),"blackout":self.engine.blackout(),"output":"null_disarmed","physical":"unknown","limits":{"fixtures":32,"playbacks":8,"cues":32,"palettes":32,"targets":64,"writers":1,"writer_history":WRITER_HISTORY,"cached_responses":64,"pages":16,"message_bytes":65536,"depth":12,"lease_ms":2000,"renew_ms":500},"authority_inventory":self.engine.control_inventory(),"grants":self.sessions.iter().map(|(writer,session)|json!({"writer":writer,"scope":SCOPE,"remaining_ms":session.expiry.saturating_sub(self.tick)*10})).collect::<Vec<_>>(),"capability_metadata":self.engine.patch().fixtures().iter().map(|f|json!({"fixture":f.id,"attributes":f.capabilities.iter().map(|c|json!({"attribute":c.attribute,"unit":c.attribute.unit(),"min":c.min,"max":c.max,"step":1,"default":c.default})).collect::<Vec<_>>(),"coherent_groups":match f.mode {crate::fixture::SyntheticMode::Dimmer=>json!([]),crate::fixture::SyntheticMode::Rgb=>json!([["red","green","blue"]]),crate::fixture::SyntheticMode::RgbPosition=>json!([["red","green","blue"],["pan","tilt"]])}})).collect::<Vec<_>>(),"snapshot_budget":"16 encoded pages of at most 65536 bytes; overbudget edits refused atomically","scope":SCOPE,"applied_auto":"unavailable"});
        if self.timed {
            inventory["wire_schema"] = json!("lx04-v1");
            inventory["release"] = json!({"transition_ms":500,"preview_validity_ms":2000,"transition":self.engine.transition_status(),"preview_available":self.preview.is_some()});
        }
        if self.store.is_some() {
            inventory["wire_schema"] = json!("lx04-durable-v1");
            inventory["snapshot"]["durability"] =
                serde_json::to_value(self.durability).map_err(|_| "unavailable")?;
            inventory["checkpoint"] = json!({"available":true,"recovery":"current_intended_look_frozen_into_hold","active_transients_resumed":false});
        }
        if self.analysis.is_some() {
            inventory["wire_schema"] = json!(self.analysis_schema());
            inventory["analysis"] = self.analysis_status();
            inventory["applied_auto"] = json!("explicit_bounded_intensity_grant");
            if let Some(binding) = &self.analysis_binding {
                inventory["analysis_binding"] = json!(binding);
                validate_configured_analysis(&inventory)?;
            }
        }
        if self.scenes.is_some() {
            inventory["scenes"] = self.scene_status();
            inventory["applied_auto"] = json!("explicit_bounded_scene_grant");
        }
        Ok(inventory)
    }
    fn pages(&self) -> Result<Vec<Json>, &'static str> {
        let text = serde_json::to_string(&self.inventory()?).map_err(|_| "unavailable")?;
        // Chunks are UTF-8 text, never independent JSON. Assemble then decode once.
        let mut chunks = Vec::new();
        let mut offset = 0;
        while offset < text.len() {
            let mut end = (offset + 48_000).min(text.len());
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            // Escape-heavy labels must still fit the encoded envelope.
            while serde_json::to_vec(&text[offset..end])
                .map_err(|_| "unavailable")?
                .len()
                > 60_000
            {
                end = offset + (end - offset) / 2;
                while !text.is_char_boundary(end) {
                    end -= 1;
                }
            }
            chunks.push(text[offset..end].to_owned());
            offset = end;
            if chunks.len()
                > if self.analysis.is_some() && !self.analysis_emergency {
                    15
                } else {
                    16
                }
            {
                return Err("capacity");
            }
        }
        let count = chunks.len();
        chunks.into_iter().enumerate().map(|(index,chunk)|{
            let page=self.reply("snapshot",None,None,json!({"page":index,"page_count":count,"encoding":"json_utf8_chunks","chunk":chunk}));
            if serde_json::to_vec(&page).map_err(|_|"unavailable")?.len()>MAX_MESSAGE_BYTES {Err("capacity")} else {Ok(page)}
        }).collect()
    }
}
/// Validate current and retained configured identities independently: a retained
/// contribution can legitimately name an older source epoch/map than the live one.
pub fn validate_configured_analysis(inventory: &Json) -> Result<(), &'static str> {
    use crate::analysis_subscription::{Binding, Descriptor};
    if !matches!(
        inventory["wire_schema"].as_str(),
        Some(
            "lx05-v2"
                | "lx05-v3"
                | "lx05-music-v2"
                | "lx05-music-v4"
                | "lx06-scenes-v2"
                | "lx06-scenes-v4"
        )
    ) {
        return Err("configured wire schema");
    }
    let binding: Binding = serde_json::from_value(inventory["analysis_binding"].clone())
        .map_err(|_| "analysis binding")?;
    binding.validate()?;
    let status = &inventory["analysis"];
    let version = if inventory["wire_schema"] == "lx05-music-v2"
        || inventory["wire_schema"] == "lx06-scenes-v2"
        || inventory["wire_schema"] == "lx05-music-v4"
        || inventory["wire_schema"] == "lx06-scenes-v4"
    {
        "lux.analysis.v2"
    } else {
        "lux.analysis.v1"
    };
    if status["analysis_binding"] != inventory["analysis_binding"]
        || status["analysis_version"] != version
    {
        return Err("analysis binding status");
    }
    let validate = |value: &Json| -> Result<(), &'static str> {
        Descriptor::decode_configured(
            &serde_json::to_vec(value).map_err(|_| "descriptor")?,
            &binding,
        )
        .map(|_| ())
    };
    if !status["source_identity"].is_null() {
        validate(&status["source_identity"])?;
    }
    for value in status["automatic_layer"]["values"]
        .as_array()
        .ok_or("automatic layer")?
    {
        let provenance = &value["provenance"];
        if provenance.is_null() {
            if value["source"] == "mode-entry-continuity" {
                continue;
            }
            return Err("configured provenance required");
        }
        if provenance["analysis_version"] != version {
            return Err("analysis provenance version");
        }
        validate(&provenance["source_identity"])?;
    }
    Ok(())
}
fn error_reason(e: Error) -> &'static str {
    match e {
        Error::Range | Error::Default => "range",
        Error::Capacity => "capacity",
        Error::Id => "id",
        Error::Version => "version",
        Error::Unavailable => "unavailable",
        Error::Duplicate => "duplicate",
        Error::Group => "group",
        _ => "target",
    }
}

/// Test-only interoperability checker, not a public trusted inventory decoder.
#[cfg(test)]
#[derive(Default)]
struct PageAssembler {
    key: Option<(String, String, String, String, usize, u64)>,
    pages: BTreeMap<usize, String>,
}
#[cfg(test)]
impl PageAssembler {
    pub fn push(&mut self, page: &Json, receipt_tick: u64) -> Result<Option<Json>, &'static str> {
        let result = self.push_inner(page, receipt_tick);
        if result.is_err() {
            self.key = None;
            self.pages.clear();
        }
        result
    }
    fn push_inner(&mut self, page: &Json, tick: u64) -> Result<Option<Json>, &'static str> {
        let string = |key: &str| page[key].as_str().map(str::to_owned).ok_or("malformed");
        if page["kind"] != "snapshot" || page["contract"] != "C-LIGHT" || page["version"] != 1 {
            return Err("version");
        }
        let show = string("show_id")?;
        crate::lighting_contract::validate_show_id(&show).map_err(|_| "wrong_show")?;
        let epoch = string("epoch")?;
        counter::parse(&epoch)?;
        let revision = string("revision")?;
        counter::parse(&revision)?;
        let sequence = string("sequence")?;
        counter::parse(&sequence)?;
        let body = &page["body"];
        let count = body["page_count"].as_u64().ok_or("malformed")? as usize;
        let index = body["page"].as_u64().ok_or("malformed")? as usize;
        if count == 0 || count > 16 || index >= count || body["encoding"] != "json_utf8_chunks" {
            return Err("capacity");
        }
        let chunk = body["chunk"].as_str().ok_or("malformed")?;
        if chunk.len() > 60_000
            || serde_json::to_vec(page).map_err(|_| "malformed")?.len() > MAX_MESSAGE_BYTES
        {
            return Err("capacity");
        }
        if let Some((s, e, r, q, n, start)) = &self.key {
            if s != &show
                || e != &epoch
                || r != &revision
                || q != &sequence
                || *n != count
                || tick < *start
                || tick - *start > 200
            {
                return Err("stale_page");
            }
        } else {
            self.key = Some((show, epoch, revision, sequence, count, tick));
        }
        if self.pages.insert(index, chunk.into()).is_some() {
            return Err("duplicate");
        }
        if self.pages.len() != count {
            return Ok(None);
        }
        let text = self.pages.values().cloned().collect::<String>();
        // Aggregate is bounded by <=16 encoded pages, rather than single-message limit.
        let value = strict_json_bounded(text.as_bytes(), 16 * MAX_MESSAGE_BYTES)?;
        let (show, epoch, revision, _, _, _) = self.key.as_ref().ok_or("malformed")?;
        if value["snapshot"]["show_id"] != *show
            || value["snapshot"]["epoch"] != *epoch
            || value["snapshot"]["revision"] != *revision
        {
            return Err("identity");
        }
        let spec: PatchSpec =
            serde_json::from_value(value["patch"].clone()).map_err(|_| "malformed")?;
        let patch = crate::fixture::Patch::validate(spec).map_err(error_reason)?;
        let snapshot: crate::lighting_contract::Snapshot =
            serde_json::from_value(value["snapshot"].clone()).map_err(|_| "malformed")?;
        snapshot
            .validate_for_schema(&patch, value["wire_schema"].as_str().unwrap_or("lx03-v1"))
            .map_err(error_reason)?;
        if value["wire_schema"] == "lx05-v2"
            || value["wire_schema"] == "lx05-v3"
            || value["wire_schema"] == "lx05-music-v4"
        {
            validate_configured_analysis(&value)?;
        }
        self.key = None;
        self.pages.clear();
        Ok(Some(value))
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;
    fn service() -> Service {
        let patch = crate::fixture::Patch::validate(PatchSpec {
            version: 1,
            patch_revision: 1,
            fixtures: vec![],
        })
        .unwrap();
        Service::new(Authority::new(patch, "11111111-1111-4111-8111-111111111111", 9).unwrap())
            .unwrap()
    }
    #[test]
    fn assembled_identity_depth_patch_and_deadline_refuse() {
        let s = service();
        let pages = s.pages().unwrap();
        let page = &pages[0];
        assert_eq!(
            PageAssembler::default().push(page, 0).unwrap(),
            Some(s.inventory().unwrap())
        );
        for field in ["show_id", "epoch", "revision"] {
            let mut inventory = s.inventory().unwrap();
            inventory["snapshot"][field] = if field == "show_id" {
                json!("22222222-2222-4222-8222-222222222222")
            } else {
                json!("8")
            };
            let mut modified = page.clone();
            modified["body"]["chunk"] = json!(serde_json::to_string(&inventory).unwrap());
            assert_eq!(PageAssembler::default().push(&modified, 0), Err("identity"));
        }
        let mut nested = page.clone();
        nested["body"]["chunk"] = json!("[[[[[[[[[[[[[0]]]]]]]]]]]]]");
        assert_eq!(PageAssembler::default().push(&nested, 0), Err("depth"));
        let mut badpatch = s.inventory().unwrap();
        badpatch["patch"]["patch_revision"] = json!("2");
        let mut modified = page.clone();
        modified["body"]["chunk"] = json!(serde_json::to_string(&badpatch).unwrap());
        assert!(PageAssembler::default().push(&modified, 0).is_err());
        let mut p0 = page.clone();
        p0["body"]["page_count"] = json!(2);
        let mut p1 = p0.clone();
        p1["body"]["page"] = json!(1);
        let mut a = PageAssembler::default();
        assert!(a.push(&p0, 0).unwrap().is_none());
        assert_eq!(a.push(&p1, 201), Err("stale_page"));
        let mut a = PageAssembler::default();
        a.push(&p0, 0).unwrap();
        p1["revision"] = json!("2");
        assert_eq!(a.push(&p1, 1), Err("stale_page"));
    }
}

#[derive(Clone)]
struct PreviewEntry {
    writer: Id,
    lease: u64,
    preview: ReleasePreview,
    payload: Json,
}
fn preview_nonce() -> Result<String, &'static str> {
    let mut bytes = [0u8; 16];
    // SAFETY: valid16byte output buffer; nonblocking OS randomness, no device opened.
    let n = unsafe { libc::getrandom(bytes.as_mut_ptr().cast(), bytes.len(), libc::GRND_NONBLOCK) };
    if n != bytes.len() as isize {
        return Err("unavailable");
    }
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod durability_page_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn real_checkpointed_and_error_pages_roundtrip_with_schema_aware_validation() {
        let path = std::env::temp_dir().join(format!("lux-page-durable-{}", std::process::id()));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let show = "11111111-1111-4111-8111-111111111111";
        let store =
            std::sync::Arc::new(crate::recovery::CheckpointStore::open(&path, show).unwrap());
        let patch = crate::fixture::Patch::validate(PatchSpec {
            version: 1,
            patch_revision: 1,
            fixtures: vec![],
        })
        .unwrap();
        let mut s = Service::recover(store.clone(), patch).unwrap();
        let envelope = |kind: &str, id: u64, lease: Option<&str>, body: Json| json!({"contract":"C-LIGHT","version":1,"show_id":show,"module":"lighting","epoch":"1","writer":"writer","lease":lease,"request_id":id.to_string(),"expected_revision":"0","kind":kind,"body":body});
        s.handle(&serde_json::to_vec(&envelope("grant", 1, None, json!({"scope":SCOPE}))).unwrap());
        let checkpoint = |id| {
            envelope(
                "command",
                id,
                Some("1"),
                json!({"scope":SCOPE,"command":{"action":"checkpoint"}}),
            )
        };
        assert_eq!(
            s.handle(&serde_json::to_vec(&checkpoint(2)).unwrap())[0]["kind"],
            "applied"
        );
        for expected in ["checkpointed", "error"] {
            if expected == "error" {
                std::fs::write(path.join("lighting-checkpoint.json"), b"{\"version\":99}").unwrap();
                assert_eq!(
                    s.handle(&serde_json::to_vec(&checkpoint(3)).unwrap())[0]["reason"],
                    "durability"
                );
            }
            let inventory = s.inventory().unwrap();
            assert_eq!(inventory["snapshot"]["durability"], expected);
            let pages = s.pages().unwrap();
            let mut assembler = PageAssembler::default();
            let mut result = None;
            for page in pages {
                result = assembler.push(&page, 0).unwrap().or(result);
            }
            assert_eq!(result, Some(inventory));
        }
        drop(s);
        drop(store);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[cfg(test)]
mod analysis_clock_tests {
    use super::*;
    use crate::{
        analysis_subscription::Descriptor,
        authority::Mode,
        fixture::{FixtureSpec, Patch, PatchSpec, SyntheticMode},
    };
    fn ready_service() -> Service {
        ready_service_binding(None)
    }
    fn ready_service_binding(binding: Option<crate::analysis_subscription::Binding>) -> Service {
        ready_service_mode(binding, false)
    }
    fn ready_service_mode(
        binding: Option<crate::analysis_subscription::Binding>,
        music: bool,
    ) -> Service {
        ready_service_mode_clock(binding, music, || 1000)
    }
    fn ready_service_mode_clock(
        binding: Option<crate::analysis_subscription::Binding>,
        music: bool,
        mut clock: impl FnMut() -> u64,
    ) -> Service {
        let configured = binding.is_some();
        let id = Id::new("fixture-11").unwrap();
        let patch = Patch::validate(PatchSpec {
            version: 1,
            patch_revision: 1,
            fixtures: vec![FixtureSpec::synthetic(
                id.clone(),
                "Synthetic",
                SyntheticMode::Dimmer,
                1,
            )],
        })
        .unwrap();
        let mut s = Service::timed(
            Authority::new(patch, "11111111-1111-4111-8111-111111111111", 9).unwrap(),
        )
        .unwrap();
        if let Some(binding) = &binding {
            s.enable_configured_analysis(binding.clone()).unwrap();
        } else {
            s.enable_analysis().unwrap();
        }
        if music {
            s.enable_musical_analysis().unwrap();
        }
        s.engine.execute(Command::Mode(Mode::Auto)).unwrap();
        let c: Json =
            serde_json::from_str(include_str!("../tests/fixtures/lx05/v1/e09.json")).unwrap();
        let d = if let Some(binding) = &binding {
            let mut descriptor: Json = serde_json::from_slice(include_bytes!(
                "../tests/fixtures/lx05/v2/configured-descriptor.json"
            ))
            .unwrap();
            if binding.version == 3 {
                descriptor["version"] = json!(3);
                descriptor["subscription"] = json!("lux.aux.v3");
                descriptor["inputs"] = json!(binding.inputs);
                descriptor["sources"] = json!(binding.sources);
            }
            Descriptor::decode_configured(&serde_json::to_vec(&descriptor).unwrap(), binding)
                .unwrap()
        } else {
            Descriptor::decode(&serde_json::to_vec(&c["descriptor"]).unwrap()).unwrap()
        };
        let channel_count = d.inputs.len();
        let origin = counter::parse(&d.first_frame).unwrap();
        let calibration = counter::parse(&d.calibration_revision).unwrap();
        let a = s.analysis.as_mut().unwrap();
        a.attach(d);
        for n in 0..111u64 {
            let now = clock();
            let mut b = b"GAW1".to_vec();
            for v in [0u64, 1, calibration, now] {
                b.extend(v.to_be_bytes());
            }
            b.extend([0; 4]);
            for i in 0..10 {
                let h = c["packets_hex"][i].as_str().unwrap();
                let mut p: Vec<u8> = (0..h.len())
                    .step_by(2)
                    .map(|j| u8::from_str_radix(&h[j..j + 2], 16).unwrap())
                    .collect();
                p[24..32].copy_from_slice(&(origin + n * 480 + i as u64 * 48).to_be_bytes());
                p[32..36].copy_from_slice(&(n as u32 * 10 + i as u32).to_be_bytes());
                if channel_count > 4 {
                    let legacy = p.clone();
                    p.truncate(48);
                    p[6..8].copy_from_slice(&(channel_count as u16).to_be_bytes());
                    for frame in 0..48 {
                        p.extend_from_slice(&legacy[48 + frame * 12..48 + (frame + 1) * 12]);
                        for _ in 4..channel_count {
                            p.extend_from_slice(&legacy[48 + frame * 12 + 6..48 + frame * 12 + 9]);
                        }
                    }
                }
                b.extend(p);
            }
            a.ingest(&b, now).unwrap();
            if n == 0 {
                a.calibrate("start", now).unwrap();
            }
            if n == 50 {
                a.calibrate("finish", now).unwrap();
            }
        }
        assert!(a.ready());
        assert_eq!(a.descriptor.as_ref().unwrap().version >= 2, configured);
        let generation = a.generation;
        let writer = Id::new("clock-test").unwrap();
        s.sessions.insert(
            writer.clone(),
            Session {
                lease: 1,
                expiry: 200,
                highwater: 1,
                cache: VecDeque::new(),
            },
        );
        s.analysis_grant = Some(AnalysisGrant {
            writer,
            lease: 1,
            issue_revision: s.engine.revision(),
            patch: 1,
            generation,
            expiry: 100,
            fixtures: vec![id],
            cap: 500,
            color_cap: None,
        });
        s
    }
    fn refresh_test_source(s: &mut Service, n: u64) {
        let c: Json =
            serde_json::from_str(include_str!("../tests/fixtures/lx05/v1/e09.json")).unwrap();
        let a = s.analysis.as_mut().unwrap();
        let d = a.descriptor.as_ref().unwrap();
        let origin = counter::parse(&d.first_frame).unwrap();
        let calibration = counter::parse(&d.calibration_revision).unwrap();
        let now = crate::analysis_subscription::monotonic_ms().unwrap();
        let mut bytes = b"GAW1".to_vec();
        for v in [0u64, 1, calibration, now] {
            bytes.extend(v.to_be_bytes());
        }
        bytes.extend([0; 4]);
        for i in 0..10 {
            let hex = c["packets_hex"][i].as_str().unwrap();
            let mut packet: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|j| u8::from_str_radix(&hex[j..j + 2], 16).unwrap())
                .collect();
            packet[24..32].copy_from_slice(&(origin + n * 480 + i as u64 * 48).to_be_bytes());
            packet[32..36].copy_from_slice(&(n as u32 * 10 + i as u32).to_be_bytes());
            bytes.extend(packet);
        }
        a.ingest(&bytes, now).unwrap();
        s.analysis_now = now;
    }
    #[test]
    fn maximum_scene_grant_renewal_and_preview_are_compact_with_many_stores() {
        let mut s = ready_service_mode(None, true);
        s.analysis_grant = None;
        let fixtures = (0..32)
            .map(|i| {
                FixtureSpec::synthetic(
                    Id::new(&format!("fixture-{i:02}")).unwrap(),
                    "RGB",
                    SyntheticMode::Rgb,
                    1 + i * 4,
                )
            })
            .collect::<Vec<_>>();
        let patch = Patch::validate(PatchSpec {
            version: 1,
            patch_revision: 2,
            fixtures,
        })
        .unwrap();
        let map = crate::scenes::ShowMap::defaults(&patch);
        let values = map.values(&Id::new("driving").unwrap(), &patch).unwrap();
        let stores = (0..32)
            .map(|i| crate::authority::StoredLook {
                kind: StoredKind::Cue,
                id: Id::new(&format!("cue-{i:02}")).unwrap(),
                values: values.clone(),
            })
            .collect();
        s.engine = Authority::new(patch, "11111111-1111-4111-8111-111111111111", 9)
            .unwrap()
            .with_stores(stores)
            .unwrap();
        s.engine.enable_analysis();
        s.engine.execute(Command::Mode(Mode::Auto)).unwrap();
        s.enable_scene_director().unwrap();
        let e = s.analysis.as_ref().unwrap().scene_evidence();
        let actual = s
            .engine
            .snapshot()
            .fixtures
            .into_iter()
            .flat_map(|f| {
                f.attributes.into_iter().map(move |a| Value {
                    fixture: f.fixture.clone(),
                    attribute: a.attribute,
                    value: a.resolved,
                })
            })
            .collect::<Vec<_>>();
        for now in (1000..4000).step_by(100) {
            s.scenes
                .as_mut()
                .unwrap()
                .step(e.clone(), now, s.engine.patch(), &actual)
                .unwrap();
        }
        let fixture_ids = s
            .engine
            .patch()
            .fixtures()
            .iter()
            .map(|f| f.id.clone())
            .collect::<Vec<_>>();
        let command = |s: &Service, n: u64, action: Json| json!({"contract":"C-LIGHT","version":1,"show_id":"11111111-1111-4111-8111-111111111111","module":"lighting","epoch":"9","writer":"clock-test","lease":"1","request_id":n.to_string(),"expected_revision":s.engine.revision().to_string(),"kind":"command","body":{"scope":SCOPE,"command":action}});
        refresh_test_source(&mut s, 111);
        let first_request = command(
            &s,
            2,
            json!({"action":"scene_grant","fixtures":fixture_ids,"intensity_cap":1000,"color_cap":1000,"ttl_ms":2000}),
        );
        let first = s.handle(&serde_json::to_vec(&first_request).unwrap());
        assert_eq!(first[0]["kind"], "applied", "{first:?}");
        assert_eq!(first[0]["body"]["application"], "scene_granted");
        assert!(serde_json::to_vec(&first[0]).unwrap().len() < MAX_MESSAGE_BYTES);
        s.engine.step_scene(&values,&fixture_ids,1000,1000,&json!({"map_revision":"1","scene_id":"driving","analysis_generation":s.analysis.as_ref().unwrap().generation.to_string(),"section_id":null,"source_identity":s.analysis_status()["source_identity"],"source_window_range":s.analysis_status()["source_window_range"]})).unwrap();
        let populated = s.inventory().unwrap();
        assert_eq!(
            populated["scenes"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
                .len(),
            128
        );
        assert!(s.pages().unwrap().len() > 1);
        refresh_test_source(&mut s, 112);
        let renewal_request = command(
            &s,
            3,
            json!({"action":"scene_grant","fixtures":fixture_ids,"intensity_cap":1000,"color_cap":1000,"ttl_ms":2000}),
        );
        let renewal = s.handle(&serde_json::to_vec(&renewal_request).unwrap());
        assert_eq!(renewal[0]["kind"], "applied", "{renewal:?}");
        assert!(serde_json::to_vec(&renewal[0]).unwrap().len() < MAX_MESSAGE_BYTES);
        let preview_request = command(
            &s,
            4,
            json!({"action":"scene_preview","scene_id":"driving"}),
        );
        let revision = s.engine.revision();
        let preview = s.handle(&serde_json::to_vec(&preview_request).unwrap());
        assert_eq!(preview[0]["kind"], "applied");
        assert_eq!(preview[0]["body"]["actual"].as_array().unwrap().len(), 128);
        assert!(serde_json::to_vec(&preview[0]).unwrap().len() < MAX_MESSAGE_BYTES);
        assert_eq!(revision, s.engine.revision());
        let retry = s.handle(&serde_json::to_vec(&preview_request).unwrap());
        assert_eq!(retry, preview);
        if let Ok(path) = std::env::var("GP_SCENES_MAX_FIXTURE_OUT") {
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(std::path::Path::new(&path).join("maximum.json"),serde_json::to_string_pretty(&json!({"provenance":"Actual calibrated Lux source decoder, production director/authority,32RGB fixtures and32cues; real leased grant+renewal+preview ACKs. Software/null output.","inventory":s.inventory().unwrap(),"grant":{"request":renewal_request,"replies":renewal},"preview":{"request":preview_request,"replies":preview}})).unwrap()+"\n").unwrap();
        }
    }
    #[test]
    fn scene_producer_authority_loss_and_real_command_preview() {
        let mut s = ready_service_mode(None, true);
        s.engine
            .execute(Command::ReplacePatch(PatchSpec {
                version: 1,
                patch_revision: 2,
                fixtures: vec![FixtureSpec::synthetic(
                    Id::new("fixture-11").unwrap(),
                    "RGB",
                    SyntheticMode::Rgb,
                    1,
                )],
            }))
            .unwrap();
        s.analysis_grant = None;
        s.enable_scene_director().unwrap();
        s.analysis_now = 1000;
        let initial = s.inventory().unwrap();
        let e = s.analysis.as_ref().unwrap().scene_evidence();
        let actual = s
            .engine
            .snapshot()
            .fixtures
            .into_iter()
            .flat_map(|f| {
                f.attributes.into_iter().map(move |a| Value {
                    fixture: f.fixture.clone(),
                    attribute: a.attribute,
                    value: a.resolved,
                })
            })
            .collect::<Vec<_>>();
        for now in (1000..4000).step_by(100) {
            s.scenes
                .as_mut()
                .unwrap()
                .step(e.clone(), now, s.engine.patch(), &actual)
                .unwrap();
        }
        s.scene_generation = Some(s.analysis.as_ref().unwrap().generation);
        s.analysis_grant = Some(AnalysisGrant {
            writer: Id::new("clock-test").unwrap(),
            lease: 1,
            issue_revision: s.engine.revision(),
            patch: 2,
            generation: s.analysis.as_ref().unwrap().generation,
            expiry: 100,
            fixtures: vec![Id::new("fixture-11").unwrap()],
            cap: 700,
            color_cap: Some(650),
        });
        s.poll_analysis_clock(None, 1000, 1, || Ok(1000)).unwrap();
        let active = s.inventory().unwrap();
        assert!(
            active["scenes"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["attribute"] == "red")
        );
        assert!(active["analysis"]["grant"].is_null());
        let scene_id = s.scenes.as_ref().unwrap().status.current.clone().unwrap();
        let request = json!({"contract":"C-LIGHT","version":1,"show_id":"11111111-1111-4111-8111-111111111111","module":"lighting","epoch":"9","writer":"clock-test","lease":"1","request_id":"2","expected_revision":s.engine.revision().to_string(),"kind":"command","body":{"scope":SCOPE,"command":{"action":"scene_preview","scene_id":scene_id}}});
        let before = s.engine.revision();
        let preview = s.handle(&serde_json::to_vec(&request).unwrap());
        assert_eq!(preview[0]["kind"], "applied");
        assert_eq!(s.engine.revision(), before);
        assert_eq!(preview[0]["body"]["actual"].as_array().unwrap().len(), 4);
        for actual in preview[0]["body"]["actual"].as_array().unwrap() {
            let a = active["snapshot"]["fixtures"][0]["attributes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["attribute"] == actual["attribute"])
                .unwrap();
            assert_eq!(actual["value"], a["resolved"]);
            assert_eq!(actual["source"], a["source"]);
        }
        s.poll_analysis_clock(None, 1101, 2, || Ok(1101)).unwrap();
        let held = s.inventory().unwrap();
        assert!(held["scenes"]["grant"].is_null());
        assert!(held["scenes"]["current"].is_null());
        assert!(
            held["scenes"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
                .iter()
                .all(|v| v["source"] == "scene-held")
        );
        if let Ok(path) = std::env::var("GP_SCENES_FIXTURE_OUT") {
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(std::path::Path::new(&path).join("producer.json"),serde_json::to_string_pretty(&json!({"provenance":"Actual Lux calibrated PCM analyzer, production Director, Authority, Service inventory and leased command handler; elapsed director clock injected; null output.","initial":initial,"active":active,"held":held,"preview":{"request":request,"replies":preview}})).unwrap()+"\n").unwrap();
        }
    }
    /// One-time synthetic owner evidence, retained as JSON for consumer tests.
    #[test]
    #[ignore = "opt-in musical owner corpus refresh; no private audio"]
    fn musical_owner_estimate_corpus() {
        use std::f32::consts::TAU;
        for binding in [
            None,
            Some(crate::analysis_subscription::Binding {
                sources: vec![],
                version: 2,
                inputs: ["input-17", "input-03", "input-09", "input-01"]
                    .map(String::from)
                    .to_vec(),
            }),
        ] {
            let configured = binding.is_some();
            let mut service = ready_service_mode(binding, true);
            let analysis = service.analysis.as_mut().unwrap();
            let descriptor = analysis.descriptor.as_ref().unwrap().clone();
            let origin = counter::parse(&descriptor.first_frame).unwrap();
            let calibration = counter::parse(&descriptor.calibration_revision).unwrap();
            let corpus: Json =
                serde_json::from_str(include_str!("../tests/fixtures/lx05/v1/e09.json")).unwrap();
            let mut n = 111u64;
            let progression = [
                ([60., 64., 67.], 300),
                ([65., 69., 72.], 100),
                ([67., 71., 74.], 100),
                ([60., 64., 67.], 200),
                ([60., 64., 67.], 300),
                ([65., 69., 72.], 100),
                ([67., 71., 74.], 100),
                ([60., 64., 67.], 200),
                ([67., 71., 74.], 65),
            ];
            for (notes, windows) in progression {
                let frequencies = notes.map(|note| 440. * 2.0_f32.powf((note - 69.) / 12.));
                for _ in 0..windows {
                    let mut bytes = b"GAW1".to_vec();
                    for value in [0u64, 1, calibration, 1000] {
                        bytes.extend(value.to_be_bytes());
                    }
                    bytes.extend([0; 4]);
                    for packet in 0..10 {
                        let hex = corpus["packets_hex"][packet].as_str().unwrap();
                        let mut pcm: Vec<u8> = (0..hex.len())
                            .step_by(2)
                            .map(|j| u8::from_str_radix(&hex[j..j + 2], 16).unwrap())
                            .collect();
                        let first = origin + n * 480 + packet as u64 * 48;
                        pcm[24..32].copy_from_slice(&first.to_be_bytes());
                        pcm[32..36].copy_from_slice(&(n as u32 * 10 + packet as u32).to_be_bytes());
                        for frame in 0..48 {
                            let time =
                                (n * 480 + packet as u64 * 48 + frame as u64) as f32 / 48000.;
                            let tone = frequencies
                                .iter()
                                .map(|frequency| (TAU * frequency * time).sin())
                                .sum::<f32>()
                                * 0.2;
                            // Opposite guitars preserve independent-channel evidence.
                            for (channel, sample) in [(1, tone), (2, tone), (3, -tone)] {
                                let encoded = ((sample * 8388608.).round() as i32).to_be_bytes();
                                let offset = 48 + (frame * 4 + channel) * 3;
                                pcm[offset..offset + 3].copy_from_slice(&encoded[1..]);
                            }
                        }
                        bytes.extend(pcm);
                    }
                    analysis.ingest(&bytes, 1000).unwrap();
                    n += 1;
                }
            }
            service
                .poll_analysis_clock(None, 1000, 1, || Ok(1000))
                .unwrap();
            let active = service.inventory().unwrap();
            for estimate in ["chord", "key", "movement", "section"] {
                assert!(
                    !active["analysis"]["musical"][estimate].is_null(),
                    "missing {estimate}: {}",
                    active["analysis"]["musical"]
                );
            }
            assert_eq!(active["analysis"]["musical"]["key"]["tonic"], 0);
            assert!(service.engine.snapshot().fixtures[0].attributes[0].resolved <= 500);
            for value in active["analysis"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
            {
                assert_eq!(value["provenance"]["analysis_version"], "lux.analysis.v2");
            }
            service
                .engine
                .execute(Command::Touch(vec![crate::fixture::Value {
                    fixture: Id::new("fixture-11").unwrap(),
                    attribute: crate::fixture::Attribute::Intensity,
                    value: 900,
                }]))
                .unwrap();
            service
                .poll_analysis_clock(None, 1000, 2, || Ok(1000))
                .unwrap();
            assert_eq!(
                service.engine.snapshot().fixtures[0].attributes[0].resolved,
                900
            );
            service.engine.execute(Command::ClearToHold).unwrap();
            service
                .poll_analysis_clock(None, 1000, 3, || Ok(1000))
                .unwrap();
            assert_eq!(
                service.engine.snapshot().fixtures[0].attributes[0].resolved,
                900
            );
            service
                .poll_analysis_clock(None, 1101, 4, || Ok(1101))
                .unwrap();
            let stale = service.inventory().unwrap();
            service.analysis.as_mut().unwrap().absent();
            let unknown = service.inventory().unwrap();
            std::fs::create_dir_all("tests/fixtures/musical").unwrap();
            let name = if configured {
                "configured-estimates"
            } else {
                "legacy-estimates"
            };
            std::fs::write(
                format!("tests/fixtures/musical/{name}.json"),
                serde_json::to_string_pretty(
                    &json!({"active":active,"stale":stale,"unknown":unknown}),
                )
                .unwrap()
                    + "\n",
            )
            .unwrap();
        }
    }
    #[test]
    fn musical_opt_in_schema_source_frames_and_stale_reset() {
        for binding in [
            None,
            Some(crate::analysis_subscription::Binding {
                sources: vec![],
                version: 2,
                inputs: ["input-17", "input-03", "input-09", "input-01"]
                    .map(String::from)
                    .to_vec(),
            }),
            Some(crate::analysis_subscription::Binding {
                version: 3,
                inputs: (1..=7).map(|n| format!("input-{n:02}")).collect(),
                sources: [
                    "kick", "bass", "guitar-1", "guitar-2", "vocal", "synth", "backs",
                ]
                .map(String::from)
                .to_vec(),
            }),
        ] {
            let configured = binding.is_some();
            let mut service = ready_service_mode(binding, true);
            service
                .poll_analysis_clock(None, 1000, 1, || Ok(1000))
                .unwrap();
            let active = service.inventory().unwrap();
            assert_eq!(
                active["wire_schema"],
                if configured {
                    "lx05-music-v4"
                } else {
                    "lx05-music-v3"
                }
            );
            assert_eq!(active["analysis"]["analysis_version"], "lux.analysis.v2");
            assert_eq!(active["analysis"]["musical"]["state"], "tracking");
            let music_frame = counter::parse(
                active["analysis"]["musical"]["source_frame"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            let window_end = counter::parse(
                active["analysis"]["source_window_range"]["end_frame_exclusive"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap();
            assert!(music_frame <= window_end);
            assert!(
                window_end - music_frame < 6144,
                "last spectral hop must remain within one hop"
            );
            for value in active["analysis"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
            {
                if !value["provenance"].is_null() {
                    assert_eq!(value["provenance"]["analysis_version"], "lux.analysis.v2");
                }
            }
            assert!(active["analysis"]["beat"].is_null());
            assert!(active["analysis"]["harmony"].is_null());
            if configured {
                validate_configured_analysis(&active).unwrap();
            }
            // Every ownership/calibration discontinuity discards musical history.
            let mut reset = service.analysis.as_ref().unwrap().clone();
            for reason in [
                "lost_windows",
                "gap_overlap_reorder",
                "mapping",
                "clock_regression",
            ] {
                reset.invalidate(reason);
                let status = reset.status(1000);
                assert_eq!(status["musical"]["state"], "unknown");
                assert!(status["musical"]["source_frame"].is_null());
                assert_eq!(status["musical"]["chroma"], json!(vec![0; 12]));
                for estimate in ["chord", "key", "movement", "section"] {
                    assert!(status["musical"][estimate].is_null());
                }
            }
            let mut calibration = service.analysis.as_ref().unwrap().clone();
            calibration.calibrate("start", 1000).unwrap();
            assert_eq!(calibration.status(1000)["musical"]["state"], "unknown");
            let descriptor = calibration.descriptor.as_ref().unwrap().clone();
            calibration.attach(descriptor);
            assert!(calibration.status(1000)["musical"]["source_frame"].is_null());
            let before = service.engine.snapshot();
            service
                .poll_analysis_clock(None, 1101, 2, || Ok(1101))
                .unwrap();
            let stale = service.inventory().unwrap();
            assert_eq!(stale["analysis"]["musical"]["state"], "stale");
            assert!(stale["analysis"]["musical"]["source_frame"].is_null());
            assert!(stale["analysis"]["musical"]["key"].is_null());
            assert_eq!(stale["analysis"]["musical"]["chroma"], json!(vec![0; 12]));
            if active["analysis_binding"]["version"] == 3 {
                assert_eq!(
                    active["analysis"]["rms_millionths"]
                        .as_array()
                        .unwrap()
                        .len(),
                    7
                );
                if let Ok(path) = std::env::var("LUX_SEVEN_FEED_CORPUS") {
                    std::fs::write(
                        path,
                        serde_json::to_string_pretty(&json!({"active":active,"stale":stale}))
                            .unwrap()
                            + "\n",
                    )
                    .unwrap();
                }
            }
            if std::env::var_os("LUX_GENERATE_MUSICAL_CORPUS").is_some() {
                let name = if configured {
                    "configured"
                } else {
                    "legacy-binding"
                };
                std::fs::create_dir_all("tests/fixtures/musical").unwrap();
                std::fs::write(
                    format!("tests/fixtures/musical/{name}.json"),
                    serde_json::to_string_pretty(&json!({"active":active,"stale":stale})).unwrap()
                        + "\n",
                )
                .unwrap();
            }
            assert!(service.analysis_grant.is_none());
            assert_eq!(
                before.fixtures[0].attributes[0].resolved,
                service.engine.snapshot().fixtures[0].attributes[0].resolved
            );
            service.analysis.as_mut().unwrap().absent();
            assert_eq!(
                service.inventory().unwrap()["analysis"]["musical"]["state"],
                "unknown"
            );
        }
        assert!(ready_service().enable_musical_analysis().is_err());
        assert!(
            ready_service().inventory().unwrap()["analysis"]
                .get("musical")
                .is_none()
        );
    }
    #[test]
    fn processing_and_encoding_delay_or_clock_regression_never_commit_stale_step() {
        for clocks in [[1101, 1102], [1099, 1101], [1099, 1098]] {
            let mut s = ready_service();
            let baseline = s.engine.snapshot();
            let mut times = clocks.into_iter();
            s.poll_analysis_clock(None, 1099, 1, || Ok(times.next().unwrap()))
                .unwrap();
            assert!(s.analysis_grant.is_none());
            assert_eq!(
                s.engine.snapshot().fixtures[0].attributes[0].resolved,
                baseline.fixtures[0].attributes[0].resolved
            );
            assert_eq!(
                s.analysis.as_ref().unwrap().status(s.analysis_now)["confidence"],
                0
            );
            s.pages().unwrap();
        }
    }
    #[test]
    fn actual_near_fifteen_page_step_capacity_freezes_exact_auto_layer() {
        use crate::authority::StoredLook;
        use crate::fixture::{Attribute, Value};
        let template = ready_service();
        fn build(template: &Service, full: usize, tail: usize, padding: usize) -> Service {
            let mut fixtures = (0..32)
                .map(|i| {
                    FixtureSpec::synthetic(
                        Id::new(&format!("f{i:02}{}", "x".repeat(60))).unwrap(),
                        "",
                        SyntheticMode::RgbPosition,
                        i * 7 + 1,
                    )
                })
                .collect::<Vec<_>>();
            for f in fixtures.iter_mut().take(2) {
                f.label = "\"".repeat(padding);
            }
            let patch = Patch::validate(PatchSpec {
                version: 1,
                patch_revision: 1,
                fixtures,
            })
            .unwrap();
            let all = patch
                .fixtures()
                .iter()
                .flat_map(|f| {
                    f.capabilities.iter().map(|c| Value {
                        fixture: f.id.clone(),
                        attribute: c.attribute,
                        value: c.default,
                    })
                })
                .collect::<Vec<_>>();
            let count = full + usize::from(tail > 0);
            let stores = (0..count)
                .map(|i| StoredLook {
                    kind: if i < 32 {
                        StoredKind::Cue
                    } else {
                        StoredKind::Palette
                    },
                    id: Id::new(&format!("s{i:02}{}", "x".repeat(60))).unwrap(),
                    values: if i == full {
                        all[..tail * 7].to_vec()
                    } else {
                        all.clone()
                    },
                })
                .collect();
            let mut e = Authority::new(patch, "11111111-1111-4111-8111-111111111111", 9)
                .unwrap()
                .with_stores(stores)
                .unwrap();
            for values in all.chunks(56) {
                e.execute(Command::Touch(values.to_vec())).unwrap();
            }
            e.execute(Command::ClearToHold).unwrap();
            e.enable_analysis();
            e.execute(Command::Mode(Mode::Auto)).unwrap();
            let targets = e
                .patch()
                .fixtures()
                .iter()
                .map(|f| f.id.clone())
                .collect::<Vec<_>>();
            for _ in 0..5 {
                e.step_auto(&targets, 500, 99).unwrap();
            }
            let mut s = template.clone();
            s.engine = e;
            s.analysis_now = 1000;
            let g = s.analysis_grant.as_mut().unwrap();
            g.fixtures = targets;
            g.issue_revision = s.engine.revision();
            s
        }
        // Find a real production boundary with bounded binary searches, then pack
        // the last partial store and legal quoted labels to within a few bytes.
        fn maximum(mut low: usize, mut high: usize, mut fits: impl FnMut(usize) -> bool) -> usize {
            while low < high {
                let mid = (low + high).div_ceil(2);
                if fits(mid) { low = mid } else { high = mid - 1 }
            }
            low
        }
        let full = maximum(0, 63, |n| build(&template, n, 0, 0).pages().is_ok());
        let tail = maximum(0, 31, |n| build(&template, full, n, 0).pages().is_ok());
        let padding = maximum(0, 128, |n| build(&template, full, tail, n).pages().is_ok());
        let mut s = build(&template, full, tail, padding);
        assert_eq!(s.pages().unwrap().len(), 15);
        let before = s.engine.snapshot();
        let mut candidate = s.clone();
        candidate
            .poll_analysis_inner(None, 1000, 1, &mut || Ok(1000))
            .unwrap();
        assert_eq!(
            candidate.pages().unwrap_err(),
            "capacity",
            "fixture must actually cross production active15page budget"
        );
        s.poll_analysis_clock(None, 1000, 1, || Ok(1000)).unwrap();
        assert!(s.analysis_grant.is_none());
        assert_eq!(
            s.analysis.as_ref().unwrap().reason,
            Some("snapshot_capacity")
        );
        assert!(s.pages().unwrap().len() <= 16);
        for (old, new) in before.fixtures.iter().zip(s.engine.snapshot().fixtures) {
            for (a, b) in old.attributes.iter().zip(new.attributes) {
                assert_eq!(a.resolved, b.resolved);
                assert_eq!(a.auto, b.auto);
                assert_eq!(a.hold, b.hold);
                if b.attribute == Attribute::Intensity {
                    assert_eq!(b.auto, Some(99));
                    assert!(b.contributors.iter().any(|c| c.source
                        == crate::lighting_contract::Source::Auto(
                            Id::new("analysis-held").unwrap()
                        )));
                }
            }
        }
    }
    #[test]
    fn qualified_pcm_active_loss_and_manual_owner_output_corpus() {
        let mut s = ready_service();
        s.poll_analysis_clock(None, 1000, 1, || Ok(1000)).unwrap();
        let active = s.inventory().unwrap();
        assert_eq!(
            active["snapshot"]["fixtures"][0]["attributes"][0]["resolved"],
            20
        );
        s.poll_analysis_clock(None, 1101, 2, || Ok(1101)).unwrap();
        let lost = s.inventory().unwrap();
        assert!(s.analysis_grant.is_none());
        let request = json!({"contract":"C-LIGHT","version":1,"show_id":"11111111-1111-4111-8111-111111111111","module":"lighting","epoch":"9","writer":"clock-test","lease":"1","request_id":"2","expected_revision":s.engine.revision().to_string(),"kind":"command","body":{"scope":SCOPE,"command":{"action":"touch","values":[{"fixture":"fixture-11","attribute":"intensity","value":900}]}}});
        let replies = s.handle(&serde_json::to_vec(&request).unwrap());
        assert_eq!(replies[0]["kind"], "applied");
        let manual = s.inventory().unwrap();
        let corpus = json!({"schema":"lx05-v1","provenance":"Actual independent Lux PCM decoder/Calibrator/Analyzer/Director and Authority/Service. Exact accepted E09 PCM; injected same-host acquisition clock1000/1101ms, synthetic fixture; no physical output.","active":active,"aged_loss":lost,"manual":{"request":request,"replies":replies,"inventory":manual}});
        if std::env::var_os("LUX_GENERATE_LX05_CORPUS").is_some() {
            std::fs::write(
                "tests/fixtures/lx05/v1/features.json",
                serde_json::to_string_pretty(&corpus).unwrap() + "\n",
            )
            .unwrap();
        } else {
            assert_eq!(
                corpus,
                serde_json::from_str::<Json>(include_str!(
                    "../tests/fixtures/lx05/v1/features.json"
                ))
                .unwrap()
            );
        }
    }
    #[test]
    fn configured_owner_corpus_active_absent_retained_and_manual() {
        let binding = crate::analysis_subscription::Binding {
            sources: vec![],
            version: 2,
            inputs: ["input-17", "input-03", "input-09", "input-01"]
                .map(String::from)
                .to_vec(),
        };
        let mut s = ready_service_binding(Some(binding.clone()));
        s.poll_analysis_clock(None, 1000, 1, || Ok(1000)).unwrap();
        let active = s.inventory().unwrap();
        validate_configured_analysis(&active).unwrap();
        s.analysis.as_mut().unwrap().absent();
        s.poll_analysis_clock(None, 1001, 2, || Ok(1001)).unwrap();
        let absent = s.inventory().unwrap();
        assert!(absent["analysis"]["source_identity"].is_null());
        assert!(
            !absent["analysis"]["automatic_layer"]["values"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        validate_configured_analysis(&absent).unwrap();
        let old =
            &absent["analysis"]["automatic_layer"]["values"][0]["provenance"]["source_identity"];
        assert_eq!(old["version"], 2);
        assert_eq!(old["inputs"], json!(binding.inputs));
        let request = json!({"contract":"C-LIGHT","version":1,"show_id":"11111111-1111-4111-8111-111111111111","module":"lighting","epoch":"9","writer":"clock-test","lease":"1","request_id":"2","expected_revision":s.engine.revision().to_string(),"kind":"command","body":{"scope":SCOPE,"command":{"action":"touch","values":[{"fixture":"fixture-11","attribute":"intensity","value":900}]}}});
        let replies = s.handle(&serde_json::to_vec(&request).unwrap());
        assert_eq!(replies[0]["kind"], "applied");
        let manual = s.inventory().unwrap();
        let config = crate::service_config::ServiceConfig {
            version: 1,
            show_id: "22222222-2222-4222-8222-222222222222".into(),
            patch: PatchSpec {
                version: 1,
                patch_revision: 7,
                fixtures: vec![
                    FixtureSpec::synthetic(
                        Id::new("front").unwrap(),
                        "Front",
                        SyntheticMode::Dimmer,
                        1,
                    ),
                    FixtureSpec::synthetic(
                        Id::new("back").unwrap(),
                        "Back",
                        SyntheticMode::Rgb,
                        12,
                    ),
                ],
            },
            analysis: Some(binding),
        };
        let mut cold =
            Service::new(Authority::new(config.validate().unwrap(), &config.show_id, 1).unwrap())
                .unwrap();
        cold.enable_configured_analysis(config.analysis.clone().unwrap())
            .unwrap();
        let cold_start = cold.inventory().unwrap();
        let status_request = json!({"contract":"C-LIGHT","version":1,"show_id":config.show_id,"module":"lighting","epoch":"1","writer":null,"lease":null,"request_id":null,"expected_revision":null,"kind":"analysis_status","body":{}});
        let status_reply = cold.handle(&serde_json::to_vec(&status_request).unwrap());
        let corpus = json!({"schema":"lx05-v2","provenance":"Actual independent Lux configured decoder, existing Calibrator/Analyzer/Director and Authority/Service; accepted synthetic E09 PCM with configured source frame/calibration headers; injected same-host clocks. No hardware output.","config":config,"cold_start":cold_start,"status":{"request":status_request,"replies":status_reply},"active":active,"source_absent_retained":absent,"manual":{"request":request,"replies":replies,"inventory":manual}});
        if std::env::var_os("LUX_GENERATE_LX05_V2_CORPUS").is_some() {
            std::fs::write(
                "tests/fixtures/lx05/v2/configured.json",
                serde_json::to_string_pretty(&corpus).unwrap() + "\n",
            )
            .unwrap();
        } else {
            let saved = std::fs::read("tests/fixtures/lx05/v2/configured.json").unwrap();
            assert_eq!(corpus, serde_json::from_slice::<Json>(&saved).unwrap());
        }
    }
    #[test]
    fn assist_without_grant_encoding_age_expiry_clears_snapshot_proposal() {
        let mut s = ready_service();
        s.engine.execute(Command::Mode(Mode::Assist)).unwrap();
        s.analysis_grant = None;
        let mut times = [1099, 1101].into_iter();
        s.poll_analysis_clock(None, 1099, 1, || Ok(times.next().unwrap()))
            .unwrap();
        assert!(s.analysis.as_ref().unwrap().proposal.is_none());
        assert_eq!(s.analysis.as_ref().unwrap().state, "stale");
        assert!(
            s.engine
                .snapshot()
                .fixtures
                .iter()
                .flat_map(|f| &f.attributes)
                .all(|a| a.proposal.is_none())
        );
        assert_eq!(s.analysis_status()["confidence"], 0);
    }
}
