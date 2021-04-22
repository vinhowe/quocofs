use crate::formats::{ReferenceFormat, ReferenceFormatSpecification, HASHES};
use crate::object::{ObjectHash, ObjectId, HASH_LENGTH, UUID_LENGTH};
use crate::Result;
use std::collections::{hash_map, HashMap};
use std::convert::TryInto;
use std::io;
use std::io::{BufRead, Read, Write};
use std::mem::size_of;
use std::ops::Index;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HashesDataType = HashMap<ObjectId, ObjectHash>;

const ENTRY_LENGTH: usize = UUID_LENGTH + HASH_LENGTH;

pub struct Hashes {
    last_updated: SystemTime,
    data: HashesDataType,
}

impl Hashes {
    pub fn new() -> Self {
        Hashes {
            last_updated: SystemTime::now(),
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: &ObjectId, hash: &ObjectHash) -> Option<ObjectHash> {
        self.data.insert(*id, *hash)
    }

    pub fn remove(&mut self, id: &ObjectId) -> Option<ObjectHash> {
        self.data.remove(id)
    }

    pub fn get_hash(&self, id: &ObjectId) -> Option<&ObjectHash> {
        self.data.get(id)
    }

    pub fn get_id(&self, hash: &ObjectHash) -> Option<&ObjectId> {
        self.data.iter().find(|x| *x.1 == *hash).map(|x| x.0)
    }

    pub fn get_last_updated(&self) -> &SystemTime {
        &self.last_updated
    }

    pub fn get_ids(&self) -> hash_map::Keys<'_, ObjectId, ObjectHash> {
        self.data.keys()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, ObjectId, ObjectHash> {
        self.data.iter()
    }
}

impl ReferenceFormat for Hashes {
    fn specification() -> &'static ReferenceFormatSpecification {
        &HASHES
    }

    fn load<R: BufRead + Read>(&mut self, reader: &mut R) -> Result<()> {
        Self::check_magic_bytes(reader)?;

        let mut timestamp = [0u8; size_of::<u64>()];
        reader.read_exact(&mut timestamp)?;
        let timestamp = u64::from_le_bytes(timestamp);
        self.last_updated = UNIX_EPOCH + Duration::from_millis(timestamp);

        let mut entry_buf = Vec::with_capacity(UUID_LENGTH + HASH_LENGTH);
        loop {
            entry_buf.clear();

            let entry_bytes_read = reader
                .take(ENTRY_LENGTH as u64)
                .read_to_end(&mut entry_buf)?;

            if entry_bytes_read == 0 {
                break;
            }

            if entry_bytes_read < UUID_LENGTH + HASH_LENGTH {
                return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into());
            }

            self.data.insert(
                entry_buf[..UUID_LENGTH].try_into()?,
                entry_buf[UUID_LENGTH..].try_into()?,
            );
        }

        Ok(())
    }

    fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(Self::specification().magic_bytes)?;
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();
        writer.write_all(&now.to_le_bytes())?;
        for (id, hash) in self.data.iter() {
            assert_eq!(id.len(), UUID_LENGTH);
            assert_eq!(hash.len(), HASH_LENGTH);
            writer.write_all(id)?;
            writer.write_all(hash)?;
        }
        Ok(())
    }
}

impl Index<ObjectId> for Hashes {
    type Output = ObjectHash;

    fn index(&self, index: ObjectId) -> &Self::Output {
        &self.get_hash(&index).unwrap()
    }
}

impl Default for Hashes {
    fn default() -> Self {
        Hashes::new()
    }
}
