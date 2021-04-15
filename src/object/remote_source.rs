use crate::object::{GoogleStorageObjectSource, Key};
use crate::Result;
use std::path::PathBuf;

pub enum RemoteAccessorConfig {
    GoogleStorage {
        bucket: String,
        config_path: PathBuf,
    },
}

pub enum RemoteAccessor {
    GoogleStorage(GoogleStorageObjectSource),
}

impl RemoteAccessor {
    pub fn initialize(config: RemoteAccessorConfig, key: &Key) -> Result<Self> {
        Ok(match config {
            RemoteAccessorConfig::GoogleStorage {
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
