//! C-LIGHT:1 bounded snapshot vocabulary, not an arbitration engine or wire service.
use crate::fixture::{Attribute, Error, Id, MAX_FIXTURES, Patch, VERSION};
use std::collections::BTreeSet;

pub const CONTRACT: &str = "C-LIGHT";
pub const MAX_PLAYBACKS: usize = 8;
pub const MAX_CUES: usize = 32;
pub const MAX_PALETTES: usize = 32;
pub const MAX_PAGES: usize = 16;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
pub const MAX_DEPTH: usize = 12;
pub const PAGE_TIMEOUT_MS: u16 = 2000;

/// Exact decimal string form for counters used by a future JSON adapter.
pub fn decimal_counter(value: &str) -> Result<u64, Error> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::Range);
    }
    value.parse().map_err(|_| Error::Range)
}

pub fn validate_show_id(value: &str) -> Result<(), Error> {
    if value.len() != 36
        || !value.bytes().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        return Err(Error::Id);
    }
    Ok(())
}

/// IDs name stored/playing contributors, never their array positions.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    FixtureDefault,
    Playback(Id),
    Auto(Id),
    Hold,
    Programmer,
    Release,
}
impl Source {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::FixtureDefault => "fixture_default",
            Self::Playback(_) => "playback",
            Self::Auto(_) => "auto",
            Self::Hold => "hold",
            Self::Programmer => "programmer",
            Self::Release => "release",
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Contributor {
    pub source: Source,
    pub value: i32,
    pub winner: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Inhibit {
    None,
    Blackout,
    Protection,
    Unavailable,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Volatile,
    Checkpointed,
    Error,
}

/// Output observations remain unavailable in LX-01; never inferred from intent.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttributeSnapshot {
    pub attribute: Attribute,
    pub programmer: Option<i32>,
    pub hold: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto: Option<i32>,
    pub stored: Vec<StoredValue>,
    pub playing: Vec<NamedValue>,
    pub proposal: Option<i32>,
    pub resolved: i32,
    pub final_intent: i32,
    pub source: Source,
    pub contributors: Vec<Contributor>,
    pub inhibit: Inhibit,
    pub clamped: bool,
    pub submitted: Option<i32>,
    pub observed: Option<i32>,
}
/// Cue and palette identities occupy separate bounded namespaces.
#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum StoredKind {
    Cue,
    Palette,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StoredValue {
    pub kind: StoredKind,
    pub id: Id,
    pub value: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NamedValue {
    pub id: Id,
    pub value: i32,
}
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureSnapshot {
    pub fixture: Id,
    pub attributes: Vec<AttributeSnapshot>,
}

/// A complete pure snapshot (transport pagination is a later adapter). Client selection is deliberately absent:
/// it belongs to the client and confers no engine ownership.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u16,
    pub show_id: String,
    #[serde(with = "crate::wire::counter")]
    pub epoch: u64,
    #[serde(with = "crate::wire::counter")]
    pub revision: u64,
    #[serde(with = "crate::wire::counter")]
    pub patch_revision: u64,
    pub durability: Durability,
    pub fixtures: Vec<FixtureSnapshot>,
}
impl Snapshot {
    /// Durable owner wire schema allows honest checkpoint/error status; static LX03
    /// validation continues refusing unbacked nonvolatile claims.
    pub fn validate_for_schema(&self, patch: &Patch, schema: &str) -> Result<(), Error> {
        if matches!(
            schema,
            "lx06-scenes-v1" | "lx06-scenes-v3" | "lx06-scenes-v2" | "lx06-scenes-v4"
        ) {
            let mut logical = self.clone();
            logical.durability = Durability::Volatile;
            logical.validate_inner(patch, true, true)
        } else if matches!(
            schema,
            "lx05-v1"
                | "lx05-v2"
                | "lx05-v3"
                | "lx05-music-v1"
                | "lx05-music-v3"
                | "lx05-music-v2"
                | "lx05-music-v4"
        ) {
            let mut logical = self.clone();
            logical.durability = Durability::Volatile;
            logical.validate_inner(patch, true, false)
        } else if schema == "lx04-durable-v1" {
            let mut logical = self.clone();
            logical.durability = Durability::Volatile;
            logical.validate(patch)
        } else if matches!(schema, "lx03-v1" | "lx04-v1") {
            self.validate(patch)
        } else {
            Err(Error::Version)
        }
    }
    /// Schema/capability consistency only. Does not resolve contributors, masters,
    /// holds or inhibit policy (LX-02), persist state (LX-04), or submit output.
    pub fn validate(&self, patch: &Patch) -> Result<(), Error> {
        self.validate_inner(patch, false, false)
    }
    fn validate_inner(&self, patch: &Patch, analysis: bool, scenes: bool) -> Result<(), Error> {
        if self.version != VERSION {
            return Err(Error::Version);
        }
        validate_show_id(&self.show_id)?;
        if self.patch_revision != patch.revision() {
            return Err(Error::Target);
        }
        if self.durability != Durability::Volatile {
            return Err(Error::Unavailable);
        }
        if self.fixtures.len() > MAX_FIXTURES {
            return Err(Error::Capacity);
        }
        if self.fixtures.len() != patch.fixtures().len() {
            return Err(Error::Target);
        }
        let mut cue_ids = BTreeSet::new();
        let mut palette_ids = BTreeSet::new();
        let mut playback_ids = BTreeSet::new();
        for (index, fixture) in self.fixtures.iter().enumerate() {
            let spec = patch.fixture(&fixture.fixture).ok_or(Error::Target)?;
            if self.fixtures[..index]
                .iter()
                .any(|f| f.fixture == fixture.fixture)
            {
                return Err(Error::Duplicate);
            }
            if fixture.attributes.len() != spec.capabilities.len() {
                return Err(Error::Group);
            }
            for (ai, attribute) in fixture.attributes.iter().enumerate() {
                let cap = spec
                    .capability(attribute.attribute)
                    .ok_or(Error::Unavailable)?;
                if fixture.attributes[..ai]
                    .iter()
                    .any(|a| a.attribute == attribute.attribute)
                {
                    return Err(Error::Duplicate);
                }
                if attribute.stored.len() > MAX_CUES + MAX_PALETTES
                    || attribute.playing.len() > MAX_PLAYBACKS
                    || attribute.contributors.len() > MAX_PLAYBACKS + 4
                {
                    return Err(Error::Capacity);
                }
                for (vi, value) in attribute.stored.iter().enumerate() {
                    if attribute.stored[..vi]
                        .iter()
                        .any(|v| v.kind == value.kind && v.id == value.id)
                    {
                        return Err(Error::Duplicate);
                    }
                    match value.kind {
                        StoredKind::Cue => {
                            cue_ids.insert(&value.id);
                        }
                        StoredKind::Palette => {
                            palette_ids.insert(&value.id);
                        }
                    }
                }
                for (vi, value) in attribute.playing.iter().enumerate() {
                    if attribute.playing[..vi].iter().any(|v| v.id == value.id) {
                        return Err(Error::Duplicate);
                    }
                }
                playback_ids.extend(attribute.playing.iter().map(|v| &v.id));
                if cue_ids.len() > MAX_CUES
                    || palette_ids.len() > MAX_PALETTES
                    || playback_ids.len() > MAX_PLAYBACKS
                {
                    return Err(Error::Capacity);
                }
                if attribute
                    .contributors
                    .iter()
                    .filter(|c| matches!(c.source, Source::Auto(_)))
                    .count()
                    > 1
                {
                    return Err(Error::Duplicate);
                }
                if attribute.auto.is_some()
                    && (!analysis
                        || (!scenes && attribute.attribute != Attribute::Intensity)
                        || (scenes
                            && !matches!(
                                attribute.attribute,
                                crate::fixture::Attribute::Intensity
                                    | crate::fixture::Attribute::Red
                                    | crate::fixture::Attribute::Green
                                    | crate::fixture::Attribute::Blue
                            )))
                {
                    return Err(Error::Unavailable);
                }
                let values = [
                    attribute.programmer,
                    attribute.hold,
                    attribute.release,
                    attribute.auto,
                    attribute.proposal,
                    Some(attribute.resolved),
                    Some(attribute.final_intent),
                ];
                if values
                    .into_iter()
                    .flatten()
                    .chain(attribute.stored.iter().map(|v| v.value))
                    .chain(attribute.playing.iter().map(|v| v.value))
                    .chain(attribute.contributors.iter().map(|c| c.value))
                    .any(|v| !cap.contains(v))
                {
                    return Err(Error::Range);
                }
                if attribute.submitted.is_some() || attribute.observed.is_some() {
                    return Err(Error::Unavailable);
                }
                for (ci, c) in attribute.contributors.iter().enumerate() {
                    if attribute.contributors[..ci]
                        .iter()
                        .any(|p| p.source == c.source)
                    {
                        return Err(Error::Duplicate);
                    }
                    match &c.source {
                        Source::Auto(id)
                            if !analysis
                                || (!scenes && attribute.attribute != Attribute::Intensity)
                                || (scenes
                                    && !matches!(
                                        attribute.attribute,
                                        crate::fixture::Attribute::Intensity
                                            | crate::fixture::Attribute::Red
                                            | crate::fixture::Attribute::Green
                                            | crate::fixture::Attribute::Blue
                                    ))
                                || attribute.auto != Some(c.value)
                                || match id.as_str() {
                                    "mode-entry-continuity"
                                    | "analysis-active"
                                    | "analysis-held" => {
                                        attribute.attribute != Attribute::Intensity
                                    }
                                    "scene-active" | "scene-held" => !scenes,
                                    _ => true,
                                } =>
                        {
                            return Err(Error::Unavailable);
                        }
                        Source::FixtureDefault if c.value != cap.default => {
                            return Err(Error::Default);
                        }
                        Source::Programmer if attribute.programmer != Some(c.value) => {
                            return Err(Error::Target);
                        }
                        Source::Release if attribute.release != Some(c.value) => {
                            return Err(Error::Target);
                        }
                        Source::Hold if attribute.hold != Some(c.value) => {
                            return Err(Error::Target);
                        }
                        Source::Playback(id)
                            if !attribute
                                .playing
                                .iter()
                                .any(|p| &p.id == id && p.value == c.value) =>
                        {
                            return Err(Error::Target);
                        }
                        _ => (),
                    }
                    if c.winner && c.value != attribute.resolved {
                        return Err(Error::Target);
                    }
                }
                // Every applied source is reported, including sources hidden by manual priority.
                // Defaults are the fallback only; ASSIST proposals are never applied AUTO.
                if !analysis && matches!(attribute.source, Source::Auto(_)) {
                    return Err(Error::Unavailable);
                }
                let has =
                    |source: &Source| attribute.contributors.iter().any(|c| &c.source == source);
                if attribute.programmer.is_some() != has(&Source::Programmer)
                    || attribute.hold.is_some() != has(&Source::Hold)
                    || attribute.release.is_some() != has(&Source::Release)
                    || attribute.auto.is_some()
                        != attribute
                            .contributors
                            .iter()
                            .any(|c| matches!(c.source, Source::Auto(_)))
                    || attribute
                        .playing
                        .iter()
                        .any(|p| !has(&Source::Playback(p.id.clone())))
                {
                    return Err(Error::Target);
                }
                let fallback = attribute.programmer.is_none()
                    && attribute.hold.is_none()
                    && attribute.release.is_none()
                    && attribute.playing.is_empty()
                    && attribute.auto.is_none();
                if has(&Source::FixtureDefault) != fallback {
                    return Err(Error::Target);
                }
                for c in &attribute.contributors {
                    let eligible = if attribute.programmer.is_some() {
                        c.source == Source::Programmer
                    } else if attribute.release.is_some() {
                        c.source == Source::Release
                    } else if attribute.hold.is_some() {
                        c.source == Source::Hold
                    } else if attribute.auto.is_some() {
                        matches!(c.source, Source::Auto(_))
                    } else if fallback {
                        c.source == Source::FixtureDefault
                    } else {
                        matches!(c.source, Source::Playback(_))
                    };
                    if c.winner && !eligible {
                        return Err(Error::Target);
                    }
                    if attribute.attribute == Attribute::Intensity {
                        let expected = eligible
                            && (attribute.programmer.is_some()
                                || attribute.hold.is_some()
                                || attribute.release.is_some()
                                || attribute.auto.is_some()
                                || fallback
                                || c.value
                                    == attribute.playing.iter().map(|p| p.value).max().unwrap());
                        if c.winner != expected {
                            return Err(Error::Target);
                        }
                    }
                }
                if !attribute.contributors.iter().any(|c| {
                    c.source == attribute.source && c.winner && c.value == attribute.resolved
                }) {
                    return Err(Error::Target);
                }
                match &attribute.source {
                    Source::Programmer if attribute.programmer != Some(attribute.resolved) => {
                        return Err(Error::Target);
                    }
                    Source::Release if attribute.release != Some(attribute.resolved) => {
                        return Err(Error::Target);
                    }
                    Source::Hold if attribute.hold != Some(attribute.resolved) => {
                        return Err(Error::Target);
                    }
                    Source::FixtureDefault if attribute.resolved != cap.default => {
                        return Err(Error::Default);
                    }
                    Source::Playback(id)
                        if !attribute
                            .playing
                            .iter()
                            .any(|p| &p.id == id && p.value == attribute.resolved) =>
                    {
                        return Err(Error::Target);
                    }
                    _ => (),
                }
            }
            for group in [
                &[Attribute::Red, Attribute::Green, Attribute::Blue][..],
                &[Attribute::Pan, Attribute::Tilt][..],
            ] {
                let members: Vec<_> = fixture
                    .attributes
                    .iter()
                    .filter(|a| group.contains(&a.attribute))
                    .collect();
                if let Some(first) = members.first() {
                    for member in &members[1..] {
                        if member.source != first.source
                            || member.programmer.is_some() != first.programmer.is_some()
                            || member.hold.is_some() != first.hold.is_some()
                            || member.proposal.is_some() != first.proposal.is_some()
                            || member.stored.len() != first.stored.len()
                            || member.playing.len() != first.playing.len()
                            || member.contributors.len() != first.contributors.len()
                            || member.stored.iter().any(|v| {
                                !first
                                    .stored
                                    .iter()
                                    .any(|p| p.kind == v.kind && p.id == v.id)
                            })
                            || member
                                .playing
                                .iter()
                                .any(|v| !first.playing.iter().any(|p| p.id == v.id))
                            || member.contributors.iter().any(|c| {
                                !first
                                    .contributors
                                    .iter()
                                    .any(|p| p.source == c.source && p.winner == c.winner)
                            })
                        {
                            return Err(Error::Group);
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn output_state(&self) -> &'static str {
        "null_disarmed"
    }
    pub fn physical_state(&self) -> &'static str {
        "unknown"
    }
}
