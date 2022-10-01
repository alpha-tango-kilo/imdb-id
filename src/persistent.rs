use std::{
    borrow::Cow,
    fs::{
        File,
        OpenOptions,
    },
    io,
    io::{
        BufReader,
        Write,
    },
    path::PathBuf,
};

use once_cell::sync::Lazy;
use serde::{
    Deserialize,
    Serialize,
};

use crate::DiskError;

static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let mut config_path =
        dirs::config_dir().expect("Platform unsupported by dirs");
    config_path.push("imdb-id.json");
    config_path
});

type Result<T, E = DiskError> = std::result::Result<T, E>;

#[derive(Debug, Serialize, Deserialize)]
pub struct OnDiskConfig<'a> {
    pub api_key: Cow<'a, str>,
}

impl<'a> OnDiskConfig<'a> {
    pub fn save(&self) -> Result<()> {
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

    pub fn load() -> Result<Self> {
        let file =
            File::open(CONFIG_PATH.as_path()).map_err(|err| {
                match err.kind() {
                    io::ErrorKind::NotFound => {
                        DiskError::NotFound(CONFIG_PATH.to_string_lossy())
                    },
                    _ => DiskError::Write(err),
                }
            })?;
        let config =
            serde_json::from_reader(BufReader::new(file)).map_err(|err| {
                DiskError::Deserialise(err, CONFIG_PATH.to_string_lossy())
            })?;
        Ok(config)
    }
}
