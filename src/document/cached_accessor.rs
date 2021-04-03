use crate::document::{DocumentAccessor, DocumentHash, DocumentId};
use std::collections::{HashMap, VecDeque};
use std::io;
use std::io::{Cursor, Read};
use std::ops::Index;
use std::str;

/// Max cache size in bytes (2 GiB)
const MAX_CACHE_SIZE: usize = 1024 * 1024 * 1024 * 2;

pub struct CachedDocumentAccessor<D: DocumentAccessor> {
    inner: D,
    cache: HashMap<DocumentId, Vec<u8>>,
    insertion_order: VecDeque<DocumentId>,
    /// Total size of all cached documents in bytes
    size: usize,
}

impl<D: DocumentAccessor> CachedDocumentAccessor<D> {
    pub fn new(accessor: D) -> Self {
        CachedDocumentAccessor {
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

    fn remove(&mut self, id: &DocumentId) -> Option<Vec<u8>> {
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
                "Found document ID in cache, but couldn't find it in insertion order list.",
            ),
        );

        Some(entry)
    }

    fn insert(&mut self, id: &DocumentId, data: Vec<u8>) -> Option<Vec<u8>> {
        let existing_data = self.remove(id);

        self.size += data.len();
        self.cache.insert(*id, data);
        self.insertion_order.push_front(*id);
        self.cull();

        existing_data
    }

    fn insert_reader<InR: Read>(
        &mut self,
        id: &DocumentId,
        reader: &mut InR,
    ) -> io::Result<Option<Vec<u8>>> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(self.insert(id, data))
    }

    fn load_into_cache(&mut self, id: &DocumentId) -> bool {
        if let Some(mut reader) = self.inner.document(id) {
            self.insert_reader(id, &mut reader)
                .expect("Error when attempting to read into document cache.");
            return true;
        }

        false
    }

    /// Removes document entries until either the total cache size is under [`MAX_CACHE_SIZE`] or
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

impl<D: DocumentAccessor> Index<&DocumentId> for CachedDocumentAccessor<D> {
    type Output = [u8];

    fn index(&self, index: &[u8; 16]) -> &Self::Output {
        &self.cache[index]
    }
}

impl<D: DocumentAccessor> DocumentAccessor for CachedDocumentAccessor<D> {
    type OutReader = Cursor<Vec<u8>>;

    fn document(&mut self, id: &DocumentId) -> Option<Cursor<Vec<u8>>> {
        if !self.document_exists(id) {
            return None;
        }

        if !self.cache.contains_key(id) && !self.load_into_cache(id) {
            return None;
        }

        // TODO: See if this irresponsibly fills memory
        Some(Cursor::new(self.cache[id].clone()))
    }

    fn document_exists(&self, id: &DocumentId) -> bool {
        // Checks if key exists in cache first because inner accessor might have to check the
        //  filesystem. Maybe it would be a good idea to store a cached list of all document IDs,
        //  but I doubt it would provide any noticeable performance boost ever.
        self.cache.contains_key(id) || self.inner.document_exists(id)
    }

    fn delete_document(&mut self, id: &DocumentId) -> bool {
        self.remove(id);
        self.inner.delete_document(id)
    }

    fn create_document<R: Read>(&mut self, reader: &mut R) -> Option<DocumentId> {
        if let Some(id) = self.inner.create_document(reader) {
            if self.insert_reader(&id, reader).is_err() {
                // TODO: Panic seems appropriate here because this should never ever happen,
                //  but the way the UUID is generated is ultimately up to the underlying
                //  DocumentAccessor implementor. This is either defensive or paranoid.
                panic!("Created a new document with an existing name");
            }
            return Some(id);
        }

        None
    }

    fn modify_document<R: Read>(&mut self, id: &DocumentId, reader: &mut R) -> bool {
        if !self.document_exists(id) {
            return false;
        }

        self.insert_reader(id, reader)
            .expect("Error when reading into document cache.");
        true
    }

    fn document_hash(&self, id: &[u8; 16]) -> Option<&DocumentHash> {
        // Hashes and Names on FsDocumentAccessor act as caches
        self.inner.document_hash(id)
    }

    fn document_id_with_name(&self, name: &str) -> Option<&DocumentId> {
        self.inner.document_id_with_name(name)
    }

    fn set_document_name(&mut self, id: &DocumentId, name: &str) -> bool {
        self.inner.set_document_name(id, name)
    }

    fn flush(&mut self) -> bool {
        self.inner.flush()
    }
}
