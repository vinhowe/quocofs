use pyo3::exceptions;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyType};
use pyo3::{create_exception, PyContextProtocol};
use quocofs::error::QuocoError;
use quocofs::formats::{Hashes, ReferenceFormat};
use quocofs::object::{
    Finish, Key, ObjectId, ObjectSource, QuocoReader, QuocoWriter, RemoteSourceConfig,
    CHUNK_LENGTH, HASH_LENGTH, KEY_LENGTH, MAX_DATA_LENGTH, MAX_NAME_LENGTH, SALT_LENGTH,
    UUID_LENGTH,
};
use quocofs::session::{close_session, get_session, new_session};
use quocofs::*;
use std::io;
use std::io::{BufReader, Cursor, Read};

create_exception!(module, IoError, exceptions::PyException);
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
create_exception!(module, NoRemotes, exceptions::PyException);
create_exception!(module, TempFileDeleteFailed, exceptions::PyException);
create_exception!(module, TempFileDeletesFailed, exceptions::PyException);
create_exception!(module, GoogleStorageError, exceptions::PyException);

struct PyQuocoError(QuocoError);

impl From<PyQuocoError> for PyErr {
    fn from(err: PyQuocoError) -> PyErr {
        match err.0 {
            QuocoError::IoError(io_err) => exceptions::PyOSError::new_err(io_err.to_string()),
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
            QuocoError::NoRemotes => NoRemotes::new_err(err.0.to_string()),
            QuocoError::TempFileDeleteFailed(_) => TempFileDeleteFailed::new_err(err.0.to_string()),
            QuocoError::TempFileDeletesFailed(_) => {
                TempFileDeletesFailed::new_err(err.0.to_string())
            }
            QuocoError::GoogleStorageError(_) => GoogleStorageError::new_err(err.0.to_string()),
        }
    }
}

impl From<QuocoError> for PyQuocoError {
    fn from(err: QuocoError) -> PyQuocoError {
        PyQuocoError(err)
    }
}

trait PyRemoteAccessConfigProvider {
    fn create(self) -> RemoteSourceConfig;
}

#[pyclass]
#[derive(Clone)]
struct GoogleStorageAccessorConfig {
    bucket: String,
    config_path: String,
}

#[pymethods]
impl GoogleStorageAccessorConfig {
    #[new]
    fn new(bucket: String, config_path: String) -> Self {
        GoogleStorageAccessorConfig {
            bucket,
            config_path,
        }
    }
}

impl PyRemoteAccessConfigProvider for GoogleStorageAccessorConfig {
    fn create(self) -> RemoteSourceConfig {
        RemoteSourceConfig::GoogleStorage {
            bucket: self.bucket,
            config_path: self.config_path.into(),
        }
    }
}

#[pyclass(name = "Session")]
struct PySession {
    id: UuidBytes,
}

#[pymethods]
impl PySession {
    #[new]
    fn new(path: &str, key: Key, remote: Option<&PyAny>) -> PyResult<Self> {
        Ok(PySession {
            id: new_session(
                path,
                &key,
                remote
                    .map(|c| {
                        // TODO: Remove this once we add more remote providers
                        #[allow(clippy::match_single_binding)]
                        c.extract().map(|c| match c {
                            GoogleStorageAccessorConfig { .. } => c.create(),
                        } as RemoteSourceConfig)
                    })
                    .transpose()?,
            )
            .map_err(PyQuocoError)?,
        })
    }

    fn object<'p>(&self, py: Python<'p>, id: ObjectId) -> PyResult<&'p PyBytes> {
        let mut object_data = Vec::new();
        let mut object_reader = get_session(&self.id)
            .borrow_mut()
            .local
            .object(&id)
            .map_err(PyQuocoError)?;
        object_reader.read_to_end(&mut object_data)?;

        Ok(PyBytes::new(py, &object_data))
    }

    fn create_object<'p>(&self, py: Python<'p>, data: Vec<u8>) -> PyResult<&'p PyBytes> {
        let object_id = get_session(&self.id)
            .borrow_mut()
            .local
            .create_object(&mut (Box::new(Cursor::new(data)) as Box<dyn ReadSeek>))
            .map_err(PyQuocoError)?;

        Ok(PyBytes::new(py, &object_id))
    }

    fn modify_object(&self, id: ObjectId, data: Vec<u8>) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .local
            .modify_object(&id, &mut (Box::new(Cursor::new(data)) as Box<dyn ReadSeek>))
            .map_err(PyQuocoError)?;

        Ok(())
    }

    fn delete_object(&self, id: ObjectId) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .local
            .delete_object(&id)
            .map_err(PyQuocoError)?;

        Ok(())
    }

    fn object_id_with_name(&self, name: &str) -> PyResult<Option<ObjectId>> {
        Ok(get_session(&self.id)
            .borrow()
            .local
            .object_id_with_name(name)
            .map(|o| o.copied())
            .map_err(PyQuocoError)?)
    }

    fn set_object_name(&self, id: ObjectId, name: &str) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .local
            .set_object_name(&id, name)
            .map_err(PyQuocoError)?;

        Ok(())
    }

    fn flush(&self) -> PyResult<()> {
        Ok(get_session(&self.id)
            .borrow_mut()
            .local
            .flush()
            .map_err(PyQuocoError)?)
    }

    fn object_temp_file(&self, id: ObjectId) -> PyResult<String> {
        let path = get_session(&self.id)
            .borrow_mut()
            .object_temp_file(&id)
            .map(|path| path.to_str().unwrap().to_string())
            .map_err(PyQuocoError)?;

        Ok(path)
    }

    fn clear_temp_files(&self) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .clear_temp_files()
            .map_err(PyQuocoError)?;

        Ok(())
    }

    fn push_remote(&self) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .push_remote()
            .map_err(PyQuocoError)?;

        Ok(())
    }

    fn pull_remote(&self) -> PyResult<()> {
        get_session(&self.id)
            .borrow_mut()
            .pull_remote()
            .map_err(PyQuocoError)?;

        Ok(())
    }
}

#[pyproto]
impl PyContextProtocol for PySession {
    fn __enter__(&mut self) -> PyResult<()> {
        self.pull_remote()
    }

    fn __exit__(
        &mut self,
        _ty: Option<&PyType>,
        _value: Option<&PyAny>,
        _traceback: Option<&PyAny>,
    ) -> PyResult<bool> {
        self.clear_temp_files()?;
        self.push_remote()?;
        Ok(close_session(&self.id))
    }
}

#[pymodule]
fn init_hashes_module(_py: Python, _m: &PyModule) -> PyResult<()> {
    #[pyfn(_m, "loads")]
    fn loads(py: Python, data: Vec<u8>, key: Key) -> PyResult<&PyDict> {
        let mut hashes = Hashes::default();
        let py_hashes = PyDict::new(py);
        hashes
            .load(&mut BufReader::new(QuocoReader::new(
                Cursor::new(data),
                &key,
            )))
            .map_err(PyQuocoError)?;
        hashes.iter().try_for_each(|(id, hash)| {
            py_hashes.set_item(PyBytes::new(py, id), PyBytes::new(py, hash))
        })?;
        Ok(py_hashes)
    }
    Ok(())
}

#[pymodule]
fn quocofs(_py: Python, _m: &PyModule) -> PyResult<()> {
    // Constants
    _m.add("CHUNK_LENGTH", CHUNK_LENGTH).unwrap();
    _m.add("MAX_DATA_LENGTH", MAX_DATA_LENGTH).unwrap();
    _m.add("KEY_LENGTH", KEY_LENGTH).unwrap();
    _m.add("MAX_NAME_LENGTH", MAX_NAME_LENGTH).unwrap();
    _m.add("SALT_LENGTH", SALT_LENGTH).unwrap();
    _m.add("HASH_LENGTH", HASH_LENGTH).unwrap();
    _m.add("UUID_LENGTH", UUID_LENGTH).unwrap();

    // Exception types
    _m.add("IoError", _py.get_type::<IoError>())?;
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
    _m.add("UndeterminedError", _py.get_type::<UndeterminedError>())?;
    _m.add("SessionDisposed", _py.get_type::<SessionDisposed>())?;
    _m.add("SessionPathLocked", _py.get_type::<SessionPathLocked>())?;
    _m.add(
        "TempFileDeleteFailed",
        _py.get_type::<TempFileDeleteFailed>(),
    )?;
    _m.add(
        "TempFileDeletesFailed",
        _py.get_type::<TempFileDeletesFailed>(),
    )?;
    _m.add("NoRemotes", _py.get_type::<NoRemotes>())?;
    _m.add("GoogleStorageError", _py.get_type::<GoogleStorageError>())?;

    // Classes
    _m.add_class::<GoogleStorageAccessorConfig>()?;
    _m.add_class::<PySession>()?;

    // Submodules
    let hashes_module = PyModule::new(_py, "hashes")?;
    init_hashes_module(_py, hashes_module)?;
    _m.add_submodule(hashes_module)?;

    #[pyfn(_m, "dumps")]
    fn dumps(py: Python, data: Vec<u8>, key: Key) -> PyResult<&PyBytes> {
        let compressed_encrypted_data = Vec::new();
        // compress_encrypt_data(&key, &mut Cursor::new(data), &mut compressed_encrypted_data)
        //     .map_err(PyQuocoError)?;
        let mut writer = QuocoWriter::new(compressed_encrypted_data, &key);
        io::copy(&mut Cursor::new(data), &mut writer).map_err(|err| PyQuocoError(err.into()))?;
        Ok(PyBytes::new(py, &writer.finish()?))
    }

    #[pyfn(_m, "loads")]
    fn loads(py: Python, data: Vec<u8>, key: Key) -> PyResult<&PyBytes> {
        let mut plaintext = Vec::new();
        QuocoReader::new(Cursor::new(data), &key)
            .read_to_end(&mut plaintext)
            .map_err(|err| PyQuocoError(err.into()))?;
        Ok(PyBytes::new(py, &plaintext))
    }

    #[pyfn(_m, "key")]
    fn key(py: Python, password: String, salt: [u8; SALT_LENGTH]) -> PyResult<&PyBytes> {
        Ok(PyBytes::new(
            py,
            &util::generate_key(&password, &salt).map_err(PyQuocoError)?,
        ))
    }

    #[pyfn(_m, "sha256")]
    fn sha256(py: Python, data: Vec<u8>) -> PyResult<&PyBytes> {
        let mut data_reader = Cursor::new(data);

        Ok(PyBytes::new(
            py,
            &util::sha256(&mut data_reader).map_err(PyQuocoError)?,
        ))
    }

    Ok(())
}
