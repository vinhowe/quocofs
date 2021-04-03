use crate::document::{DocumentHash, DocumentId, HASH_LENGTH, UUID_LENGTH};
use crate::error::QuocoError;
use crate::formats::{ReferenceFormat, ReferenceFormatSpecification, HASHES};
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{BufRead, Read, Write};
use std::mem::size_of;
use std::ops::Index;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type HashesDataType = HashMap<DocumentId, DocumentHash>;

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

    // pub fn deserialize(data: Vec<u8>) -> Result<HashMap<DocumentId, String>, QuocoError> {}
    //
    // pub fn serialize(&mut self) -> Result<Vec<u8>, QuocoError> {
    //     let mut data = Vec::new();
    //     data.write(ReferenceFormat::info().magic_bytes)?;
    //     for name in self.data.iter() {
    //         data.write(name.0)?;
    //         // Strip name of non-ASCII characters
    //         data.write(
    //             &name
    //                 .1
    //                 .chars()
    //                 .filter(|c| c.is_ascii() && *c != '\0')
    //                 .collect::<String>()
    //                 .as_bytes(),
    //         )?;
    //         data.push(0u8);
    //     }
    //     Ok(data)
    // }

    pub fn insert(&mut self, id: &DocumentId, hash: &DocumentHash) -> Option<DocumentHash> {
        self.data.insert(*id, *hash)
    }

    pub fn get_hash(&self, id: &DocumentId) -> Option<&DocumentHash> {
        self.data.get(id)
    }

    pub fn get_id(&self, hash: &DocumentHash) -> Option<&DocumentId> {
        self.data.iter().find(|x| *x.1 == *hash).map(|x| x.0)
    }
}

impl ReferenceFormat for Hashes {
    fn specification() -> &'static ReferenceFormatSpecification {
        &HASHES
    }

    // pub fn serialize_hashes<'a>(
    //     hashes_map: HashMap<DocumentId, DocumentHash>,
    // ) -> Result<Vec<u8>, QuocoError> {
    //     let mut data = Vec::with_capacity((UUID_LENGTH + HASH_LENGTH) * hashes_map.len());
    //     data.write(HASHES.magic_bytes)?;
    //     for hash in hashes_map.iter() {
    //         data.write(hash.0)?;
    //         data.write(hash.1)?;
    //     }
    //     Ok(data)
    // }
    //
    // pub fn deserialize_hashes(
    //     data: Vec<u8>,
    // ) -> Result<HashMap<DocumentId, DocumentHash>, QuocoError> {
    //     let mut hashes = HashMap::<DocumentId, DocumentHash>::new();
    //
    //     let mut data_reader = Cursor::new(data);
    //     let mut magic_bytes = [0u8; 4];
    //     data_reader.read(&mut magic_bytes)?;
    //
    //     check_magic_bytes(&magic_bytes, &HASHES)?;
    //
    //     let mut chunk = [0u8; UUID_LENGTH + HASH_LENGTH];
    //     loop {
    //         let bytes_read = data_reader.read(&mut chunk)?;
    //
    //         if bytes_read == 0 {
    //             break;
    //         }
    //
    //         hashes.insert(
    //             chunk[..UUID_LENGTH].try_into()?,
    //             chunk[UUID_LENGTH..].try_into()?,
    //         );
    //     }
    //     Ok(hashes)
    // }

    fn load<R: BufRead + Read>(&mut self, reader: &mut R) -> Result<(), QuocoError> {
        Self::check_magic_bytes(reader)?;

        let mut timestamp = [0u8; size_of::<u64>()];
        reader.read_exact(&mut timestamp)?;
        let timestamp = u64::from_le_bytes(timestamp);
        self.last_updated = UNIX_EPOCH + Duration::from_millis(timestamp);

        let mut entry_buf = [0u8; UUID_LENGTH + HASH_LENGTH];
        loop {
            let entry_bytes_read = reader.read(&mut entry_buf)?;

            if entry_bytes_read == 0 {
                break;
            }

            self.data.insert(
                entry_buf[..UUID_LENGTH].try_into()?,
                entry_buf[UUID_LENGTH..].try_into()?,
            );
        }
        Ok(())
    }

    fn save<W: Write>(&self, writer: &mut W) -> Result<(), QuocoError> {
        writer.write_all(Self::specification().magic_bytes)?;
        let now: u64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();
        writer.write_all(&now.to_le_bytes())?;
        for hash in self.data.iter() {
            writer.write_all(hash.0)?;
            writer.write_all(hash.1)?;
        }
        Ok(())
    }
}

impl Index<DocumentId> for Hashes {
    type Output = DocumentHash;

    fn index(&self, index: DocumentId) -> &Self::Output {
        &self.get_hash(&index).unwrap()
    }
}

impl Default for Hashes {
    fn default() -> Self {
        Hashes::new()
    }
}

// impl<W: Write> Index<String> for Names<W> {
//     type Output = DocumentId;
//
//     fn index(&self, index: String) -> &Self::Output {
//         self.data[&index]
//     }
// }

// pub fn serialize_names<'a>(
//     names_map: HashMap<DocumentId, String>,
// ) -> Result<Vec<u8>, QuocoError> {
//     let mut data = Vec::new();
//     data.write(NAMES.magic_bytes)?;
//     for name in names_map.iter() {
//         data.write(name.0)?;
//         // Strip name of non-ASCII characters
//         data.write(
//             &name
//                 .1
//                 .chars()
//                 .filter(|c| c.is_ascii() && *c != '\0')
//                 .collect::<String>()
//                 .as_bytes(),
//         )?;
//         data.push(0u8);
//     }
//     Ok(data)
// }
//
// pub fn deserialize_names(data: Vec<u8>) -> Result<HashMap<DocumentId, String>, QuocoError> {
//     let mut names = HashMap::new();
//     let mut data_reader = Cursor::new(data);
//
//     let mut magic_bytes = [0u8; 4];
//     data_reader.read(&mut magic_bytes)?;
//
//     check_magic_bytes(&magic_bytes, &NAMES)?;
//
//     let mut uuid = [0u8; UUID_LENGTH];
//
//     loop {
//         let uuid_bytes_read = data_reader.read(&mut uuid)?;
//
//         if uuid_bytes_read == 0 {
//             break;
//         }
//
//         let mut string_buffer = Vec::new();
//         let name_bytes_read = data_reader.read_until(0u8, &mut string_buffer).unwrap();
//
//         let name = String::from_utf8(string_buffer[..name_bytes_read - 1].to_vec()).unwrap();
//
//         names.insert(uuid, name);
//     }
//     Ok(names)
// }
