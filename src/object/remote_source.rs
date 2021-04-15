use crate::object::{GoogleStorageObjectSource, Key};
use crate::Result;
use std::path::PathBuf;

pub enum RemoteSourceConfig {
    GoogleStorage {
        bucket: String,
        config_path: PathBuf,
    },
}

pub enum RemoteSource {
    GoogleStorage(GoogleStorageObjectSource),
}

impl RemoteSource {
    pub fn initialize(config: RemoteSourceConfig, key: &Key) -> Result<Self> {
        Ok(match config {
            RemoteSourceConfig::GoogleStorage {
                bucket,
                config_path,
            } => Self::GoogleStorage(GoogleStorageObjectSource::open(
                &bucket,
                config_path.as_path(),
                key,
            )?),
        })
    }
}
