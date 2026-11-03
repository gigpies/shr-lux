//! Private crash-safe owner checkpoint and durably reserved restart epochs.
use crate::fixture::{Id, Patch, PatchSpec, Value};
use crate::lighting_contract::{StoredKind, validate_show_id};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
const LIMIT: usize = 16 * 65536;
const CHECKPOINT: &str = "lighting-checkpoint.json";
const EPOCH: &str = "lighting-epoch.json";
const CONFIG: &str = "lighting-startup.json";
static SCRATCH: AtomicU64 = AtomicU64::new(0);
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedLook {
    pub kind: StoredKind,
    pub id: Id,
    pub values: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedMaster {
    pub fixture: Id,
    pub level: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableState {
    pub format: String,
    pub version: u16,
    pub show_id: String,
    #[serde(with = "crate::wire::counter")]
    pub epoch: u64,
    #[serde(with = "crate::wire::counter")]
    pub revision: u64,
    pub patch: PatchSpec,
    pub stores: Vec<SavedLook>,
    pub manual_hold: Vec<Value>,
    pub intended_look: Vec<Value>,
    pub master: i32,
    pub fixture_masters: Vec<SavedMaster>,
    pub blackout: bool,
}
impl DurableState {
    pub fn validate(&self, show: &str) -> io::Result<Patch> {
        if self.format != "shr-lux-checkpoint" || self.version != 1 {
            return Err(invalid("checkpoint version"));
        }
        validate_show_id(&self.show_id).map_err(|_| invalid("show id"))?;
        if self.show_id != show {
            return Err(invalid("wrong show"));
        }
        let patch = Patch::validate(self.patch.clone()).map_err(|_| invalid("patch"))?;
        validate_values(&patch, &self.manual_hold)?;
        validate_values(&patch, &self.intended_look)?;
        let expected: usize = patch.fixtures().iter().map(|f| f.capabilities.len()).sum();
        if self.intended_look.len() != expected {
            return Err(invalid("incomplete intended look"));
        }
        let mut cues = BTreeSet::new();
        let mut palettes = BTreeSet::new();
        for s in &self.stores {
            let ids = match s.kind {
                StoredKind::Cue => &mut cues,
                StoredKind::Palette => &mut palettes,
            };
            if !ids.insert(s.id.clone()) || ids.len() > 32 {
                return Err(invalid("store capacity/duplicate"));
            }
            validate_values(&patch, &s.values)?;
        }
        if !(0..=1000).contains(&self.master) {
            return Err(invalid("master"));
        }
        let mut fixtures = BTreeSet::new();
        for m in &self.fixture_masters {
            if !fixtures.insert(&m.fixture)
                || patch.fixture(&m.fixture).is_none()
                || !(0..=1000).contains(&m.level)
            {
                return Err(invalid("fixture master"));
            }
        }
        Ok(patch)
    }
}
fn validate_values(patch: &Patch, values: &[Value]) -> io::Result<()> {
    if values.len() > 224 {
        return Err(invalid("mask capacity"));
    }
    for f in patch.fixtures() {
        let part = values
            .iter()
            .filter(|v| v.fixture == f.id)
            .cloned()
            .collect::<Vec<_>>();
        patch.validate_values(&part).map_err(|_| invalid("mask"))?;
    }
    if values.iter().any(|v| patch.fixture(&v.fixture).is_none()) {
        return Err(invalid("mask target"));
    }
    Ok(())
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Epoch {
    format: String,
    version: u16,
    show_id: String,
    #[serde(with = "crate::wire::counter")]
    epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteFault {
    None,
    BeforeRename,
    AfterRename,
}
/// Holding this object retains the exclusive owner lock for the authority lifetime.
pub struct CheckpointStore {
    directory: PathBuf,
    show: String,
    epoch: u64,
    configured_patch: Option<PatchSpec>,
    _lock: OwnerLock,
}
impl CheckpointStore {
    pub fn open(directory: &Path, show: &str) -> io::Result<Self> {
        Self::open_inner(directory, show, None)
    }
    pub fn open_configured(
        directory: &Path,
        config: &crate::service_config::ServiceConfig,
    ) -> io::Result<Self> {
        config.validate().map_err(invalid)?;
        Self::open_inner(directory, &config.show_id, Some(config))
    }
    fn open_inner(
        directory: &Path,
        show: &str,
        config: Option<&crate::service_config::ServiceConfig>,
    ) -> io::Result<Self> {
        crate::local_service::private_directory(directory)?;
        validate_show_id(show).map_err(|_| invalid("show id"))?;
        let had_lock = fs::symlink_metadata(directory.join("lighting-owner.lock")).is_ok();
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(directory.join("lighting-owner.lock"))?;
        private_file(&lock)?;
        // SAFETY: valid retained file descriptor, nonblocking exclusive advisory lock.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Self::open_locked(directory, show, config, had_lock, OwnerLock(lock))
    }
    fn open_locked(
        directory: &Path,
        show: &str,
        config: Option<&crate::service_config::ServiceConfig>,
        had_lock: bool,
        lock: OwnerLock,
    ) -> io::Result<Self> {
        // Startup identity is checked under the same retained lock, before epoch
        // reservation. Legacy callers cannot silently enter configured storage.
        let identity_exists = fs::symlink_metadata(directory.join(CONFIG)).is_ok();
        if let Some(config) = config {
            if identity_exists {
                let saved =
                    crate::service_config::ServiceConfig::decode(&read_private(directory, CONFIG)?)
                        .map_err(invalid)?;
                if &saved != config {
                    return Err(invalid("startup identity mismatch"));
                }
            } else if fs::read_dir(directory)?
                .any(|entry| entry.map_or(true, |e| e.file_name() != "lighting-owner.lock"))
                || had_lock
            {
                return Err(invalid(
                    "configured startup requires fresh private directory",
                ));
            }
        } else if identity_exists {
            return Err(invalid("configured startup identity required"));
        }
        let prior = match read_private(directory, EPOCH) {
            Ok(bytes) => {
                let raw = crate::wire::strict_json(&bytes).map_err(invalid)?;
                let prior: Epoch =
                    serde_json::from_value(raw).map_err(|_| invalid("epoch schema"))?;
                if prior.format != "shr-lux-epoch" || prior.version != 1 || prior.show_id != show {
                    return Err(invalid("epoch identity/version"));
                }
                prior.epoch
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if had_lock || fs::symlink_metadata(directory.join(CHECKPOINT)).is_ok() {
                    return Err(invalid("checkpoint exists without epoch registry"));
                }
                0
            }
            Err(e) => return Err(e),
        };
        // Refuse mismatched/corrupt checkpoint before consuming epoch or modifying any file.
        if let Ok(bytes) = read_private(directory, CHECKPOINT) {
            let state = decode(&bytes)?;
            state.validate(show)?;
            if config.is_some_and(|c| c.patch != state.patch) {
                return Err(invalid("configured checkpoint patch mismatch"));
            }
            if state.epoch > prior {
                return Err(invalid("checkpoint newer than epoch registry"));
            }
        } else if fs::symlink_metadata(directory.join(CHECKPOINT)).is_ok() {
            return Err(invalid("unreadable checkpoint"));
        }
        let epoch = prior
            .checked_add(1)
            .ok_or_else(|| invalid("epoch exhausted"))?;
        let record = Epoch {
            format: "shr-lux-epoch".into(),
            version: 1,
            show_id: show.into(),
            epoch,
        };
        if let Some(config) = config
            && !identity_exists
        {
            atomic_replace(
                directory,
                CONFIG,
                &serde_json::to_vec(config)?,
                WriteFault::None,
            )?;
        }
        atomic_replace(
            directory,
            EPOCH,
            &serde_json::to_vec(&record)?,
            WriteFault::None,
        )?;
        Ok(Self {
            directory: directory.to_owned(),
            show: show.into(),
            epoch,
            configured_patch: config.map(|c| c.patch.clone()),
            _lock: lock,
        })
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn show_id(&self) -> &str {
        &self.show
    }
    pub fn load(&self) -> io::Result<Option<DurableState>> {
        match read_private(&self.directory, CHECKPOINT) {
            Ok(bytes) => {
                let state = decode(&bytes)?;
                state.validate(&self.show)?;
                if self
                    .configured_patch
                    .as_ref()
                    .is_some_and(|p| p != &state.patch)
                {
                    return Err(invalid("configured checkpoint patch mismatch"));
                }
                if state.epoch >= self.epoch {
                    return Err(invalid("checkpoint epoch"));
                }
                Ok(Some(state))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn load_scene_map(&self, patch: &Patch) -> io::Result<Option<crate::scenes::ShowMap>> {
        match read_private(&self.directory, "lighting-scenes.json") {
            Ok(bytes) => {
                let raw = crate::wire::strict_json_bounded(&bytes, LIMIT).map_err(invalid)?;
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Saved {
                    show_id: String,
                    map: crate::scenes::ShowMap,
                }
                let saved: Saved =
                    serde_json::from_value(raw).map_err(|_| invalid("scene map schema"))?;
                if saved.show_id != self.show {
                    return Err(invalid("scene map show"));
                }
                saved.map.validate(patch).map_err(invalid)?;
                Ok(Some(saved.map))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub fn save_scene_map(&self, map: &crate::scenes::ShowMap, patch: &Patch) -> io::Result<()> {
        map.validate(patch).map_err(invalid)?;
        atomic_replace(
            &self.directory,
            "lighting-scenes.json",
            &serde_json::to_vec(&serde_json::json!({"show_id":self.show,"map":map}))?,
            WriteFault::None,
        )
    }
    pub fn checkpoint(&self, state: &DurableState) -> io::Result<()> {
        self.checkpoint_with_fault(state, WriteFault::None)
    }
    /// Fault injection applies only to owned private test storage; never changes sync policy.
    pub fn checkpoint_with_fault(&self, state: &DurableState, fault: WriteFault) -> io::Result<()> {
        state.validate(&self.show)?;
        if self
            .configured_patch
            .as_ref()
            .is_some_and(|p| p != &state.patch)
        {
            return Err(invalid("configured checkpoint patch mismatch"));
        }
        if state.epoch != self.epoch {
            return Err(invalid("wrong checkpoint epoch"));
        }
        if let Ok(bytes) = read_private(&self.directory, CHECKPOINT) {
            decode(&bytes)?.validate(&self.show)?;
        } else if fs::symlink_metadata(self.directory.join(CHECKPOINT)).is_ok() {
            return Err(invalid("unreadable checkpoint"));
        }
        let bytes = serde_json::to_vec(state)?;
        if bytes.len() > LIMIT {
            return Err(invalid("checkpoint capacity"));
        }
        atomic_replace(&self.directory, CHECKPOINT, &bytes, fault)
    }
}
fn decode(bytes: &[u8]) -> io::Result<DurableState> {
    let value = crate::wire::strict_json_bounded(bytes, LIMIT).map_err(invalid)?;
    serde_json::from_value(value).map_err(|_| invalid("checkpoint schema"))
}
fn private_file(file: &File) -> io::Result<()> {
    let m = file.metadata()?;
    // SAFETY: geteuid has no pointer arguments.
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o777 != 0o600 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "owned0600 file required",
        ));
    }
    Ok(())
}
fn read_private(directory: &Path, name: &str) -> io::Result<Vec<u8>> {
    // Nonblocking open prevents a replaced FIFO from hanging before metadata validation.
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join(name))?;
    private_file(&file)?;
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > LIMIT {
        return Err(invalid("file capacity"));
    }
    Ok(bytes)
}
fn atomic_replace(directory: &Path, name: &str, bytes: &[u8], fault: WriteFault) -> io::Result<()> {
    let generation = SCRATCH
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
        .map_err(|_| invalid("scratch counter exhausted"))?;
    let scratch = directory.join(format!(".lux-{}-{generation}.tmp", std::process::id()));
    let mut created = false;
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&scratch)?;
        created = true;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        if fault == WriteFault::BeforeRename {
            return Err(io::Error::other("injected before rename"));
        }
        fs::rename(&scratch, directory.join(name))?;
        if fault == WriteFault::AfterRename {
            return Err(io::Error::other(
                "injected directory sync failure; replacement durability uncertain",
            ));
        }
        File::open(directory)?.sync_all()?;
        Ok(())
    })();
    // Only our successfully created temp can be removed; never remove a collision.
    if result.is_err() && created && fs::symlink_metadata(&scratch).is_ok() {
        let _ = fs::remove_file(&scratch);
    }
    result
}

// Construct only after successful acquisition, before any fallible startup work.
// Closing alone can retain flock ownership in a fork-inherited descriptor.
struct OwnerLock(File);
impl Drop for OwnerLock {
    fn drop(&mut self) {
        // SAFETY: valid retained descriptor. Explicit unlock also ends ownership
        // if another thread briefly forked with a CLOEXEC descriptor copy.
        let _ = unsafe { libc::flock(self.0.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    use crate::fixture::{FixtureSpec, SyntheticMode};
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn refused_startup_unlocks_even_with_an_inherited_descriptor() {
        let directory =
            std::env::temp_dir().join(format!("lux-lock-refusal-{}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                fs::remove_dir_all(&self.0).unwrap();
            }
        }
        let _cleanup = Cleanup(directory.clone());
        let config = crate::service_config::ServiceConfig {
            version: 1,
            show_id: "11111111-1111-4111-8111-111111111111".into(),
            patch: PatchSpec {
                version: 1,
                patch_revision: 1,
                fixtures: vec![FixtureSpec::synthetic(
                    Id::new("front").unwrap(),
                    "Front",
                    SyntheticMode::Dimmer,
                    1,
                )],
            },
            analysis: None,
        };
        drop(CheckpointStore::open_configured(&directory, &config).unwrap());
        let before = [CONFIG, EPOCH].map(|name| fs::read(directory.join(name)).unwrap());
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(directory.join("lighting-owner.lock"))
            .unwrap();
        // SAFETY: live descriptor. Retain the same open-file description that a
        // concurrent fork would inherit until exec, without forking the test harness.
        assert_eq!(
            unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        let inherited = lock.try_clone().unwrap();
        // Exercise the real refused legacy-startup path after lock acquisition.
        let error =
            CheckpointStore::open_locked(&directory, &config.show_id, None, true, OwnerLock(lock))
                .err()
                .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            [CONFIG, EPOCH].map(|name| fs::read(directory.join(name)).unwrap()),
            before
        );
        let recovered = CheckpointStore::open_configured(&directory, &config).unwrap();
        assert_eq!(recovered.epoch(), 2);
        drop(inherited);
        // Dropping the old duplicate must not release the new owner's lock.
        assert_eq!(
            CheckpointStore::open_configured(&directory, &config)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::WouldBlock
        );
        drop(recovered);
        assert_eq!(
            CheckpointStore::open_configured(&directory, &config)
                .unwrap()
                .epoch(),
            3
        );
    }
}
