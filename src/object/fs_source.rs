use crate::error::QuocoError;
use crate::error::QuocoError::{NoObjectWithName, ObjectDoesNotExist};
use crate::formats::{Hashes, Names, ReferenceFormat};
use crate::object::finish::Finish;
use crate::object::{Key, ObjectHash, ObjectId, ObjectSource, QuocoReader, QuocoWriter};
use crate::util::bytes_to_hex_str;
use crate::Result;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::str;
use std::{fs, io};
use uuid::Uuid;

pub struct FsObjectSource {
    names: Names,
    hashes: Hashes,
    path: PathBuf,
    key: Key,
    lock: bool,
}

pub const LOCK_FILE_NAME: &str = "quoco.lock";

impl FsObjectSource {
    pub fn open(path: &Path, key: &Key) -> Result<Self> {
        Self::check_no_lock(path)?;

        let accessor = FsObjectSource {
            path: path.into(),
            names: FsObjectSource::load_reference_format(Names::new(), path, key)?,
            hashes: FsObjectSource::load_reference_format(Hashes::new(), path, key)?,
            key: *key,
            lock: true,
        };

        // Only acquire lock after decryption works
        Self::touch_lock(path)?;

        Ok(accessor)
    }

    pub fn unlock(&mut self) -> Result<()> {
        fs::remove_file(self.path.join(LOCK_FILE_NAME))?;
        self.lock = false;
        Ok(())
    }

    // TODO: Work out naming/semantic division between this and check_no_lock
    fn check_lock(&self) -> Result<()> {
        if !self.lock {
            return Err(QuocoError::SessionDisposed);
        }
        Ok(())
    }

    fn modify_object_unchecked<R: Read>(&mut self, id: &ObjectId, reader: &mut R) -> Result<()> {
        let object_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path.join(Path::new(&bytes_to_hex_str(id))))?;
        let mut writer = QuocoWriter::new(object_file, &self.key);
        io::copy(reader, &mut writer)
            .expect("Error when attempting to modify object on filesystem.");
        writer
            .finish()
            .expect("Couldn't finish writing to object on filesystem.");
        Ok(())
    }

    fn load_reference_format<F: ReferenceFormat>(
        mut format: F,
        path: &Path,
        key: &Key,
    ) -> Result<F> {
        let path = path.join(F::specification().name);
        if path.exists() {
            let mut file_reader = BufReader::new(QuocoReader::new(File::open(&path)?, key));
            format.load(&mut file_reader)?;
        }
        Ok(format)
    }

    fn save_reference_format<F: ReferenceFormat>(&self, format: &F) -> Result<()> {
        let path = self.path.join(F::specification().name);
        let mut file_writer = QuocoWriter::new(File::create(&path)?, &self.key);

        format.save(&mut file_writer)?;
        file_writer.finish()?;
        Ok(())
    }

    pub fn touch_lock(path: &Path) -> Result<()> {
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(path.join(LOCK_FILE_NAME))
            .map(|_| ())
            .map_err(|e| e.into())
    }

    pub fn check_no_lock(path: &Path) -> Result<()> {
        if path.join(LOCK_FILE_NAME).exists() {
            return Err(QuocoError::SessionPathLocked(String::from(
                path.to_str().unwrap(),
            )));
        }
        Ok(())
    }
}

impl ObjectSource for FsObjectSource {
    type OutReader = QuocoReader<File>;
    fn object(&mut self, id: &ObjectId) -> Result<Self::OutReader> {
        let object_path = self.path.join(&bytes_to_hex_str(id));
        Ok(QuocoReader::new(fs::File::open(object_path)?, &self.key))
    }

    fn object_exists(&self, id: &ObjectId) -> Result<bool> {
        self.check_lock()?;

        Ok(self.path.join(Path::new(&bytes_to_hex_str(id))).exists())
    }

    fn delete_object(&mut self, id: &ObjectId) -> Result<()> {
        fs::remove_file(self.path.join(Path::new(&bytes_to_hex_str(id))))?;

        Ok(())
    }

    fn create_object<InR: Read>(&mut self, reader: &mut InR) -> Result<ObjectId> {
        self.check_lock()?;

        let new_id = {
            let uuid = Uuid::new_v4();
            *uuid.as_bytes()
        };
        self.modify_object_unchecked(&new_id, reader)?;

        Ok(new_id)
    }

    fn modify_object<InR: Read>(&mut self, id: &ObjectId, reader: &mut InR) -> Result<()> {
        self.modify_object_unchecked(id, reader)
    }

    fn object_hash(&self, id: &ObjectId) -> Result<&ObjectHash> {
        self.check_lock().unwrap();

        match self.hashes.get_hash(id) {
            None => Err(ObjectDoesNotExist(*id)),
            Some(hash) => Ok(hash),
        }
    }

    fn object_id_with_name(&self, name: &str) -> Result<&ObjectId> {
        self.check_lock().unwrap();

        match self.names.get_id(name) {
            None => Err(NoObjectWithName(name.into())),
            Some(name) => Ok(name),
        }
    }

    fn set_object_name(&mut self, id: &[u8; 16], name: &str) -> Result<()> {
        self.check_lock()?;

        self.names.insert(id, name);

        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.check_lock()?;

        self.save_reference_format(&self.hashes)?;
        self.save_reference_format(&self.names)?;

        Ok(())
    }
}

impl Drop for FsObjectSource {
    fn drop(&mut self) {
        self.flush().expect("Failed to flush.");
        self.unlock()
            .expect("Failed to release lock. You may have to release it manually.");
    }
}
