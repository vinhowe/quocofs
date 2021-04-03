use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};
use pyo3::{create_exception, PyContextProtocol};
use pyo3::{exceptions, PyClass};
use quocofs::document::{
    DocumentAccessor, DocumentId, Key, QuocoReader, QuocoWriter, CHUNK_LENGTH, HASH_LENGTH,
    KEY_LENGTH, MAX_DATA_LENGTH, MAX_NAME_LENGTH, SALT_LENGTH, UUID_LENGTH,
};
use quocofs::error::QuocoError;
use quocofs::finish::Finish;
use quocofs::session::{close_session, get_session, new_session};
use quocofs::util::{generate_key, sha256};
use quocofs::*;
use std::io;
use std::io::{Cursor, Read};

create_exception!(module, EncryptionError, exceptions::PyException);
create_exception!(module, DecryptionError, exceptions::PyException);
create_exception!(module, EmptyInput, exceptions::PyException);
create_exception!(module, KeyGenerationError, exceptions::PyException);
create_exception!(module, NameTooLong, exceptions::PyException);
create_exception!(module, InvalidMagicBytes, exceptions::PyException);
create_exception!(module, EncryptionInputTooLong, exceptions::PyException);
create_exception!(module, UndeterminedError, exceptions::PyException);
create_exception!(module, SessionDisposed, exceptions::PyException);
create_exception!(module, SessionPathLocked, exceptions::PyException);

struct PyQuocoError(QuocoError);

impl From<PyQuocoError> for PyErr {
    fn from(err: PyQuocoError) -> PyErr {
        match err.0 {
            QuocoError::IOError(io_err) => exceptions::PyOSError::new_err(io_err.to_string()),
            QuocoError::EncryptionError(_) => EncryptionError::new_err(err.0.to_string()),
            QuocoError::DecryptionError(_) => DecryptionError::new_err(err.0.to_string()),
            QuocoError::EmptyInput => EmptyInput::new_err(err.0.to_string()),
            QuocoError::KeyGenerationError => KeyGenerationError::new_err(err.0.to_string()),
            QuocoError::NameTooLong(_) => NameTooLong::new_err(err.0.to_string()),
            QuocoError::InvalidMagicBytes(_) => InvalidMagicBytes::new_err(err.0.to_string()),
            QuocoError::EncryptionInputTooLong(_) => {
                EncryptionInputTooLong::new_err(err.0.to_string())
            }
            QuocoError::UndeterminedError => UndeterminedError::new_err(err.0.to_string()),
            QuocoError::SessionDisposed => SessionDisposed::new_err(err.0.to_string()),
            QuocoError::SessionPathLocked(_) => SessionPathLocked::new_err(err.0.to_string()),
        }
    }
}

impl From<QuocoError> for PyQuocoError {
    fn from(err: QuocoError) -> PyQuocoError {
        PyQuocoError(err)
    }
}

#[pyclass(name = "Session")]
struct PySession {
    id: UuidBytes,
}

#[pymethods]
impl PySession {
    #[new]
    fn new(path: &str, key: Key) -> Self {
        PySession {
            id: new_session(path, &key).unwrap(),
        }
    }

    fn create_document<'p>(&self, py: Python<'p>, data: Vec<u8>) -> PyResult<&'p PyBytes> {
        // TODO: Holy... figure out how to pull this out into its own function
        let document_id = get_session(&self.id)
            .borrow_mut()
            .cache
            .create_document(&mut Cursor::new(data))
            // TODO: This error handling is awful and I hate it
            .expect("Couldn't create a document!");
        Ok(PyBytes::new(py, &document_id))
    }

    fn modify_document(&self, id: DocumentId, data: Vec<u8>) -> PyResult<bool> {
        Ok(get_session(&self.id)
            .borrow_mut()
            .cache
            .modify_document(&id, &mut Cursor::new(data)))
    }

    fn delete_document(&self, id: DocumentId) -> bool {
        get_session(&self.id)
            .borrow_mut()
            .cache
            .delete_document(&id)
    }

    fn document_id_with_name(&self, name: &str) -> Option<DocumentId> {
        get_session(&self.id)
            .borrow()
            .cache
            .document_id_with_name(name)
            .copied()
    }

    fn set_document_name(&self, id: DocumentId, name: &str) -> bool {
        get_session(&self.id)
            .borrow_mut()
            .cache
            .set_document_name(&id, name)
    }

    fn flush(&self) -> bool {
        get_session(&self.id).borrow_mut().cache.flush()
    }

    fn document_temp_file(&self, id: DocumentId) -> PyResult<String> {
        get_session(&self.id)
            .borrow_mut()
            .document_temp_file(&id)
            .map(|path| path.to_str().unwrap().to_string())
            .map_err(|err| PyQuocoError(err).into())
        // TODO: This error handling is awful and I hate it
    }

    fn clear_temp_files(&self) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .clear_temp_files()
            .map_err(|err| PyQuocoError(err).into())
    }
}

#[pyproto]
impl PyContextProtocol for PySession {
    fn __enter__(&mut self) -> () {}

    fn __exit__(
        &mut self,
        _ty: Option<&PyType>,
        _value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) -> bool {
        close_session(&self.id)
    }
}

#[pymodule]
fn quocofs(_py: Python, _m: &PyModule) -> PyResult<()> {
    // Constants
    _m.add("CHUNK_LENGTH", CHUNK_LENGTH).unwrap();
    _m.add("MAX_DATA_LENGTH", MAX_DATA_LENGTH).unwrap();
    _m.add("MAX_NAME_LENGTH", MAX_NAME_LENGTH).unwrap();
    _m.add("SALT_LENGTH", SALT_LENGTH).unwrap();
    _m.add("KEY_LENGTH", KEY_LENGTH).unwrap();
    _m.add("HASH_LENGTH", HASH_LENGTH).unwrap();
    _m.add("UUID_LENGTH", UUID_LENGTH).unwrap();

    // Exception types
    _m.add("EncryptionError", _py.get_type::<EncryptionError>())?;
    _m.add("DecryptionError", _py.get_type::<DecryptionError>())?;
    _m.add("EmptyInput", _py.get_type::<EmptyInput>())?;
    _m.add("KeyGenerationError", _py.get_type::<KeyGenerationError>())?;
    _m.add("NameTooLong", _py.get_type::<NameTooLong>())?;
    _m.add("InvalidMagicBytes", _py.get_type::<InvalidMagicBytes>())?;
    _m.add(
        "EncryptionInputTooLong",
        _py.get_type::<EncryptionInputTooLong>(),
    )?;
    _m.add("Undetermined", _py.get_type::<UndeterminedError>())?;
    _m.add("SessionDisposed", _py.get_type::<SessionDisposed>())?;
    _m.add("SessionPathLocked", _py.get_type::<SessionPathLocked>())?;

    // Classes
    _m.add_class::<PySession>()?;

    #[pyfn(_m, "dumps")]
    fn dumps_py(py: Python, data: Vec<u8>, key: [u8; KEY_LENGTH]) -> PyResult<&PyBytes> {
        let compressed_encrypted_data = Vec::new();
        // compress_encrypt_data(&key, &mut Cursor::new(data), &mut compressed_encrypted_data)
        //     .map_err(PyQuocoError)?;
        let mut writer = QuocoWriter::new(compressed_encrypted_data, &key);
        io::copy(&mut Cursor::new(data), &mut writer).map_err(|err| PyQuocoError(err.into()))?;
        Ok(PyBytes::new(py, &writer.finish()?))
    }

    #[pyfn(_m, "loads")]
    fn loads_py(py: Python, data: Vec<u8>, key: [u8; KEY_LENGTH]) -> PyResult<&PyBytes> {
        let mut plaintext = Vec::new();
        // decrypt_decompress_data(
        //     &key,
        //     &mut Cursor::new(data),
        //     &mut decrypted_decompressed_data,
        // )
        // .map_err(|err| PyErr::from(PyQuocoError(err)))?;

        // io::copy(
        //     &mut QuocoReader::new(Cursor::new(data), &key),
        //     &mut plaintext,
        // )?;
        QuocoReader::new(Cursor::new(data), &key)
            .read_to_end(&mut plaintext)
            .map_err(|err| PyQuocoError(err.into()))?;
        Ok(PyBytes::new(py, &plaintext))
    }

    #[pyfn(_m, "key")]
    fn key_py(py: Python, password: String, salt: [u8; SALT_LENGTH]) -> PyResult<&PyBytes> {
        Ok(PyBytes::new(
            py,
            &generate_key(password.as_str(), &salt).map_err(PyQuocoError)?,
        ))
    }

    #[pyfn(_m, "sha256")]
    fn sha256_py(py: Python, data: Vec<u8>) -> PyResult<&PyBytes> {
        let mut hash = [0u8; HASH_LENGTH];
        let mut data_reader = Cursor::new(data);

        sha256(&mut data_reader, hash.as_mut_ptr()).map_err(PyQuocoError)?;

        Ok(PyBytes::new(py, &hash))
    }

    //
    // #[pyfn(_m, "serialize_hashes")]
    // fn serialize_hashes_py(
    //     py: Python,
    //     hashes: HashMap<[u8; UUID_LENGTH], [u8; HASH_LENGTH]>,
    // ) -> PyResult<&PyBytes> {
    //     Ok(PyBytes::new(
    //         py,
    //         &serialize_hashes(hashes.try_into().unwrap()).map_err(PyQuocoError)?,
    //     ))
    // }
    //
    // #[pyfn(_m, "deserialize_hashes")]
    // fn deserialize_hashes_py(py: Python, data: Vec<u8>) -> PyResult<&PyDict> {
    //     let py_hashes = PyDict::new(py);
    //     deserialize_hashes(data)
    //         .map_err(PyQuocoError)?
    //         .iter()
    //         .for_each(|item| {
    //             py_hashes
    //                 .set_item(PyBytes::new(py, item.0), PyBytes::new(py, item.1))
    //                 .unwrap()
    //         });
    //     Ok(py_hashes)
    // }
    //
    // #[pyfn(_m, "serialize_names")]
    // fn serialize_names_py(
    //     py: Python,
    //     names: HashMap<[u8; UUID_LENGTH], String>,
    // ) -> PyResult<&PyBytes> {
    //     for value in names.values() {
    //         if value.is_empty() {
    //             return Err(exceptions::PyValueError::new_err(format!(
    //                 "Name cannot be empty",
    //             )));
    //         }
    //         if value.len() > MAX_NAME_LENGTH {
    //             return Err(exceptions::PyValueError::new_err(format!(
    //                 "Name is too long ({} > {} max).",
    //                 value.len(),
    //                 MAX_NAME_LENGTH
    //             )));
    //         }
    //     }
    //
    //     Ok(PyBytes::new(
    //         py,
    //         &serialize_names(names).map_err(PyQuocoError)?,
    //     ))
    // }
    //
    // #[pyfn(_m, "deserialize_names")]
    // fn deserialize_names_py(py: Python, data: Vec<u8>) -> PyResult<&PyDict> {
    //     let py_names = PyDict::new(py);
    //     deserialize_names(data)
    //         .map_err(PyQuocoError)?
    //         .iter()
    //         .for_each(|item| py_names.set_item(PyBytes::new(py, item.0), item.1).unwrap());
    //     Ok(py_names)
    // }

    Ok(())
}
