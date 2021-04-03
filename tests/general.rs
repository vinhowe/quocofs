use base64::encode;
use libsodium_sys::randombytes_buf;
use quocofs::document::SALT_LENGTH;
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

//
// #[test]
// fn test_encryption() {
//     let mut key =
//         unsafe { MaybeUninit::<[u8; crypto_box_SEEDBYTES as usize]>::uninit().assume_init() };
//     key_from_passwd(
//         &mut key,
//         "abcdefghijklmnop",
//         &decode(TEST_SALT).unwrap().try_into().unwrap(),
//     )
//     .unwrap();
//     unsafe {
//         crypto_secretstream_xchacha20poly1305_keygen(key.as_mut_ptr());
//     }
//     // encrypt_file(&key, "test.txt", "test-enc.txt").unwrap();
//     // decrypt_file(&key, "test-enc.txt", "test-dec.txt").unwrap();
// }

// #[test]
// fn test_encryption() {
//     let mut in_file = File::open("test.txt").unwrap();
//
//     let mut encrypted_file_out = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-enc.txt")
//         .unwrap();
//
//     let mut key = generate_key(
//         "abcdefghijklmnop",
//         &decode(TEST_SALT).unwrap().try_into().unwrap(),
//     )
//         .unwrap();
//     unsafe {
//         crypto_secretstream_xchacha20poly1305_keygen(key.as_mut_ptr());
//     }
//
//     encrypt(&key, &mut in_file, &mut encrypted_file_out);
//
//     let mut decrypted_file = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-dec.txt")
//         .unwrap();
//     let mut encrypted_file_in = File::open("test-enc.txt").unwrap();
//
//     decrypt(&key, &mut encrypted_file_in, &mut decrypted_file);
// }
//
// #[test]
// fn test_chunked_compression() {
//     let mut in_file = File::open("test.txt").unwrap();
//     let mut compressed_file_out = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-zip.txt.br")
//         .unwrap();
//
//     compress(&mut in_file, compressed_file_out);
//
//     let mut compressed_file_in = File::open("test-zip.txt.br").unwrap();
//     let mut decompressed_file = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-unzipped.txt")
//         .unwrap();
//
//     decompress(&mut compressed_file_in, decompressed_file);
// }
//
// #[test]
// fn test_compress_encrypt_decrypt_decompress() {
//     let mut key = generate_key(
//         "abcdefghijklmnop",
//         &decode(TEST_SALT).unwrap().try_into().unwrap(),
//     )
//         .unwrap();
//     unsafe {
//         crypto_secretstream_xchacha20poly1305_keygen(key.as_mut_ptr());
//     }
//
//     let mut in_file = File::open("test.txt").unwrap();
//     let mut stored_file_out = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-zip.per")
//         .unwrap();
//
//     let mut compressed_data = Vec::new();
//
//     compress(&mut in_file, &mut compressed_data);
//     encrypt(
//         &key,
//         &mut Cursor::new(&compressed_data),
//         &mut stored_file_out,
//     );
//
//     let mut stored_file_in = File::open("test-zip.per").unwrap();
//     let mut expanded_file = OpenOptions::new()
//         .write(true)
//         //.create_new(true)
//         .create(true)
//         .truncate(true)
//         .open("test-expanded.txt")
//         .unwrap();
//
//     let mut decrypted_data = Vec::new();
//     decrypt(&key, &mut stored_file_in, &mut decrypted_data);
//     decompress(&mut Cursor::new(decrypted_data), expanded_file);
// }
