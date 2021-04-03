use quocofs::document::Key;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::{fs, io};

pub fn tests_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
}

#[allow(dead_code)]
pub const TEST_PASSWORD: &str = "beneke";
#[allow(dead_code)]
pub const TEST_SALT: &[u8] = b",,\xc9)\x1fcK'8\xd9a\xc1\xad6\x942";
pub const TEST_KEY: &Key = b"\xc7\xbb\xf1\xbc-S\xa2\xa9r@\xcb\xc3;\x88%\xe5\x9e\xd4o\xb4\xf4X?\x9c\xb9\xdd\xcb\x85\r)N\x9f";

// No idea why the compiler doesn't think this is being used, it's used in tests in reader
#[allow(dead_code)]
pub fn output_vs_reference_test<B, F>(
    base_dir: PathBuf,
    input_dirname: &str,
    reference_dirname: &str,
    mut copy_fn: F,
) where
    F: FnMut(&mut File, &mut Cursor<Vec<u8>>) -> io::Result<B>,
{
    let in_dir = base_dir.join(input_dirname);
    let reference_dir = base_dir.join(reference_dirname);

    let mut in_paths: Vec<_> = fs::read_dir(in_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    in_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for in_path in in_paths {
        // NOTE: Spot for breakpoints based on file name
        // if in_path
        //     .file_name()
        //     .unwrap()
        //     .to_str()
        //     .unwrap()
        //     .split(".")
        //     .collect::<Vec<_>>()[0]
        //     == "<filename>"
        // {
        //     println!("");
        // }

        println!("Testing {}", in_path.file_name().unwrap().to_str().unwrap());
        let reference_path = reference_dir.join(in_path.file_name().unwrap());
        let mut in_file = File::open(in_path).unwrap();
        let mut reference_file = File::open(reference_path).unwrap();

        let mut reference_data = Vec::new();
        let mut output_data = Cursor::new(Vec::new());

        reference_file.read_to_end(&mut reference_data).unwrap();

        copy_fn(&mut in_file, &mut output_data).unwrap();

        assert_eq!(output_data.into_inner(), reference_data)
    }
}

#[allow(dead_code)]
pub fn output_vs_input_test<B, F>(base_dir: PathBuf, input_dirname: &str, mut copy_fn: F)
where
    F: FnMut(&mut File, &mut Cursor<Vec<u8>>) -> io::Result<B>,
{
    let mut in_paths: Vec<_> = fs::read_dir(base_dir.join(input_dirname))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    in_paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    for in_path in in_paths {
        if in_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split('.')
            .collect::<Vec<_>>()[0]
            == "4032"
        {
            println!("4032");
        }
        println!("Testing {}", in_path.file_name().unwrap().to_str().unwrap());
        let mut in_file = File::open(in_path).unwrap();

        let mut reference_data = Vec::new();
        let mut output_data = Cursor::new(Vec::new());

        in_file.read_to_end(&mut reference_data).unwrap();
        in_file.seek(SeekFrom::Start(0)).unwrap();

        copy_fn(&mut in_file, &mut output_data).unwrap();

        assert_eq!(output_data.into_inner(), reference_data)
    }
}
