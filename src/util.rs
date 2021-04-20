use crate::error::QuocoError::{KeyGenerationError, TempFileDeleteFailed, UndeterminedError};
use crate::object::{CHUNK_LENGTH, HASH_LENGTH, SALT_LENGTH};
use crate::Result;
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
) -> Result<[u8; crypto_box_SEEDBYTES as usize]> {
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

pub fn sha256<R: Read>(reader: &mut R) -> Result<[u8; HASH_LENGTH]> {
    let mut state = MaybeUninit::<crypto_hash_sha256_state>::uninit();
    unsafe {
        if crypto_hash_sha256_init(state.as_mut_ptr()) != 0 {
            return Err(UndeterminedError);
        }
    }
    let mut state = unsafe { state.assume_init() };

    let mut in_chunk = [0u8; CHUNK_LENGTH as usize];
    let mut bytes_read;
    let mut hash = [0u8; HASH_LENGTH];
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
        if crypto_hash_sha256_final(&mut state, hash.as_mut_ptr()) != 0 {
            return Err(UndeterminedError);
        }
    }
    Ok(hash)
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

pub fn shred_file(path: &Path) -> Result<()> {
    // -u flag deletes the file after overwriting it
    if let Ok(status) = Command::new("shred")
        .arg("-u")
        .arg(path.as_os_str())
        .status()
    {
        if !status.success() {
            // TODO: Figure out how to make this
            return Err(TempFileDeleteFailed(path.to_str().unwrap().to_string()));
        }
    }

    Ok(())
}

pub fn delete_file(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn bytes_to_hex_str(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

pub fn hex_str_to_bytes(hex: &str) -> Vec<u8> {
    hex::decode(hex).expect("Couldn't decode byte string")
}

// TODO: Come up with a more descriptive name for this and its arguments
pub fn sync_primary_replica<T, Ra, A>(
    primary: &Option<T>,
    replica: &Option<T>,
    modify: A,
) -> Result<()>
where
    T: Eq,
    A: FnOnce(Option<&T>, bool) -> Result<Ra>,
{
    let mut values_eq = false;
    if replica.is_none()
        || (primary.is_some() && {
            values_eq = primary.as_ref().unwrap() == replica.as_ref().unwrap();
            !values_eq
        })
    {
        // We can assume that primary will have a value at this branch
        modify(primary.as_ref(), true)?;
        return Ok(());
    }

    // If both had values and neq each other, then the statement above would have been
    // called
    if values_eq {
        return Ok(());
    }

    modify(None, false)?;
    Ok(())
}
