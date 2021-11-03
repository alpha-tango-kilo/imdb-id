use crate::get_api_key;
use crate::omdb::test_api_key;
use crate::Result;
use reqwest::blocking::Client;
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
    pub fn new_from_prompt(client: &Client) -> Result<Self> {
        let api_key = get_api_key(client)?;
        Ok(OnDiskConfig { api_key })
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
        Ok(config)
    }

    pub fn check(&self, client: &Client) -> std::result::Result<(), String> {
        test_api_key(&self.api_key, client)
    }

    pub fn validate(&mut self, client: &Client) -> Result<()> {
        if let Err(why) = self.check(client) {
            eprintln!("{}", why);
            self.api_key = get_api_key(client)?;
        }
        Ok(())
    }

    fn config_path() -> PathBuf {
        let mut config_path = dirs::config_dir().expect("Platform unsupported by dirs");
        config_path.push("imdb-id.json");
        config_path
    }
}