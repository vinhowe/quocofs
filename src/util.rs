use crate::document::{CHUNK_LENGTH, SALT_LENGTH};
use crate::error::QuocoError;
use crate::error::QuocoError::{KeyGenerationError, UndeterminedError};
use libsodium_sys::{
    crypto_box_SEEDBYTES, crypto_hash_sha256_final, crypto_hash_sha256_init,
    crypto_hash_sha256_state, crypto_hash_sha256_update, crypto_pwhash, crypto_pwhash_ALG_DEFAULT,
    crypto_pwhash_MEMLIMIT_INTERACTIVE, crypto_pwhash_OPSLIMIT_INTERACTIVE,
};
use std::fs;
use std::io::Read;
use std::mem::MaybeUninit;
use std::path::Path;
use std::process::Command;

pub fn generate_key<'a>(
    password: &'a str,
    salt: &'a [u8; SALT_LENGTH],
) -> Result<[u8; crypto_box_SEEDBYTES as usize], QuocoError> {
    let mut key = [0u8; crypto_box_SEEDBYTES as usize];
    unsafe {
        if crypto_pwhash(
            key.as_mut_ptr() as *mut _,
            crypto_box_SEEDBYTES as u64,
            password.as_ptr() as *const i8,
            password.len() as u64,
            salt.as_ptr() as *const _,
            crypto_pwhash_OPSLIMIT_INTERACTIVE as u64,
            crypto_pwhash_MEMLIMIT_INTERACTIVE as usize,
            crypto_pwhash_ALG_DEFAULT as i32,
        ) != 0
        {
            return Err(KeyGenerationError);
        }
    }

    Ok(key)
}

pub fn sha256<R: Read>(reader: &mut R, hash: *mut u8) -> Result<(), QuocoError> {
    let mut state = MaybeUninit::<crypto_hash_sha256_state>::uninit();
    unsafe {
        if crypto_hash_sha256_init(state.as_mut_ptr()) != 0 {
            return Err(UndeterminedError);
        }
    }
    let mut state = unsafe { state.assume_init() };

    let mut in_chunk = [0u8; CHUNK_LENGTH as usize];
    let mut bytes_read;
    loop {
        bytes_read = reader.read(&mut in_chunk)?;

        if bytes_read == 0 {
            break;
        }

        unsafe {
            if crypto_hash_sha256_update(
                &mut state,
                in_chunk[..bytes_read].as_ptr(),
                bytes_read as u64,
            ) != 0
            {
                return Err(UndeterminedError);
            };
        }
    }

    unsafe {
        if crypto_hash_sha256_final(&mut state, hash as *mut _) != 0 {
            return Err(UndeterminedError);
        }
    }
    Ok(())
}

pub fn is_shred_available() -> bool {
    if cfg!(windows) {
        // TODO: Determine how to find if shred is available on Windows or what alternatives exist
        return false;
    }

    Command::new("which")
        .arg("shred")
        .output()
        .unwrap()
        .status
        .success()
}

pub fn shred_file(path: &Path) -> bool {
    // -u flag deletes the file after overwriting it
    if let Ok(status) = Command::new("shred")
        .arg("-u")
        .arg(path.as_os_str())
        .status()
    {
        return status.success();
    }

    false
}

pub fn delete_file(path: &Path) -> bool {
    fs::remove_file(path).is_ok()
}

// // TODO: Consider creating a nicer abstraction for hashes/names de/serialization
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
//
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
