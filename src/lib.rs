pub use uuid::Bytes as UuidBytes;

use crate::error::QuocoError;
pub use crate::session::SESSIONS;
use std::fs::File;
use std::io::{Cursor, Read, Seek};
use std::result;

pub mod error;
pub mod formats;
pub mod object;
pub mod session;
pub mod util;

pub trait ReadSeek: Read + Seek {}
impl ReadSeek for File {}
impl ReadSeek for Cursor<Vec<u8>> {}

type Result<T> = result::Result<T, QuocoError>;
