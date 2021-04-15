use crate::object::{
    CachedObjectSource, FsObjectSource, GoogleStorageObjectSource, Key, ObjectId, ObjectSource,
    RemoteSource, RemoteSourceConfig,
};
use crate::util::{bytes_to_hex_str, delete_file, is_shred_available, shred_file};
use crate::Result;
use crate::UuidBytes;
use lazy_static::lazy_static;
use owning_ref::{MutexGuardRef, OwningRef};
use std::cell::RefCell;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::{env, io};
use uuid::Uuid;

lazy_static! {
    pub static ref SESSIONS: Mutex<HashMap<UuidBytes, RefCell<Session<FsObjectSource, GoogleStorageObjectSource >>>> =
        Mutex::new(HashMap::new());
}

type SessionMutexGuard<'a> = OwningRef<
    MutexGuard<
        'a,
        HashMap<
            UuidBytes,
            RefCell<Session<FsObjectSource, GoogleStorageObjectSource>>,
            RandomState,
        >,
    >,
    RefCell<Session<FsObjectSource, GoogleStorageObjectSource>>,
>;

// TODO: Modularize this so remote backends can be swapped out (a factory enum might do just fine)
pub fn new_session(
    local_path: &str,
    key: &Key,
    remote_config: Option<RemoteSourceConfig>,
) -> Result<UuidBytes> {
    let uuid = *Uuid::new_v4().as_bytes();
    // TODO: Figure out how to just handle results inside of maps
    let mut remote_accessor = None;
    if let Some(c) = remote_config {
        let accessor_wrapper = RemoteSource::initialize(c, key)?;
        remote_accessor = Some(match accessor_wrapper {
            RemoteSource::GoogleStorage(accessor) => accessor,
        })
    };

    let new_session = Session::new(
        FsObjectSource::open(PathBuf::from(local_path).as_path(), key)?,
        remote_accessor,
    )?;
    SESSIONS
        .lock()
        .unwrap()
        .insert(uuid, RefCell::new(new_session));
    Ok(uuid)
}

pub fn get_session<'a>(id: &UuidBytes) -> SessionMutexGuard<'a> {
    MutexGuardRef::new(SESSIONS.lock().unwrap()).map(|g| g.get(id).unwrap())
}

pub fn close_session(id: &UuidBytes) -> bool {
    SESSIONS.lock().unwrap().remove(id).is_some()
}

pub fn clear_sessions() {
    // Sessions release locks when they are dropped
    SESSIONS.lock().unwrap().clear()
}

// TODO: Make a local-only variant of Session
pub struct Session<L: ObjectSource, R: ObjectSource> {
    pub local: CachedObjectSource<L>,
    pub remote: Option<CachedObjectSource<R>>,
    temp_files: HashMap<ObjectId, PathBuf>,
}

impl<L: ObjectSource, R: ObjectSource> Session<L, R> {
    pub fn new(accessor: L, remote: Option<R>) -> Result<Self> {
        Ok(Session {
            local: CachedObjectSource::new(accessor),
            remote: if let Some(remote) = remote {
                Some(CachedObjectSource::new(remote))
            } else {
                None
            },
            temp_files: HashMap::new(),
        })
    }

    pub fn object_temp_file(&mut self, id: &ObjectId) -> Result<PathBuf> {
        if self.temp_files.contains_key(id) {
            return Ok(self.temp_files[id].clone());
        }

        let temp_file_path = env::temp_dir().join(Path::new(&bytes_to_hex_str(id)));
        io::copy(
            &mut self.local.object(id)?,
            &mut File::create(temp_file_path.clone())?,
        )?;
        self.temp_files.insert(*id, temp_file_path.clone());

        Ok(temp_file_path)
    }

    pub fn clear_temp_files(&mut self) -> Result<()> {
        let can_shred = is_shred_available();
        let local = &mut self.local;
        let shred_successful = self
            .temp_files
            .drain()
            .map(|f| {
                local
                    .modify_object(
                        &f.0,
                        &mut File::open(f.1.clone()).expect("Couldn't open temp file"),
                    )
                    .expect("Failed to modify object from temp");

                if can_shred {
                    return shred_file(f.1.as_path());
                }

                delete_file(f.1.as_path())
            })
            .all(|i| i);
        if !shred_successful {
            return Err(
                io::Error::new(io::ErrorKind::Other, "Failed to clear all temp files.").into(),
            );
        }
        Ok(())
    }
}

impl<L: ObjectSource, R: ObjectSource> Drop for Session<L, R> {
    fn drop(&mut self) {
        // TODO: Fix error handling on this whole thing
        self.clear_temp_files().expect("Failed to clear temp files")
    }
}
