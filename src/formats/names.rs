use crate::formats::{ReferenceFormat, ReferenceFormatSpecification, NAMES};
use crate::object::{ObjectId, UUID_LENGTH};
use crate::Result;
use std::collections::{hash_map, HashMap};
use std::io::{BufRead, Read, Write};
use std::ops::Index;

type NamesDataType = HashMap<ObjectId, String>;

pub struct Names {
    data: NamesDataType,
}

impl Names {
    pub fn new() -> Self {
        Names {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: &ObjectId, name: &str) -> Option<String> {
        self.data.insert(*id, name.into())
    }

    pub fn remove(&mut self, id: &ObjectId) -> Option<String> {
        self.data.remove(id)
    }

    pub fn remove_name(&mut self, name: &str) -> Option<ObjectId> {
        if let Some(id) = self.get_id(name).copied() {
            return self.data.remove(&id).map(|_| id);
        }

        None
    }

    pub fn get_name(&self, id: &ObjectId) -> Option<&String> {
        self.data.get(id)
    }

    pub fn get_id(&self, name: &str) -> Option<&ObjectId> {
        self.data.iter().find(|x| *x.1 == name).map(|x| x.0)
    }

    pub fn get_ids(&self) -> hash_map::Keys<'_, ObjectId, String> {
        self.data.keys()
    }

    pub fn iter(&self) -> hash_map::Iter<'_, ObjectId, String> {
        self.data.iter()
    }
}

impl ReferenceFormat for Names {
    fn specification() -> &'static ReferenceFormatSpecification {
        &NAMES
    }

    fn load<R: BufRead + Read>(&mut self, reader: &mut R) -> Result<()> {
        Self::check_magic_bytes(reader)?;
        let mut uuid = [0u8; UUID_LENGTH];

        loop {
            let uuid_bytes_read = reader.read(&mut uuid)?;

            if uuid_bytes_read == 0 {
                break;
            }

            let mut string_buffer = Vec::new();
            let name_bytes_read = reader.read_until(0u8, &mut string_buffer)?;

            let name = String::from_utf8(string_buffer[..name_bytes_read - 1].to_vec()).unwrap();

            self.data.insert(uuid, name);
        }
        Ok(())
    }

    fn save<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(Self::specification().magic_bytes)?;
        for name in self.data.iter() {
            writer.write_all(name.0)?;
            // Strip name of non-ASCII characters
            writer.write_all(
                &name
                    .1
                    .chars()
                    .filter(|c| c.is_ascii() && *c != '\0')
                    .collect::<String>()
                    .as_bytes(),
            )?;
            writer.write_all(&[0u8])?;
        }
        Ok(())
    }
}

impl Index<ObjectId> for Names {
    type Output = String;

    fn index(&self, index: [u8; 16]) -> &Self::Output {
        &self.data[&index]
    }
}

impl Default for Names {
    fn default() -> Self {
        Names::new()
    }
}
