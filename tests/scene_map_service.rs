use serde_json::{Value as Json, json};
use shr_lux::{authority::Authority, fixture::*, recovery::CheckpointStore, wire::Service};
use std::{
    os::unix::fs::PermissionsExt,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
static N: AtomicUsize = AtomicUsize::new(0);
struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "lux-scenes-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(p)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn patch() -> Patch {
    Patch::validate(PatchSpec {
        version: 1,
        patch_revision: 1,
        fixtures: vec![FixtureSpec::synthetic(
            Id::new("wash").unwrap(),
            "Wash",
            SyntheticMode::Rgb,
            1,
        )],
    })
    .unwrap()
}
const SHOW: &str = "11111111-1111-4111-8111-111111111111";
fn request(s: &Service, kind: &str, n: u64, body: Json) -> Json {
    json!({"contract":"C-LIGHT","version":1,"show_id":SHOW,"module":"lighting","epoch":s.engine().snapshot().epoch.to_string(),"writer":"editor","lease":if kind=="grant"{None}else{Some("1")},"request_id":n.to_string(),"expected_revision":s.engine().revision().to_string(),"kind":kind,"body":body})
}
fn send(s: &mut Service, kind: &str, n: u64, body: Json) -> Json {
    let r = request(s, kind, n, body);
    s.handle(&serde_json::to_vec(&r).unwrap())[0].clone()
}
fn service(store: Arc<CheckpointStore>) -> Service {
    let mut s = Service::reserved(store, patch(), true).unwrap();
    s.enable_analysis().unwrap();
    s.enable_musical_analysis().unwrap();
    s.enable_scene_director().unwrap();
    s
}
#[test]
fn actual_atomic_editor_revision_preview_persistence_and_recovery() {
    let dir = Directory::new();
    let store = Arc::new(CheckpointStore::open(&dir.0, SHOW).unwrap());
    let mut s = service(store.clone());
    assert_eq!(
        send(&mut s, "grant", 1, json!({"scope":"lighting-control"}))["kind"],
        "applied"
    );
    let original = s.inventory().unwrap();
    let mut map = original["scenes"]["map"].clone();
    map["revision"] = "2".into();
    map["scenes"][0]["mood"] = "warm".into();
    let revision = s.engine().revision();
    assert_eq!(
        send(
            &mut s,
            "command",
            2,
            json!({"scope":"lighting-control","command":{"action":"scene_map_replace","map":map}})
        )["kind"],
        "applied"
    );
    assert!(s.engine().revision() > revision);
    let new = s.inventory().unwrap();
    assert_eq!(new["scenes"]["map"]["revision"], "2");
    let rev = s.engine().revision();
    let preview = send(
        &mut s,
        "command",
        3,
        json!({"scope":"lighting-control","command":{"action":"scene_preview","scene_id":"intimate"}}),
    );
    assert_eq!(preview["kind"], "applied");
    assert_eq!(preview["body"]["application"], "scene_preview");
    assert_eq!(s.engine().revision(), rev);
    let old = send(
        &mut s,
        "command",
        4,
        json!({"scope":"lighting-control","command":{"action":"scene_map_replace","map":original["scenes"]["map"]}}),
    );
    assert_eq!(old["kind"], "rejected");
    assert_eq!(old["reason"], "scene_map_revision");
    let mut invalid = new["scenes"]["map"].clone();
    invalid["revision"] = "3".into();
    invalid["roles"][0]["fixtures"][0] = "absent".into();
    assert_eq!(
        send(
            &mut s,
            "command",
            5,
            json!({"scope":"lighting-control","command":{"action":"scene_map_replace","map":invalid}})
        )["kind"],
        "rejected"
    );
    assert_eq!(
        s.inventory().unwrap()["scenes"]["map"],
        new["scenes"]["map"]
    );
    drop(s);
    drop(store);
    let store = Arc::new(CheckpointStore::open(&dir.0, SHOW).unwrap());
    let restored = service(store);
    let recovered = restored.inventory().unwrap();
    assert_eq!(recovered["scenes"]["map"], new["scenes"]["map"]);
    assert!(recovered["scenes"]["grant"].is_null());
    assert!(recovered["scenes"]["current"].is_null());
    assert_eq!(recovered["mode"], "manual");
}
#[test]
fn legacy_remains_unchanged_and_opt_in_requires_music() {
    let a = Authority::new(patch(), SHOW, 1).unwrap();
    let mut s = Service::new(a).unwrap();
    let legacy = s.inventory().unwrap();
    assert!(legacy.get("scenes").is_none());
    assert!(s.enable_scene_director().is_err());
    s.enable_analysis().unwrap();
    assert!(s.enable_scene_director().is_err());
    s.enable_musical_analysis().unwrap();
    s.enable_scene_director().unwrap();
    assert_eq!(s.inventory().unwrap()["wire_schema"], "lx06-scenes-v3");
}
#[test]
fn malformed_recovered_map_refuses_startup() {
    let dir = Directory::new();
    let store = Arc::new(CheckpointStore::open(&dir.0, SHOW).unwrap());
    std::fs::write(
        dir.0.join("lighting-scenes.json"),
        b"{\"show_id\":\"wrong\",\"map\":{}}",
    )
    .unwrap();
    std::fs::set_permissions(
        dir.0.join("lighting-scenes.json"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    let mut s = Service::reserved(store, patch(), true).unwrap();
    s.enable_analysis().unwrap();
    s.enable_musical_analysis().unwrap();
    assert!(s.enable_scene_director().is_err());
    assert!(s.inventory().unwrap().get("scenes").is_none());
}
