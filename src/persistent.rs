use crate::omdb::test_api_key;
use crate::{get_api_key, ApiKeyError, DiskError, InteractivityError};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, Write};
use std::ops::Deref;
use std::path::PathBuf;

lazy_static! {
    static ref CONFIG_PATH: PathBuf = {
        let mut config_path =
            dirs::config_dir().expect("Platform unsupported by dirs");
        config_path.push("imdb-id.json");
        config_path
    };

    // Used for errors
    static ref CONFIG_PATH_COW: Cow<'static, str> = CONFIG_PATH.to_string_lossy();
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OnDiskConfig {
    pub api_key: String,
}

impl OnDiskConfig {
    pub fn new_from_prompt() -> Result<Self, InteractivityError> {
        let api_key = get_api_key()?;
        Ok(OnDiskConfig { api_key })
    }

    pub fn save(&self) -> Result<(), DiskError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(CONFIG_PATH.as_path())
            .map_err(DiskError::Write)?;

        let ser = serde_json::to_string_pretty(&self)
            .map_err(DiskError::Serialise)?;
        file.write_all(ser.as_bytes()).map_err(DiskError::Write)
    }

    pub fn load() -> Result<Self, DiskError> {
        let file =
            File::open(CONFIG_PATH.as_path()).map_err(|err| {
                match err.kind() {
                    io::ErrorKind::NotFound => {
                        DiskError::NotFound(CONFIG_PATH_COW.deref())
                    }
                    _ => DiskError::Write(err),
                }
            })?;
        let config =
            serde_json::from_reader(BufReader::new(file)).map_err(|err| {
                DiskError::Deserialise(err, CONFIG_PATH_COW.deref())
            })?;
        Ok(config)
    }

    pub fn check(&self) -> Result<(), ApiKeyError> {
        test_api_key(&self.api_key)
    }

    pub fn validate(&mut self) -> Result<(), InteractivityError> {
        if let Err(why) = self.check() {
            eprintln!("{why}");
            self.api_key = get_api_key()?;
        }
        Ok(())
    }
}
