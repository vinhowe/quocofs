mod util;

use crate::util::{output_vs_input_test, tests_data_dir, TEST_KEY};
use quocofs::object::{
    DecryptReader, EncrypterWriter, Finish, QuocoReader, QuocoWriter, CHUNK_LENGTH,
};
use std::io;
use std::io::{Cursor, Seek, SeekFrom, Write};

#[test]
fn encrypt_bytes() {
    output_vs_input_test(
        tests_data_dir().join("encrypted-only"),
        "plaintext",
        |reader, writer| {
            let mut encrypted_data = Cursor::new(Vec::new());
            let mut encrypt_writer = EncrypterWriter::new(encrypted_data, TEST_KEY);
            io::copy(reader, &mut encrypt_writer)?;
            encrypted_data = encrypt_writer.finish()?;
            encrypted_data.seek(SeekFrom::Start(0)).unwrap();
            io::copy(&mut DecryptReader::new(encrypted_data, TEST_KEY), writer)
        },
    )
}

#[test]
fn compress_encrypt_bytes() {
    output_vs_input_test(
        tests_data_dir().join("encrypted-compressed"),
        "plaintext",
        |reader, writer| {
            let mut quoco_data = Cursor::new(Vec::new());
            let mut quoco_writer = QuocoWriter::new(quoco_data, TEST_KEY);
            io::copy(reader, &mut quoco_writer)?;
            quoco_data = quoco_writer.finish()?;
            quoco_data.seek(SeekFrom::Start(0)).unwrap();
            io::copy(&mut QuocoReader::new(quoco_data, TEST_KEY), writer)
        },
    )
}

#[test]
fn compress_decompress_empty() {
    let mut compressed_data = Cursor::new(Vec::new());
    let mut decompressed_data = Cursor::new(Vec::new());
    let mut compressor_writer = brotli::CompressorWriter::new(compressed_data, CHUNK_LENGTH, 8, 22);
    compressor_writer.flush().unwrap();
    compressed_data = compressor_writer.into_inner();
    compressed_data.seek(SeekFrom::Start(0)).unwrap();
    let mut decompressor = brotli::Decompressor::new(compressed_data, CHUNK_LENGTH);
    io::copy(&mut decompressor, &mut decompressed_data).unwrap();
}
