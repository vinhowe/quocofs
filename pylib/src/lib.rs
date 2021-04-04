use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyType};
use pyo3::{create_exception, PyContextProtocol};
use pyo3::{exceptions, PyClass};
use quocofs::object::{
    ObjectSource, ObjectId, Key, QuocoReader, QuocoWriter, CHUNK_LENGTH, HASH_LENGTH,
    KEY_LENGTH, MAX_DATA_LENGTH, MAX_NAME_LENGTH, SALT_LENGTH, UUID_LENGTH,
};
use quocofs::error::QuocoError;
use quocofs::object::finish::Finish;
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

    fn create_object<'p>(&self, py: Python<'p>, data: Vec<u8>) -> PyResult<&'p PyBytes> {
        // TODO: Holy... figure out how to pull this out into its own function
        let object_id = get_session(&self.id)
            .borrow_mut()
            .cache
            .create_object(&mut Cursor::new(data))
            // TODO: This error handling is awful and I hate it
            .expect("Couldn't create a object!");
        Ok(PyBytes::new(py, &object_id))
    }

    fn modify_object(&self, id: ObjectId, data: Vec<u8>) -> PyResult<bool> {
        Ok(get_session(&self.id)
            .borrow_mut()
            .cache
            .modify_object(&id, &mut Cursor::new(data)))
    }

    fn delete_object(&self, id: ObjectId) -> bool {
        get_session(&self.id)
            .borrow_mut()
            .cache
            .delete_object(&id)
    }

    fn object_id_with_name(&self, name: &str) -> Option<ObjectId> {
        get_session(&self.id)
            .borrow()
            .cache
            .object_id_with_name(name)
            .copied()
    }

    fn set_object_name(&self, id: ObjectId, name: &str) -> bool {
        get_session(&self.id)
            .borrow_mut()
            .cache
            .set_object_name(&id, name)
    }

    fn flush(&self) -> bool {
        get_session(&self.id).borrow_mut().cache.flush()
    }

    fn object_temp_file(&self, id: ObjectId) -> PyResult<String> {
        get_session(&self.id)
            .borrow_mut()
            .object_temp_file(&id)
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

    Ok(())
}
