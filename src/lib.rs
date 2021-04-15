pub use uuid::Bytes as UuidBytes;

use crate::error::QuocoError;
pub use crate::session::SESSIONS;
use std::result;

pub mod error;
pub mod formats;
pub mod object;
pub mod session;
pub mod util;

type Result<T> = result::Result<T, QuocoError>;
