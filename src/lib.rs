pub use uuid::Bytes as UuidBytes;

pub use crate::session::SESSIONS;
use crate::error::QuocoError;
use std::result;

pub mod object;
pub mod error;
pub mod formats;
pub mod session;
pub mod util;

type Result<T> = result::Result<T, QuocoError>;