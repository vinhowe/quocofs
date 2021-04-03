use crate::document::{MAX_DATA_LENGTH, MAX_NAME_LENGTH};
use crate::formats::ReferenceFormatSpecification;
use std::borrow::Borrow;

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
    /// Any otherwise unhandled `std::io::Error`.
    IOError(std::io::Error),
}

impl std::error::Error for QuocoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            QuocoError::EncryptionError(_) => None,
            QuocoError::DecryptionError(_) => None,
            QuocoError::EmptyInput => None,
            QuocoError::InvalidMagicBytes(_) => None,
            QuocoError::EncryptionInputTooLong(_) => None,
            QuocoError::NameTooLong(_) => None,
            QuocoError::KeyGenerationError => None,
            QuocoError::SessionPathLocked(_) => None,
            QuocoError::SessionDisposed => None,
            QuocoError::UndeterminedError => None,
            QuocoError::IOError(_) => None,
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
        // if err_inner.is_some() {
        //     let err_ref = err.into_inner().unwrap();
        //     if err_ref.is::<QuocoError>() {
        //         return err_ref;
        //     }
        // }
        QuocoError::IOError(err)
    }
}

impl From<QuocoError> for std::io::Error {
    fn from(err: QuocoError) -> std::io::Error {
        let kind = match err.borrow() {
            QuocoError::EncryptionError(err_type) | QuocoError::DecryptionError(err_type) => {
                match err_type {
                    EncryptionErrorType::Header | EncryptionErrorType::Body => {
                        std::io::ErrorKind::InvalidData
                    }
                    EncryptionErrorType::Other(_) => std::io::ErrorKind::Other,
                }
            }
            QuocoError::EmptyInput
            | QuocoError::InvalidMagicBytes(_)
            | QuocoError::NameTooLong(_)
            | QuocoError::EncryptionInputTooLong(_) => std::io::ErrorKind::InvalidData,
            QuocoError::SessionPathLocked(_) => std::io::ErrorKind::AlreadyExists,
            QuocoError::SessionDisposed => std::io::ErrorKind::BrokenPipe,
            QuocoError::KeyGenerationError | QuocoError::UndeterminedError => {
                std::io::ErrorKind::Other
            }
            QuocoError::IOError(err) => return err.kind().into(),
        };

        std::io::Error::new(kind, err)
    }
}

impl std::fmt::Display for QuocoError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuocoError::EncryptionError(error_type) => match error_type {
                EncryptionErrorType::Header => write!(f, "Error creating encryption header."),
                EncryptionErrorType::Body => write!(f, "Encryption failed."),
                EncryptionErrorType::Other(msg) => write!(f, "{}", msg),
            },
            QuocoError::DecryptionError(error_type) => match error_type {
                EncryptionErrorType::Header => write!(f, "Failed to read decryption header."),
                EncryptionErrorType::Body => write!(f, "Decryption failed."),
                EncryptionErrorType::Other(msg) => write!(f, "{}", msg),
            },
            QuocoError::EmptyInput => {
                write!(f, "Input must not be empty.")
            }
            QuocoError::EncryptionInputTooLong(length) => {
                write!(
                    f,
                    "Encryption input stream too large ({} bytes > {} max).",
                    length, MAX_DATA_LENGTH
                )
            }
            QuocoError::NameTooLong(length) => {
                write!(
                    f,
                    "Name too long ({} bytes > {} max).",
                    length, MAX_NAME_LENGTH
                )
            }
            QuocoError::KeyGenerationError => {
                write!(f, "Key generation failed.")
            }
            QuocoError::SessionPathLocked(path) => {
                write!(f, "Path {} is locked by another process", path)
            }
            QuocoError::SessionDisposed => {
                write!(f, "Attempted to use session after clearing lock.")
            }
            QuocoError::InvalidMagicBytes(data_type) => {
                write!(f, "Invalid magic bytes for {} data.", data_type)
            }
            QuocoError::UndeterminedError => {
                write!(f, "Undetermined error.")
            }
            QuocoError::IOError(ref err) => err.fmt(f),
        }
    }
}
