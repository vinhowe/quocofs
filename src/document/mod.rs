use libsodium_sys::{
    crypto_box_SEEDBYTES, crypto_hash_sha256_BYTES, crypto_pwhash_SALTBYTES,
    crypto_secretstream_xchacha20poly1305_ABYTES,
};

pub use crate::document::accessor::DocumentAccessor;
pub use crate::document::cached_accessor::CachedDocumentAccessor;
pub use crate::document::decrypt_reader::DecryptReader;
pub use crate::document::encrypt_writer::EncrypterWriter;
pub use crate::document::fs_accessor::FsDocumentAccessor;
pub use crate::document::quoco_reader::QuocoReader;
pub use crate::document::quoco_writer::QuocoWriter;

mod accessor;
mod cached_accessor;
mod decrypt_reader;
mod encrypt_writer;
pub mod finish;
mod fs_accessor;
mod quoco_reader;
mod quoco_writer;

pub const CHUNK_LENGTH: usize = 4096;
const ENCRYPTED_CHUNK_LENGTH: usize =
    CHUNK_LENGTH + crypto_secretstream_xchacha20poly1305_ABYTES as usize;
pub const KEY_LENGTH: usize = crypto_box_SEEDBYTES as usize;
// Currently data is compressed and encrypted in memory, so we set an arbitrary max file size of 4 GiB.
// TODO(vinhowe): Figure out how to use less memory securely
pub const MAX_DATA_LENGTH: usize = 1024 * 1024 * 1024 * 4;
pub const MAX_NAME_LENGTH: usize = 512;
pub const SALT_LENGTH: usize = crypto_pwhash_SALTBYTES as usize;
pub const HASH_LENGTH: usize = crypto_hash_sha256_BYTES as usize;
pub const UUID_LENGTH: usize = 16;
pub type DocumentId = [u8; UUID_LENGTH];
pub type DocumentHash = [u8; HASH_LENGTH];
pub type Key = [u8; HASH_LENGTH];
