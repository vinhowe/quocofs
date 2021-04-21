use crate::object::{BoxedObjectSource, ObjectHash, ObjectId, ObjectSource};
use crate::{ReadSeek, Result};
use std::collections::hash_map::Keys;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::ops::Index;
use std::str;
use std::time::SystemTime;

/// Max cache size in bytes (2 GiB)
const MAX_CACHE_SIZE: usize = 1024 * 1024 * 1024 * 2;

pub struct CachedObjectSource {
    inner: BoxedObjectSource,
    cache: HashMap<ObjectId, Vec<u8>>,
    insertion_order: VecDeque<ObjectId>,
    /// Total size of all cached objects in bytes
    size: usize,
}

impl CachedObjectSource {
    pub fn new(source: BoxedObjectSource) -> Self {
        CachedObjectSource {
            inner: source,
            cache: HashMap::new(),
            insertion_order: VecDeque::new(),
            size: 0,
        }
    }

    pub fn invalidate(&mut self) {
        self.cache.clear();
        self.insertion_order.clear();
        self.size = 0;
    }

    fn remove(&mut self, id: &ObjectId) -> Option<Vec<u8>> {
        if !self.cache.contains_key(id) {
            return None;
        }

        let entry = match self.cache.remove(id) {
            Some(entry) => entry,
            None => return None,
        };
        self.size -= entry.len();
        self.insertion_order.remove(
            self.insertion_order
                .iter()
                .position(|x| *x == *id)
                .expect("Found object ID in cache, but couldn't find it in insertion order list."),
        );

        Some(entry)
    }

    fn insert(&mut self, id: &ObjectId, data: Vec<u8>) -> Option<Vec<u8>> {
        let existing_data = self.remove(id);

        self.size += data.len();
        self.cache.insert(*id, data);
        self.insertion_order.push_front(*id);
        self.cull();

        existing_data
    }

    fn insert_reader<InR: Read>(
        &mut self,
        id: &ObjectId,
        reader: &mut InR,
    ) -> io::Result<Option<Vec<u8>>> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(self.insert(id, data))
    }

    fn load_into_cache(&mut self, id: &ObjectId) -> Result<()> {
        let mut object_reader = self.inner.object(id)?;
        self.insert_reader(id, &mut object_reader)?;

        Ok(())
    }

    /// Removes object entries until either the total cache size is under [`MAX_CACHE_SIZE`] or
    /// there is only one entry left.
    fn cull(&mut self) {
        while self.size > MAX_CACHE_SIZE && self.cache.len() > 1 {
            self.size -= self
                .cache
                .remove(&self.insertion_order.pop_back().unwrap())
                .unwrap()
                .len();
        }
    }

    pub fn object_cached(&mut self, id: &ObjectId) -> Result<Cursor<Vec<u8>>> {
        if !self.cache.contains_key(id) {
            self.load_into_cache(id)?;
        }

        // TODO: See if this irresponsibly fills memory
        Ok(Cursor::new(self.cache[id].clone()))
    }

    pub fn object_cached_boxed(&mut self, id: &ObjectId) -> Result<Box<Cursor<Vec<u8>>>> {
        let mut object_box = Box::default();
        std::mem::swap(&mut self.object_cached(id)?, &mut object_box);
        Ok(object_box)
    }
}

impl Index<&ObjectId> for CachedObjectSource {
    type Output = [u8];

    fn index(&self, index: &[u8; 16]) -> &Self::Output {
        &self.cache[index]
    }
}

impl ObjectSource for CachedObjectSource {
    fn object(&mut self, id: &ObjectId) -> Result<Box<dyn Read>> {
        self.object_cached_boxed(id).map(|r| r as Box<dyn Read>)
    }

    fn object_exists(&self, id: &ObjectId) -> Result<bool> {
        // Checks if key exists in cache first because inner source might have to check the
        //  filesystem. Maybe it would be a good idea to store a cached list of all object IDs,
        //  but I doubt it would provide any noticeable performance boost ever.
        Ok(self.cache.contains_key(id) || self.inner.object_exists(id)?)
    }

    fn delete_object(&mut self, id: &ObjectId) -> Result<()> {
        self.remove(id);
        self.inner.delete_object(id)
    }

    fn create_object(&mut self, reader: &mut Box<dyn ReadSeek>) -> Result<ObjectId> {
        let id = self.inner.create_object(reader)?;
        reader.seek(SeekFrom::Start(0))?;
        self.insert_reader(&id, reader)?;

        Ok(id)
    }

    fn modify_object(&mut self, id: &ObjectId, reader: &mut Box<dyn ReadSeek>) -> Result<()> {
        self.inner.modify_object(id, reader)?;
        reader.seek(SeekFrom::Start(0))?;
        self.insert_reader(id, reader)?;

        Ok(())
    }

    fn object_hash(&self, id: &[u8; 16]) -> Result<Option<&ObjectHash>> {
        // Hashes and Names on inner source act as caches
        self.inner.object_hash(id)
    }

    fn object_name(&self, id: &ObjectId) -> Result<Option<&String>> {
        self.inner.object_name(id)
    }

    fn object_id_with_name(&self, name: &str) -> Result<Option<&ObjectId>> {
        self.inner.object_id_with_name(name)
    }

    fn set_object_name(&mut self, id: &ObjectId, name: &str) -> Result<()> {
        self.inner.set_object_name(id, name)
    }

    fn remove_object_name(&mut self, id: &[u8; 16]) -> Result<()> {
        self.inner.remove_object_name(id)
    }

    fn last_updated(&self) -> &SystemTime {
        &self.inner.last_updated()
    }

    fn hashes_ids(&mut self) -> Keys<'_, ObjectId, ObjectHash> {
        self.inner.hashes_ids()
    }

    fn names_ids(&mut self) -> Keys<'_, ObjectId, String> {
        self.inner.names_ids()
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }
}
