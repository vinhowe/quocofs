use crate::formats::ReferenceFormatSpecification;
use crate::object::{MAX_DATA_LENGTH, MAX_NAME_LENGTH};
use std::string::String;

#[derive(Debug)]
pub enum EncryptionErrorType {
    Header,
    Body,
    Other(&'static str),
}

/// Largely based on
/// https://nick.groenen.me/posts/rust-error-handling/ and
/// https://www.reddit.com/r/rust/comments/gj8inf/rust_structuring_and_handling_errors_in_2020/fqlmknt/
#[derive(Debug)]
pub enum QuocoError {
    /// Any error occurred while trying to encrypt data.
    EncryptionError(EncryptionErrorType),
    /// Any error occurred while trying to decrypt data.
    ///
    /// This most likely means that the provided encryption key is incorrect, but it is returned
    /// whenever the underlying cryptography implementation fails for any reason.
    DecryptionError(EncryptionErrorType),
    EmptyInput,
    InvalidMagicBytes(&'static ReferenceFormatSpecification),
    EncryptionInputTooLong(usize),
    NameTooLong(usize),
    KeyGenerationError,
    SessionPathLocked(String),
    SessionDisposed,
    UndeterminedError,
    /// No remote object sources were found
    NoRemotes,
    TempFileDeleteFailed(String),
    TempFileDeletesFailed(Vec<(String, QuocoError)>),
    GoogleStorageError(cloud_storage::Error),
    /// Any otherwise unhandled `std::io::Error`.
    IoError(std::io::Error),
}

impl std::error::Error for QuocoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            QuocoError::EncryptionError(_)
            | QuocoError::DecryptionError(_)
            | QuocoError::EmptyInput
            | QuocoError::InvalidMagicBytes(_)
            | QuocoError::EncryptionInputTooLong(_)
            | QuocoError::NameTooLong(_)
            | QuocoError::KeyGenerationError
            | QuocoError::SessionPathLocked(_)
            | QuocoError::SessionDisposed
            | QuocoError::NoRemotes
            | QuocoError::TempFileDeleteFailed(_)
            | QuocoError::TempFileDeletesFailed(_)
            | QuocoError::UndeterminedError => None,
            QuocoError::GoogleStorageError(ref err) => err.source(),
            QuocoError::IoError(ref err) => err.source(),
        }
    }
}

impl From<std::array::TryFromSliceError> for QuocoError {
    fn from(_: std::array::TryFromSliceError) -> QuocoError {
        // TODO: Decide if this is the way to go
        QuocoError::UndeterminedError
    }
}

impl From<std::io::Error> for QuocoError {
    fn from(err: std::io::Error) -> QuocoError {
        if err.get_ref().is_some() && (*err.get_ref().unwrap()).is::<QuocoError>() {
            return *err.into_inner().unwrap().downcast::<QuocoError>().unwrap();
        }

        QuocoError::IoError(err)
    }
}

impl From<cloud_storage::Error> for QuocoError {
    fn from(err: cloud_storage::Error) -> Self {
        QuocoError::GoogleStorageError(err)
    }
}

impl From<QuocoError> for std::io::Error {
    fn from(err: QuocoError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, err)
    }
}

impl std::fmt::Display for QuocoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuocoError::EncryptionError(error_type) => match error_type {
                EncryptionErrorType::Header => write!(f, "Error creating encryption header"),
                EncryptionErrorType::Body => write!(f, "Encryption failed"),
                EncryptionErrorType::Other(msg) => write!(f, "{}", msg),
            },
            QuocoError::DecryptionError(error_type) => match error_type {
                EncryptionErrorType::Header => write!(f, "Failed to read decryption header"),
                EncryptionErrorType::Body => write!(f, "Decryption failed"),
                EncryptionErrorType::Other(msg) => write!(f, "{}", msg),
            },
            QuocoError::EmptyInput => {
                write!(f, "Input must not be empty")
            }
            QuocoError::EncryptionInputTooLong(length) => {
                write!(
                    f,
                    "Encryption input stream too large ({} bytes > {} max)",
                    length, MAX_DATA_LENGTH
                )
            }
            QuocoError::NameTooLong(length) => {
                write!(
                    f,
                    "Name too long ({} bytes > {} max)",
                    length, MAX_NAME_LENGTH
                )
            }
            QuocoError::KeyGenerationError => {
                write!(f, "Key generation failed")
            }
            QuocoError::SessionPathLocked(path) => {
                write!(f, "Path {} is locked by another process or a previous session failed to exit cleanly", path)
            }
            QuocoError::SessionDisposed => {
                write!(f, "Attempted to use session after clearing lock")
            }
            QuocoError::InvalidMagicBytes(data_type) => {
                write!(f, "Invalid magic bytes for {} data", data_type)
            }
            QuocoError::NoRemotes => {
                write!(f, "No remotes configured")
            }
            QuocoError::UndeterminedError => {
                write!(f, "Undetermined error")
            }
            QuocoError::TempFileDeleteFailed(path) => {
                write!(f, "Failed to delete temp file at path {}", path)
            }
            QuocoError::TempFileDeletesFailed(errors) => {
                // TODO: See if mutli-line errors are a huge issue for formatting
                write!(f, "Failed to delete temp files at paths:")?;
                for (path, error) in errors {
                    write!(f, "\n\t{}: {}", path, error)?;
                }
                Ok(())
            }
            QuocoError::GoogleStorageError(ref err) => err.fmt(f),
            QuocoError::IoError(ref err) => err.fmt(f),
        }
    }
}
