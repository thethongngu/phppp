use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub enable_laravel: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enable_laravel: false,
        }
    }
}

/// Load configuration from a `.phppprc` JSON file located at `root`.
pub fn load_config(root: &Path) -> std::io::Result<Config> {
    let path = root.join(".phppprc");
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(path)?;
    let cfg: Config = serde_json::from_str(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(cfg)
}
