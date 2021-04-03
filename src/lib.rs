pub use crate::session::SESSIONS;
pub use uuid::Bytes as UuidBytes;

// use crate::error::EncryptionErrorType::{Body, Header};
// use crate::error::QuocoError;
// use crate::error::QuocoError::{
//     DecryptionError, EncryptionError, EncryptionInputTooLong, UndeterminedError,
// };

pub mod document;
pub mod error;
pub mod finish;
pub mod formats;
pub mod session;
pub mod util;

// use std::io;
// use lazy_static::lazy_static;
// use std::convert::TryInto;
// use std::io::BufWriter;
// use std::io::{Cursor, Read, Seek, SeekFrom, Write};
// use std::mem::MaybeUninit;
// use std::ptr::null;

// lazy_static! {
//     static ref CACHE: HashMap<usize, Vec<u8>> = HashMap::new();
// }

// fn encrypt<R: Read + Seek, W: Write>(
//     key: &[u8],
//     in_buf: &mut R,
//     out_buf: &mut W,
// ) -> Result<(), QuocoError> {
//     // Based on unstable stream_len()
//     // TODO: Replace with stream_len when it becomes stable
//     let len;
//     {
//         let old_pos = in_buf.seek(SeekFrom::Current(0))?;
//         len = in_buf.seek(SeekFrom::End(0))?;
//
//         if old_pos != len {
//             in_buf.seek(SeekFrom::Start(old_pos))?;
//         }
//     }
//
//     if len > MAX_DATA_LENGTH as u64 {
//         return Err(EncryptionInputTooLong(len.try_into().unwrap()));
//     }
//
//     let mut state = MaybeUninit::<crypto_secretstream_xchacha20poly1305_state>::uninit();
//     let mut header = [0u8; crypto_secretstream_xchacha20poly1305_HEADERBYTES as usize];
//
//     if unsafe {
//         crypto_secretstream_xchacha20poly1305_init_push(
//             state.as_mut_ptr(),
//             header.as_mut_ptr() as *mut u8,
//             key.as_ptr(),
//         )
//     } != 0
//     {
//         return Err(EncryptionError(Header));
//     }
//
//     let mut state = unsafe { state.assume_init() };
//
//     out_buf.write(&header).unwrap();
//
//     let mut in_chunk = [0u8; CHUNK_LENGTH];
//     let mut out_chunk = [0u8; CHUNK_LENGTH + crypto_secretstream_xchacha20poly1305_ABYTES as usize];
//     let mut out_len: u64 = 0;
//     let mut bytes_read;
//
//     loop {
//         bytes_read = in_buf.read(&mut in_chunk)?;
//
//         let tag = if bytes_read < CHUNK_LENGTH {
//             crypto_secretstream_xchacha20poly1305_TAG_FINAL as u8
//         } else {
//             0
//         };
//
//         unsafe {
//             if crypto_secretstream_xchacha20poly1305_push(
//                 &mut state,
//                 out_chunk.as_mut_ptr(),
//                 &mut out_len as *mut u64,
//                 in_chunk.as_mut_ptr(),
//                 bytes_read as u64,
//                 null(),
//                 0,
//                 tag,
//             ) != 0
//             {
//                 return Err(EncryptionError(Body));
//             }
//         }
//
//         out_buf.write(&out_chunk[..out_len as usize])?;
//
//         if bytes_read < CHUNK_LENGTH {
//             break;
//         }
//     }
//
//     Ok(())
// }
//
// pub fn decrypt<R: Read + Seek, W: Write>(
//     key: &[u8],
//     in_buf: &mut R,
//     out_buf: &mut W,
// ) -> Result<(), QuocoError> {
//     let mut state = MaybeUninit::<crypto_secretstream_xchacha20poly1305_state>::uninit();
//
//     let mut header = unsafe {
//         MaybeUninit::<[u8; crypto_secretstream_xchacha20poly1305_HEADERBYTES as usize]>::uninit()
//             .assume_init()
//     };
//
//     in_buf.read_exact(&mut header)?;
//
//     unsafe {
//         if crypto_secretstream_xchacha20poly1305_init_pull(
//             state.as_mut_ptr(),
//             header.as_mut_ptr() as *mut u8,
//             key.as_ptr(),
//         ) != 0
//         {
//             return Err(DecryptionError(Header));
//         }
//     }
//
//     let mut state = unsafe { state.assume_init() };
//
//     let mut in_chunk = [0u8; CHUNK_LENGTH + crypto_secretstream_xchacha20poly1305_ABYTES as usize];
//     let mut out_chunk = [0u8; CHUNK_LENGTH];
//
//     let mut out_len: u64 = 0;
//     let mut tag: u8 = 0;
//     // let mut bytes_read;
//
//     loop {
//         let bytes_read = in_buf.read(&mut in_chunk).unwrap();
//
//         if bytes_read == 0 {
//             break;
//         }
//
//         unsafe {
//             if crypto_secretstream_xchacha20poly1305_pull(
//                 &mut state,
//                 out_chunk.as_mut_ptr(),
//                 &mut out_len as *mut u64,
//                 &mut tag as *mut u8,
//                 in_chunk[..bytes_read].as_ptr(),
//                 bytes_read as u64,
//                 null(),
//                 0,
//             ) != 0
//             {
//                 return Err(EncryptionError(Body));
//             }
//         }
//
//         out_buf.write(&out_chunk[..out_len as usize])?;
//
//         if bytes_read < CHUNK_LENGTH {
//             break;
//         }
//
//         // TODO: Come up with the obvious way to check for this (do we just need to count bytes?)
//         // if tag == crypto_secretstream_xchacha20poly1305_TAG_FINAL as u8 {
//         //     return Err(Box::from(io::Error::new(
//         //         io::ErrorKind::UnexpectedEof,
//         //         "Unexpected final tag",
//         //     )));
//         // }
//     }
//
//     Ok(())
// }
//
// fn compress<R: Read, W: Write>(reader: &mut R, writer: W) -> Result<(), QuocoError> {
//     let mut compress_writer = BufWriter::with_capacity(
//         CHUNK_LENGTH,
//         brotli::CompressorWriter::new(writer, CHUNK_LENGTH, 8, 22),
//     );
//     io::copy(reader, &mut compress_writer)?;
//     compress_writer.flush()?;
//     Ok(())
// }
//
// // fn decompress<R: Read, W: Write>(reader: &mut R, writer: &mut W) -> Result<(), QuocoError> {
// //     let mut decompress_reader = brotli::Decompressor::new(reader, 4096);
// //     io::copy(&mut decompress_reader, writer)?;
// //     Ok(())
// // }
//
// pub fn compress_encrypt_data<R: Read + Seek, W: Write>(
//     key: &[u8],
//     mut reader: &mut R,
//     mut writer: W,
// ) -> Result<(), QuocoError> {
//     let mut compressed = Vec::new();
//
//     compress(&mut reader, &mut compressed)?;
//     encrypt(&key, &mut Cursor::new(&compressed), &mut writer)?;
//
//     // encrypt(&key, &mut reader, &mut writer)?;
//
//     Ok(())
// }
//
// pub fn decrypt_decompress_data<R: Read + Seek, W: Write>(
//     key: &Key,
//     reader: &mut R,
//     mut writer: W,
// ) -> Result<(), QuocoError> {
//     // let mut decrypted = Vec::new();
//     //
//     // decrypt(&key, &mut reader, &mut decrypted)?;
//     // decompress(&mut Cursor::new(decrypted), &mut writer)?;
//
//     let mut reader = QuocoReader::new(reader, key);
//     io::copy(&mut reader, &mut writer)?;
//
//     Ok(())
// }
