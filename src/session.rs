use crate::object::{
    CachedObjectSource, ObjectSource, ObjectId, FsObjectAccessor, Key,
};
use crate::error::QuocoError;
use crate::util::{delete_file, is_shred_available, shred_file};
use crate::UuidBytes;
use lazy_static::lazy_static;
use owning_ref::{MutexGuardRef, OwningRef};
use std::cell::RefCell;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use tempfile::NamedTempFile;
use uuid::Uuid;

lazy_static! {
    pub static ref SESSIONS: Mutex<HashMap<UuidBytes, RefCell<Session<FsObjectAccessor >>>> =
        Mutex::new(HashMap::new());
}

type SessionMutexGuard<'a> = OwningRef<
    MutexGuard<'a, HashMap<UuidBytes, RefCell<Session<FsObjectAccessor>>, RandomState>>,
    RefCell<Session<FsObjectAccessor>>,
>;

pub fn new_session(path: &str, key: &Key) -> Result<UuidBytes, QuocoError> {
    let uuid = *Uuid::new_v4().as_bytes();
    let new_session = Session::new(FsObjectAccessor::open(
        PathBuf::from(path).as_path(),
        key,
    )?)?;
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

pub struct Session<D: ObjectSource> {
    pub cache: CachedObjectSource<D>,
    temp_files: HashMap<ObjectId, PathBuf>,
}

impl<D: ObjectSource> Session<D> {
    pub fn new(accessor: D) -> Result<Self, QuocoError> {
        Ok(Session {
            cache: CachedObjectSource::new(accessor),
            temp_files: HashMap::new(),
        })
    }

    pub fn object_temp_file(&mut self, id: &ObjectId) -> Result<PathBuf, QuocoError> {
        if self.temp_files.contains_key(id) {
            return Ok(self.temp_files[id].clone());
        }

        let mut temp_file = NamedTempFile::new()?;
        io::copy(&mut self.cache.object(id).unwrap(), &mut temp_file)?;
        Ok(temp_file.path().into())
    }

    pub fn clear_temp_files(&mut self) -> Result<(), QuocoError> {
        let can_shred = is_shred_available();
        let shred_successful = self
            .temp_files
            .drain()
            .map(|f| {
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
