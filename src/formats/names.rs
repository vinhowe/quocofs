use crate::document::{DocumentId, UUID_LENGTH};
use crate::error::QuocoError;
use crate::formats::{ReferenceFormat, ReferenceFormatSpecification, NAMES};
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::ops::Index;

type NamesDataType = HashMap<DocumentId, String>;

pub struct Names {
    data: NamesDataType,
}

impl Names {
    pub fn new() -> Self {
        Names {
            data: HashMap::new(),
        }
    }

    pub fn insert(&mut self, id: &DocumentId, name: &str) -> Option<String> {
        self.data.insert(*id, name.into())
    }

    pub fn get_name(&self, id: &DocumentId) -> Option<&String> {
        self.data.get(id)
    }

    pub fn get_id(&self, name: &str) -> Option<&DocumentId> {
        self.data.iter().find(|x| *x.1 == name).map(|x| x.0)
    }
}

impl ReferenceFormat for Names {
    fn specification() -> &'static ReferenceFormatSpecification {
        &NAMES
    }

    fn load<R: BufRead + Read>(&mut self, reader: &mut R) -> Result<(), QuocoError> {
        Self::check_magic_bytes(reader)?;
        let mut uuid = [0u8; UUID_LENGTH];

        loop {
            let uuid_bytes_read = reader.read(&mut uuid)?;

            if uuid_bytes_read == 0 {
                break;
            }

            let mut string_buffer = Vec::new();
            let name_bytes_read = reader.read_until(0u8, &mut string_buffer).unwrap();

            let name = String::from_utf8(string_buffer[..name_bytes_read - 1].to_vec()).unwrap();

            self.data.insert(uuid, name);
        }
        Ok(())
    }

    fn save<W: Write>(&self, writer: &mut W) -> Result<(), QuocoError> {
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

impl Index<DocumentId> for Names {
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
