use serde_json::{Value, json};
use shr_lux::{
    analysis_subscription::{AnalysisState, Binding, Client, Descriptor},
    authority::{Authority, Command},
    fixture::{Attribute, FixtureSpec, Id, PatchSpec, SyntheticMode, Value as FixtureValue},
    recovery::CheckpointStore,
    service_config::ServiceConfig,
    wire::{Service, validate_configured_analysis},
};
use std::{
    io::{Read, Write},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
fn binding() -> Binding {
    Binding {
        sources: vec![],
        version: 2,
        inputs: ["input-17", "input-03", "input-09", "input-01"]
            .map(String::from)
            .to_vec(),
    }
}
fn config() -> ServiceConfig {
    ServiceConfig {
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
                FixtureSpec::synthetic(Id::new("back").unwrap(), "Back", SyntheticMode::Rgb, 12),
            ],
        },
        analysis: Some(binding()),
    }
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "lux-configured-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn descriptor_value() -> Value {
    serde_json::from_str(include_str!("fixtures/lx05/v2/configured-descriptor.json")).unwrap()
}
#[test]
fn startup_strict_nested_schema_and_nonfixture_patch() {
    let c = config();
    let bytes = serde_json::to_vec(&c).unwrap();
    assert_eq!(ServiceConfig::decode(&bytes).unwrap(), c);
    for pointer in [
        "/extra",
        "/patch/extra",
        "/patch/fixtures/0/extra",
        "/patch/fixtures/1/capabilities/0/extra",
        "/analysis/extra",
    ] {
        let mut v = serde_json::to_value(&c).unwrap();
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        v.pointer_mut(parent).unwrap()[key] = json!(true);
        assert!(
            ServiceConfig::decode(&serde_json::to_vec(&v).unwrap()).is_err(),
            "{pointer}"
        );
    }
    let text = String::from_utf8(bytes).unwrap();
    for changed in [
        text.replacen("\"version\":1", "\"version\":1,\"version\":1", 1),
        text.replacen("\"min\":0", "\"min\":0,\"min\":0", 1),
    ] {
        assert!(ServiceConfig::decode(changed.as_bytes()).is_err());
    }
    assert!(ServiceConfig::decode(&vec![b' '; 65537]).is_err());
    for id in [
        "input-001",
        "input-0",
        "input-65536",
        "input-+1",
        "input-03",
    ] {
        let mut b = binding();
        b.inputs[0] = id.into();
        assert!(b.validate().is_err());
    }
    let mut b = binding();
    b.inputs[0] = "input-65535".into();
    b.validate().unwrap();
}
#[test]
fn configured_descriptor_requires_exact_order_and_explicit_version() {
    let value = descriptor_value();
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(Descriptor::decode(&bytes).is_err());
    let descriptor = Descriptor::decode_configured(&bytes, &binding()).unwrap();
    assert_eq!(descriptor.inputs, binding().inputs);
    for (key, replacement) in [
        ("version", json!(1)),
        ("subscription", json!("lux.aux.v1")),
        (
            "inputs",
            json!(["input-03", "input-17", "input-09", "input-01"]),
        ),
        ("map_revision", json!("0")),
        ("extra", json!(true)),
    ] {
        let mut changed = value.clone();
        changed[key] = replacement;
        assert!(
            Descriptor::decode_configured(&serde_json::to_vec(&changed).unwrap(), &binding())
                .is_err()
        );
    }
}
#[test]
fn configured_source_absent_has_pinned_binding_and_legacy_cannot_relabel() {
    let c = config();
    let mut service =
        Service::new(Authority::new(c.validate().unwrap(), &c.show_id, 9).unwrap()).unwrap();
    service.enable_configured_analysis(binding()).unwrap();
    let inventory = service.inventory().unwrap();
    assert_eq!(inventory["wire_schema"], "lx05-v2");
    assert_eq!(inventory["analysis_binding"], json!(binding()));
    assert_eq!(inventory["analysis"]["analysis_binding"], json!(binding()));
    assert_eq!(inventory["analysis"]["state"], "absent");
    assert_eq!(inventory["snapshot"]["show_id"], c.show_id);
    assert_eq!(inventory["patch"]["fixtures"].as_array().unwrap().len(), 2);
    assert_eq!(inventory["analysis"]["grant"], Value::Null);
    assert!(service.enable_analysis().is_err());
    validate_configured_analysis(&inventory).unwrap();
    let mut retained = inventory.clone();
    retained["analysis"]["automatic_layer"]["values"] = json!([{"provenance":{"analysis_version":"lux.analysis.v1","source_identity":descriptor_value()}}]);
    validate_configured_analysis(&retained).unwrap();
    retained["analysis"]["automatic_layer"]["values"][0]["provenance"]["source_identity"]["inputs"] =
        json!(["input-03", "input-17", "input-09", "input-01"]);
    assert!(validate_configured_analysis(&retained).is_err());
}
#[test]
fn locked_config_identity_and_checkpoint_recovery_preserve_show_holds_without_rearm() {
    let dir = Directory::new();
    let c = config();
    let store = Arc::new(CheckpointStore::open_configured(&dir.0, &c).unwrap());
    let mut authority = Authority::new(c.validate().unwrap(), &c.show_id, store.epoch()).unwrap();
    authority
        .execute(Command::Touch(vec![FixtureValue {
            fixture: Id::new("front").unwrap(),
            attribute: Attribute::Intensity,
            value: 730,
        }]))
        .unwrap();
    authority.execute(Command::ClearToHold).unwrap();
    let state = authority.durable_state();
    store.checkpoint(&state).unwrap();
    assert!(CheckpointStore::open_configured(&dir.0, &c).is_err());
    drop(store);
    let names = [
        "lighting-startup.json",
        "lighting-epoch.json",
        "lighting-checkpoint.json",
    ];
    let before: Vec<_> = names
        .iter()
        .map(|n| std::fs::read(dir.0.join(n)).unwrap())
        .collect();
    for variant in 0..3 {
        let mut changed = c.clone();
        match variant {
            0 => changed.show_id = "33333333-3333-4333-8333-333333333333".into(),
            1 => changed.patch.fixtures[0].label = "Other".into(),
            _ => changed.analysis.as_mut().unwrap().inputs.swap(0, 1),
        }
        assert!(CheckpointStore::open_configured(&dir.0, &changed).is_err());
        for (name, bytes) in names.iter().zip(&before) {
            assert_eq!(std::fs::read(dir.0.join(name)).unwrap(), *bytes);
        }
    }
    assert!(CheckpointStore::open(&dir.0, &c.show_id).is_err());
    let store = Arc::new(CheckpointStore::open_configured(&dir.0, &c).unwrap());
    assert_eq!(store.epoch(), 2);
    let mut service = Service::recover(store, c.validate().unwrap()).unwrap();
    service.enable_configured_analysis(binding()).unwrap();
    let value = service.inventory().unwrap();
    assert_eq!(value["snapshot"]["show_id"], c.show_id);
    assert_eq!(
        value["snapshot"]["fixtures"][0]["attributes"][0]["hold"],
        730
    );
    assert_eq!(value["analysis"]["grant"], Value::Null);
    assert_eq!(value["mode"], "manual");
    assert_eq!(value["output"], "null_disarmed");
}
#[test]
fn configured_refuses_legacy_storage_and_checkpoint_patch_tamper_without_epoch_write() {
    let dir = Directory::new();
    let c = config();
    drop(CheckpointStore::open(&dir.0, &c.show_id).unwrap());
    let old = std::fs::read(dir.0.join("lighting-epoch.json")).unwrap();
    assert!(CheckpointStore::open_configured(&dir.0, &c).is_err());
    assert_eq!(
        old,
        std::fs::read(dir.0.join("lighting-epoch.json")).unwrap()
    );
    assert!(!dir.0.join("lighting-startup.json").exists());
    let other = Directory::new();
    let store = CheckpointStore::open_configured(&other.0, &c).unwrap();
    let authority = Authority::new(c.validate().unwrap(), &c.show_id, store.epoch()).unwrap();
    let mut state = authority.durable_state();
    store.checkpoint(&state).unwrap();
    drop(store);
    state.patch.fixtures[0].label = "tampered".into();
    std::fs::write(
        other.0.join("lighting-checkpoint.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    let old = std::fs::read(other.0.join("lighting-epoch.json")).unwrap();
    assert!(CheckpointStore::open_configured(&other.0, &c).is_err());
    assert_eq!(
        old,
        std::fs::read(other.0.join("lighting-epoch.json")).unwrap()
    );
}
#[test]
fn every_reconnect_checks_configured_mapping_before_admitting_descriptor() {
    let dir = Directory::new();
    let socket = dir.0.join("analysis.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    listener.set_nonblocking(true).unwrap();
    let client = Client::start_configured(&socket, binding()).unwrap();
    let mut state = AnalysisState::default();
    for correct in [true, false, true] {
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut stream = loop {
            match listener.accept() {
                Ok((s, _)) => break s,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut length = [0; 4];
        stream.read_exact(&mut length).unwrap();
        let mut request = vec![0; u32::from_be_bytes(length) as usize];
        stream.read_exact(&mut request).unwrap();
        assert_eq!(request, br#"{"subscription":"lux.aux.v2","version":2}"#);
        let mut descriptor = descriptor_value();
        if !correct {
            descriptor["inputs"] = json!(["input-03", "input-17", "input-09", "input-01"]);
        }
        let bytes = serde_json::to_vec(&descriptor).unwrap();
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&bytes).unwrap();
        if correct {
            loop {
                client.drain(&mut state, 1000);
                if state.descriptor.is_some() {
                    break;
                }
                assert!(Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(1));
            }
            assert_eq!(state.descriptor.as_ref().unwrap().inputs, binding().inputs);
        } else {
            for _ in 0..20 {
                client.drain(&mut state, 1000);
                assert!(state.descriptor.is_none());
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        drop(stream);
        for _ in 0..25 {
            client.drain(&mut state, 1000);
            if state.descriptor.is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(state.descriptor.is_none());
    }
}

#[test]
fn configured_missing_epoch_never_reuses_prior_identity() {
    let dir = Directory::new();
    let c = config();
    drop(CheckpointStore::open_configured(&dir.0, &c).unwrap());
    let identity = std::fs::read(dir.0.join("lighting-startup.json")).unwrap();
    std::fs::remove_file(dir.0.join("lighting-epoch.json")).unwrap();
    assert!(CheckpointStore::open_configured(&dir.0, &c).is_err());
    assert!(!dir.0.join("lighting-epoch.json").exists());
    assert_eq!(
        identity,
        std::fs::read(dir.0.join("lighting-startup.json")).unwrap()
    );
}

#[test]
fn configured_cli_refuses_invalid_startup_before_private_files() {
    let dir = Directory::new();
    let files = Directory::new();
    let path = files.0.join("config.json");
    std::fs::write(&path, serde_json::to_vec(&config()).unwrap()).unwrap();
    let binary = env!("CARGO_BIN_EXE_lux-service");
    let output = std::process::Command::new(binary)
        .args([
            "--synthetic-private-dir",
            dir.0.to_str().unwrap(),
            "--config",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires explicit --analysis"));
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
    std::fs::write(&path, br#"{"version":1,"version":1}"#).unwrap();
    let output = std::process::Command::new(binary)
        .args([
            "--synthetic-private-dir",
            dir.0.to_str().unwrap(),
            "--config",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read_dir(&dir.0).unwrap().count(), 0);
}
