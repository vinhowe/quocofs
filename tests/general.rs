use base64::encode;
use libsodium_sys::randombytes_buf;
use quocofs::object::SALT_LENGTH;
use std::mem::MaybeUninit;

#[test]
fn generate_salt() {
    let mut salt = MaybeUninit::<[u8; SALT_LENGTH]>::uninit();
    unsafe {
        randombytes_buf(salt.as_mut_ptr() as *mut _, SALT_LENGTH);
    }
    let salt = unsafe { salt.assume_init() };
    let salt_str: String = encode(salt);
    println!("{}", salt_str);
}