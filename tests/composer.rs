use phppp::composer::load_autoload_paths;
use std::fs;
use tempfile::tempdir;

#[test]
fn parse_autoload_psr4() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("composer.json");
    fs::write(&path, r#"{"autoload": {"psr-4": {"App\\": "src/"}}}"#).unwrap();
    let map = load_autoload_paths(dir.path()).unwrap();
    assert_eq!(map.get("App\\").unwrap(), "src/");
}
