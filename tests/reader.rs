mod util;

use crate::util::{output_vs_reference_test, tests_data_dir, TEST_KEY};
use quocofs::document::{DecryptReader, QuocoReader};
use std::io;

#[test]
fn decrypt_bytes() {
    output_vs_reference_test(
        tests_data_dir().join("encrypted-only"),
        // tests_data_dir().join("encrypted-compressed"),
        "encrypted",
        "plaintext",
        |reader, writer| io::copy(&mut DecryptReader::new(reader, TEST_KEY), writer),
    )
}

#[test]
fn decrypt_decompress_bytes() {
    output_vs_reference_test(
        tests_data_dir().join("encrypted-compressed"),
        "encrypted",
        "plaintext",
        |reader, writer| io::copy(&mut QuocoReader::new(reader, TEST_KEY), writer),
    )
}
