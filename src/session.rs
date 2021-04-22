use crate::error::QuocoError;
use crate::error::QuocoError::{NoRemotes, TempFileDeletesFailed};
use crate::object::{
    BoxedObjectSource, CachedObjectSource, FsObjectSource, Key, ObjectId, ObjectSource,
    RemoteSource, RemoteSourceConfig,
};
use crate::util::{
    bytes_to_hex_str, delete_file, is_shred_available, shred_file, sync_primary_replica,
};
use crate::UuidBytes;
use crate::{ReadSeek, Result};
use lazy_static::lazy_static;
use owning_ref::{MutexGuardRef, OwningRef};
use std::cell::RefCell;
use std::collections::hash_map::RandomState;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::{env, io};
use uuid::Uuid;

lazy_static! {
    pub static ref SESSIONS: Mutex<HashMap<UuidBytes, RefCell<Session>>> =
        Mutex::new(HashMap::new());
}

type SessionMutexGuard<'a> =
    OwningRef<MutexGuard<'a, HashMap<UuidBytes, RefCell<Session>, RandomState>>, RefCell<Session>>;

// TODO: Modularize this so remote backends can be swapped out (a factory enum might do just fine)
pub fn new_session(
    local_path: &str,
    key: &Key,
    remote_config: Option<RemoteSourceConfig>,
) -> Result<UuidBytes> {
    let uuid = *Uuid::new_v4().as_bytes();
    let remote_accessor = remote_config
        .map(|c| {
            RemoteSource::initialize(c, key)
                .map(|w| match w {
                    RemoteSource::GoogleStorage(accessor) => Box::new(accessor),
                })
                .map(|b| b as BoxedObjectSource)
        })
        .transpose()?;

    let new_session = Session::open(
        Box::new(FsObjectSource::open(
            PathBuf::from(local_path).as_path(),
            key,
        )?),
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
pub struct Session {
    pub local: CachedObjectSource,
    pub remote: Option<CachedObjectSource>,
    temp_files: HashMap<ObjectId, PathBuf>,
}

impl Session {
    pub fn open(accessor: BoxedObjectSource, remote: Option<BoxedObjectSource>) -> Result<Self> {
        Ok(Session {
            local: CachedObjectSource::new(accessor),
            remote: remote.map(|s| CachedObjectSource::new(s)),
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
        let file_delete_errors = self
            .temp_files
            .drain()
            .map(|(id, path)| {
                (
                    path.clone(),
                    File::open(path.clone())
                        .map_err(QuocoError::from)
                        .map(|f| local.modify_object(&id, &mut (Box::new(f) as Box<dyn ReadSeek>)))
                        .and_then(|_| {
                            if can_shred {
                                shred_file(path.as_path())?;
                            } else {
                                delete_file(path.as_path())?;
                            }
                            Ok(())
                        }),
                )
            })
            .filter(|(_, result)| result.is_err())
            .map(|(path, result)| {
                (
                    path.into_os_string().into_string().unwrap(),
                    result.err().unwrap(),
                )
            })
            .collect::<Vec<(String, QuocoError)>>();

        if !file_delete_errors.is_empty() {
            return Err(TempFileDeletesFailed(file_delete_errors));
        }

        Ok(())
    }

    pub fn push_remote(&mut self) -> Result<()> {
        self.sync(SyncFrom::Local)
    }

    pub fn pull_remote(&mut self) -> Result<()> {
        self.sync(SyncFrom::Remote)
    }

    pub fn flush(&mut self) -> Result<()> {
        self.local.flush()?;
        if let Some(remote) = &mut self.remote {
            remote.flush()?
        }
        Ok(())
    }

    fn sync(&mut self, direction: SyncFrom) -> Result<()> {
        // TODO: Implement an actual distributed change logging system:
        //  https://github.com/vinhowe/quocofs/issues/5
        if self.remote.is_none() {
            return Err(NoRemotes);
        }

        let primary;
        let replica;

        match direction {
            SyncFrom::Remote => {
                primary = self.remote.as_mut().unwrap();
                replica = &mut self.local;
            }
            SyncFrom::Local => {
                primary = &mut self.local;
                replica = self.remote.as_mut().unwrap();
            }
        }

        // TODO: Break hash and name syncing into different functions
        let hash_ids: HashSet<ObjectId> = primary
            .hashes_ids()
            .chain(replica.hashes_ids())
            .copied()
            .collect();

        let name_ids: HashSet<ObjectId> = primary
            .names_ids()
            .chain(replica.names_ids())
            .copied()
            .collect();

        hash_ids.iter().try_for_each(|id| {
            sync_primary_replica(
                &primary.object_hash(&id)?.copied(),
                &replica.object_hash(&id)?.copied(),
                |_, add| {
                    if add {
                        replica.modify_object(
                            &id,
                            // Use concrete method from cached reader because it gives us reading and
                            // seeking
                            &mut primary
                                .object_cached_boxed(&id)
                                .map(|r| r as Box<dyn ReadSeek>)?,
                        )
                    } else {
                        // TODO: Object syncing looks like it works well, but add tests to be sure
                        //  that syncing doesn't incorrectly delete objects ever
                        // replica.delete_object(&id)
                        Ok(())
                    }
                },
            )
        })?;

        name_ids.iter().try_for_each(|id| {
            sync_primary_replica(
                &primary.object_name(&id)?.cloned(),
                &replica.object_name(&id)?.cloned(),
                |name, add| {
                    if add {
                        replica.set_object_name(&id, name.unwrap())
                    } else {
                        // TODO: Object syncing looks like it works well, but add tests to be sure
                        //  that syncing doesn't incorrectly delete objects ever
                        // replica.remove_object_name(&id)
                        Ok(())
                    }
                },
            )
        })?;

        self.flush()?;

        Ok(())
    }
}

enum SyncFrom {
    Remote,
    Local,
}

impl Drop for Session {
    fn drop(&mut self) {
        // TODO: Fix error handling on this whole thing
        self.clear_temp_files().expect("Failed to clear temp files")
    }
}
