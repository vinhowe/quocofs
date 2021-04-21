use crate::error::QuocoError;
use crate::formats::{Hashes, Names, ReferenceFormat};
use crate::object::finish::Finish;
use crate::object::{Key, ObjectHash, ObjectId, ObjectSource, QuocoReader, QuocoWriter};
use crate::util::{bytes_to_hex_str, sha256};
use crate::{ReadSeek, Result};
use std::collections::hash_map::Keys;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::str;
use std::time::SystemTime;
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

        let source = FsObjectSource {
            path: path.into(),
            names: FsObjectSource::load_reference_format(Names::new(), path, key)?,
            hashes: FsObjectSource::load_reference_format(Hashes::new(), path, key)?,
            key: *key,
            lock: true,
        };

        // Only acquire lock after decryption works
        Self::touch_lock(path)?;

        Ok(source)
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

    fn modify_object_unchecked<R: Read + Seek>(
        &mut self,
        id: &ObjectId,
        reader: &mut R,
    ) -> Result<()> {
        let object_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(self.path.join(Path::new(&bytes_to_hex_str(id))))?;
        let mut writer = QuocoWriter::new(object_file, &self.key);

        self.hashes.insert(id, &sha256(reader)?);
        reader.seek(SeekFrom::Start(0))?;
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
            return Err(QuocoError::SessionPathLocked(path.to_str().unwrap().into()));
        }
        Ok(())
    }
}

impl ObjectSource for FsObjectSource {
    fn object(&mut self, id: &ObjectId) -> Result<Box<dyn Read>> {
        let object_path = self.path.join(&bytes_to_hex_str(id));

        Ok(Box::new(QuocoReader::new(
            fs::File::open(object_path)?,
            &self.key,
        )))
    }

    fn object_exists(&self, id: &ObjectId) -> Result<bool> {
        self.check_lock()?;

        Ok(self.path.join(Path::new(&bytes_to_hex_str(id))).exists())
    }

    fn delete_object(&mut self, id: &ObjectId) -> Result<()> {
        self.check_lock()?;

        self.hashes.remove(id);
        self.names.remove(id);

        fs::remove_file(self.path.join(Path::new(&bytes_to_hex_str(id))))?;

        Ok(())
    }

    fn create_object(&mut self, reader: &mut Box<dyn ReadSeek>) -> Result<ObjectId> {
        self.check_lock()?;

        let new_id = {
            let uuid = Uuid::new_v4();
            *uuid.as_bytes()
        };
        self.modify_object_unchecked(&new_id, reader)?;

        Ok(new_id)
    }

    fn modify_object(&mut self, id: &ObjectId, reader: &mut Box<dyn ReadSeek>) -> Result<()> {
        self.check_lock()?;

        self.modify_object_unchecked(id, reader)
    }

    fn object_hash(&self, id: &ObjectId) -> Result<Option<&ObjectHash>> {
        self.check_lock()?;

        Ok(self.hashes.get_hash(id))
    }

    fn object_name(&self, id: &ObjectId) -> Result<Option<&String>> {
        self.check_lock()?;

        Ok(self.names.get_name(id))
    }

    fn object_id_with_name(&self, name: &str) -> Result<Option<&ObjectId>> {
        self.check_lock()?;

        Ok(self.names.get_id(name))
    }

    fn set_object_name(&mut self, id: &[u8; 16], name: &str) -> Result<()> {
        self.check_lock()?;

        self.names.insert(id, name);

        Ok(())
    }

    fn remove_object_name(&mut self, id: &[u8; 16]) -> Result<()> {
        self.check_lock()?;

        self.names.remove(id);

        Ok(())
    }

    fn last_updated(&self) -> &SystemTime {
        &self.hashes.get_last_updated()
    }

    fn hashes_ids(&mut self) -> Keys<'_, ObjectId, ObjectHash> {
        self.hashes.get_ids()
    }

    fn names_ids(&mut self) -> Keys<'_, ObjectId, String> {
        self.names.get_ids()
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
        // TODO: Based on BufWriter's Drop impl, I think it might be an anti-pattern to do anything
        //  that could raise errors in drop. Look into this and refactor accordingly.
        self.flush().expect("Failed to flush.");
        self.unlock()
            .expect("Failed to release lock. You may have to release it manually.");
    }
}
