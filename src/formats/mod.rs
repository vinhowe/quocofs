mod hashes;
mod names;

pub use crate::formats::hashes::Hashes;
pub use crate::formats::names::Names;

use crate::error::QuocoError;
use crate::Result;
use std::io::{BufRead, Read, Write};

#[derive(Debug)]
pub struct ReferenceFormatSpecification {
    pub magic_bytes: &'static [u8],
    pub name: &'static str,
}

impl std::fmt::Display for ReferenceFormatSpecification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub const NAMES: ReferenceFormatSpecification = ReferenceFormatSpecification {
    magic_bytes: b"pern",
    name: "names",
};

pub const HASHES: ReferenceFormatSpecification = ReferenceFormatSpecification {
    magic_bytes: b"perh",
    name: "hashes",
};

pub trait ReferenceFormat {
    // TODO: Is there a cleaner way to do this? I want to force every format to provide a name and
    //  magic bytes field (as used in the default implementation of check_magic_bytes) as part of
    //  the ReferenceFormat contract--and maybe I'm just stuck in OOP land--but I can't think of
    //  any solution that feels less icky than this.
    fn specification() -> &'static ReferenceFormatSpecification;

    fn load<R: BufRead + Read>(&mut self, reader: &mut R) -> Result<()>;
    fn save<W: Write>(&self, writer: &mut W) -> Result<()>;

    fn check_magic_bytes<R: Read>(reader: &mut R) -> Result<()> {
        let format_info = Self::specification();
        let mut magic_bytes = vec![0; format_info.magic_bytes.len()];
        reader.read_exact(&mut magic_bytes)?;

        if magic_bytes.ne(format_info.magic_bytes) {
            return Err(QuocoError::InvalidMagicBytes(format_info));
        }

        Ok(())
    }
}
