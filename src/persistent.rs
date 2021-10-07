use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct OnDiskConfig {
    pub api_key: String,
}

impl OnDiskConfig {
    pub fn new(api_key: &str) -> Self {
        // Test API key?
        OnDiskConfig {
            api_key: api_key.to_owned(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(OnDiskConfig::config_path())?;

        let bytes = serde_json::to_vec(&self)?;
        file.write_all(&bytes)
    }

    pub fn load() -> std::io::Result<Self> {
        let bytes = fs::read(OnDiskConfig::config_path())?;
        let config = serde_json::from_slice(&bytes)?;
        println!("Loaded config successfully");
        Ok(config)
    }

    fn config_path() -> PathBuf {
        let mut config_path = dirs::config_dir().expect("Platform unsupported by dirs");
        config_path.push("imdb-id.json");
        config_path
    }
}
