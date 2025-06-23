use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct Composer {
    #[serde(default)]
    autoload: Autoload,
}

#[derive(Deserialize, Default)]
struct Autoload {
    #[serde(rename = "psr-4", default)]
    psr4: HashMap<String, String>,
}

/// Load PSR-4 autoload namespace mappings from a `composer.json` file
/// located at `root`.
pub fn load_autoload_paths(root: &Path) -> std::io::Result<HashMap<String, String>> {
    let path = root.join("composer.json");
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read_to_string(path)?;
    let composer: Composer = serde_json::from_str(&data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(composer.autoload.psr4)
}
