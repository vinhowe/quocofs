use crate::object::{ObjectSource, ObjectHash, ObjectId};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::io::{Cursor, Read};
use std::ops::Index;
use std::str;

/// Max cache size in bytes (2 GiB)
const MAX_CACHE_SIZE: usize = 1024 * 1024 * 1024 * 2;

pub struct CachedObjectSource<D: ObjectSource> {
    inner: D,
    cache: HashMap<ObjectId, Vec<u8>>,
    insertion_order: VecDeque<ObjectId>,
    /// Total size of all cached objects in bytes
    size: usize,
}

impl<D: ObjectSource> CachedObjectSource<D> {
    pub fn new(accessor: D) -> Self {
        CachedObjectSource {
            inner: accessor,
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
            self.insertion_order.iter().position(|x| *x == *id).expect(
                "Found object ID in cache, but couldn't find it in insertion order list.",
            ),
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

    fn load_into_cache(&mut self, id: &ObjectId) -> bool {
        if let Some(mut reader) = self.inner.object(id) {
            self.insert_reader(id, &mut reader)
                .expect("Error when attempting to read into object cache.");
            return true;
        }

        false
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
}

impl<D: ObjectSource> Index<&ObjectId> for CachedObjectSource<D> {
    type Output = [u8];

    fn index(&self, index: &[u8; 16]) -> &Self::Output {
        &self.cache[index]
    }
}

impl<D: ObjectSource> ObjectSource for CachedObjectSource<D> {
    type OutReader = Cursor<Vec<u8>>;

    fn object(&mut self, id: &ObjectId) -> Option<Cursor<Vec<u8>>> {
        if !self.object_exists(id) {
            return None;
        }

        if !self.cache.contains_key(id) && !self.load_into_cache(id) {
            return None;
        }

        // TODO: See if this irresponsibly fills memory
        Some(Cursor::new(self.cache[id].clone()))
    }

    fn object_exists(&self, id: &ObjectId) -> bool {
        // Checks if key exists in cache first because inner accessor might have to check the
        //  filesystem. Maybe it would be a good idea to store a cached list of all object IDs,
        //  but I doubt it would provide any noticeable performance boost ever.
        self.cache.contains_key(id) || self.inner.object_exists(id)
    }

    fn delete_object(&mut self, id: &ObjectId) -> bool {
        self.remove(id);
        self.inner.delete_object(id)
    }

    fn create_object<R: Read>(&mut self, reader: &mut R) -> Option<ObjectId> {
        if let Some(id) = self.inner.create_object(reader) {
            if self.insert_reader(&id, reader).is_err() {
                // TODO: Panic seems appropriate here because this should never ever happen,
                //  but the way the UUID is generated is ultimately up to the underlying
                //  ObjectAccessor implementor. This is either defensive or paranoid.
                panic!("Created a new object with an existing name");
            }
            return Some(id);
        }

        None
    }

    fn modify_object<R: Read>(&mut self, id: &ObjectId, reader: &mut R) -> bool {
        if !self.object_exists(id) {
            return false;
        }

        self.insert_reader(id, reader)
            .expect("Error when reading into object cache.");
        true
    }

    fn object_hash(&self, id: &[u8; 16]) -> Option<&ObjectHash> {
        // Hashes and Names on FsObjectAccessor act as caches
        self.inner.object_hash(id)
    }

    fn object_id_with_name(&self, name: &str) -> Option<&ObjectId> {
        self.inner.object_id_with_name(name)
    }

    fn set_object_name(&mut self, id: &ObjectId, name: &str) -> bool {
        self.inner.set_object_name(id, name)
    }

    fn flush(&mut self) -> bool {
        self.inner.flush()
    }
}
