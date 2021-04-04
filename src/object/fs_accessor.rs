use crate::object::finish::Finish;
use crate::object::{ObjectSource, ObjectHash, ObjectId, Key, QuocoReader, QuocoWriter};
use crate::error::QuocoError;
use crate::formats::{Hashes, Names, ReferenceFormat};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::str;
use std::{fs, io};
use uuid::Uuid;

pub struct FsObjectAccessor {
    pub names: Names,
    pub hashes: Hashes,
    path: PathBuf,
    key: Key,
    lock: bool,
}

pub const LOCK_FILE_NAME: &str = "quoco.lock";

impl FsObjectAccessor {
    pub fn open(path: &Path, key: &Key) -> Result<Self, QuocoError> {
        Self::check_no_lock(path)?;
        Self::touch_lock(path)?;

        Ok(FsObjectAccessor {
            path: path.into(),
            names: FsObjectAccessor::load_reference_format(Names::new(), path, key)?,
            hashes: FsObjectAccessor::load_reference_format(Hashes::new(), path, key)?,
            key: *key,
            lock: true,
        })
    }

    pub fn unlock(&mut self) -> Result<(), QuocoError> {
        fs::remove_file(self.path.join(LOCK_FILE_NAME))?;
        self.lock = false;
        Ok(())
    }

    // TODO: Work out naming/semantic division between this and check_no_lock
    fn check_lock(&self) -> Result<(), QuocoError> {
        if !self.lock {
            return Err(QuocoError::SessionDisposed);
        }
        Ok(())
    }

    fn modify_object_unchecked<R: Read>(&mut self, id: &ObjectId, reader: &mut R) -> bool {
        let object_file = match OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path.join(Path::new(str::from_utf8(id).unwrap())))
        {
            Ok(val) => val,
            Err(_) => return false,
        };
        let mut writer = QuocoWriter::new(object_file, &self.key);
        io::copy(reader, &mut writer)
            .expect("Error when attempting to modify object on filesystem.");
        writer
            .finish()
            .expect("Couldn't finish writing to object on filesystem.");
        true
    }

    fn load_reference_format<F: ReferenceFormat>(
        mut format: F,
        path: &Path,
        key: &Key,
    ) -> Result<F, QuocoError> {
        let path = path.join(F::specification().name);
        if path.exists() {
            let mut file_reader = BufReader::new(QuocoReader::new(File::open(&path)?, key));
            format.load(&mut file_reader)?;
        }
        Ok(format)
    }

    fn save_reference_format<F: ReferenceFormat>(
        &self,
        format: &F,
        path: &Path,
    ) -> Result<(), QuocoError> {
        let path = path.join(F::specification().name);
        let mut file_writer = QuocoWriter::new(File::open(&path)?, &self.key);

        format.save(&mut file_writer)?;
        Ok(())
    }

    pub fn touch_lock(path: &Path) -> Result<(), QuocoError> {
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path.join(LOCK_FILE_NAME))
            .map(|_| ())
            .map_err(|e| e.into())
    }

    pub fn check_no_lock(path: &Path) -> Result<(), QuocoError> {
        if path.join(LOCK_FILE_NAME).exists() {
            return Err(QuocoError::SessionPathLocked(String::from(
                path.to_str().unwrap(),
            )));
        }
        Ok(())
    }
}

impl ObjectSource for FsObjectAccessor {
    type OutReader = QuocoReader<File>;
    fn object(&mut self, id: &ObjectId) -> Option<Self::OutReader> {
        let object_path = self.path.join(str::from_utf8(id).unwrap());
        if !object_path.exists() {
            return None;
        }
        Some(QuocoReader::new(
            fs::File::open(object_path).unwrap(),
            &self.key,
        ))
    }

    fn object_exists(&self, id: &ObjectId) -> bool {
        self.check_lock().unwrap();

        self.path
            .join(Path::new(str::from_utf8(id).unwrap()))
            .exists()
    }

    fn delete_object(&mut self, id: &ObjectId) -> bool {
        if !self.object_exists(id) {
            return false;
        }

        // TODO: Is there any point in trying to shred encrypted objects?
        return fs::remove_file(self.path.join(Path::new(str::from_utf8(id).unwrap()))).is_ok();
    }

    fn create_object<InR: Read>(&mut self, reader: &mut InR) -> Option<ObjectId> {
        self.check_lock().unwrap();

        let new_id = {
            let uuid = Uuid::new_v4();
            *uuid.as_bytes()
        };
        if !self.modify_object_unchecked(&new_id, reader) {
            return None;
        }
        Some(new_id)
    }

    fn modify_object<InR: Read>(&mut self, id: &ObjectId, reader: &mut InR) -> bool {
        if !self.object_exists(id) {
            return false;
        }

        self.modify_object_unchecked(id, reader)
    }

    fn object_hash(&self, id: &ObjectId) -> Option<&ObjectHash> {
        self.check_lock().unwrap();

        self.hashes.get_hash(id)
    }

    fn object_id_with_name(&self, name: &str) -> Option<&ObjectId> {
        self.check_lock().unwrap();

        self.names.get_id(name)
    }

    fn set_object_name(&mut self, id: &[u8; 16], name: &str) -> bool {
        self.check_lock().unwrap();

        self.names.insert(id, name);
        true
    }

    fn flush(&mut self) -> bool {
        if self
            .save_reference_format(&self.hashes, self.path.as_path())
            .is_err()
        {
            return false;
        }
        if self
            .save_reference_format(&self.names, self.path.as_path())
            .is_err()
        {
            return false;
        }
        true
    }
}

impl Drop for FsObjectAccessor {
    fn drop(&mut self) {
        self.flush();
        self.unlock()
            .expect("Failed to release lock. You may have to release it manually.");
    }
}
