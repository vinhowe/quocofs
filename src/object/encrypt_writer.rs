use crate::error::{EncryptionErrorType, QuocoError};
use crate::object::finish::Finish;
use crate::object::{Key, CHUNK_LENGTH, ENCRYPTED_CHUNK_LENGTH};
use crate::Result;
use libsodium_sys::{
    crypto_secretstream_xchacha20poly1305_HEADERBYTES,
    crypto_secretstream_xchacha20poly1305_TAG_FINAL,
    crypto_secretstream_xchacha20poly1305_TAG_MESSAGE,
    crypto_secretstream_xchacha20poly1305_init_push, crypto_secretstream_xchacha20poly1305_push,
    crypto_secretstream_xchacha20poly1305_state,
};
use std::io::Write;
use std::mem::MaybeUninit;
use std::ptr::null;
use std::{cmp, io};

pub struct EncrypterWriter<W: Write> {
    // Using an option here is a pattern from BufWriter that allows us to implement both Drop trait
    // and into_inner method
    inner: Option<W>,
    buf: [u8; CHUNK_LENGTH],
    buf_len: usize,
    chunk_buf: [u8; ENCRYPTED_CHUNK_LENGTH],
    crypto_state: Option<crypto_secretstream_xchacha20poly1305_state>,
    key: Key,
    finished: bool,
}

/// Consumers *must call* [`finish`] to finish writing data. [`flush`] will only flush the inner
/// writer. Doing it this way allows instances of this object to use the `Write` trait without
/// stripping [`flush`] of its idempotence by awkwardly forcing consumers to only call it once or
/// finishing in Drop without error handling.
/// This comes with the major drawback that EncrypterWriter can't be used transparently as an I/O
/// writer. There is probably a much cleaner way to do this but I can't think what it is.
impl<W: Write> EncrypterWriter<W> {
    pub fn new(writer: W, key: &Key) -> Self {
        #[allow(clippy::uninit_assumed_init)]
        EncrypterWriter {
            inner: Some(writer),
            buf: unsafe { MaybeUninit::<[u8; CHUNK_LENGTH]>::uninit().assume_init() },
            buf_len: 0,
            chunk_buf: unsafe {
                MaybeUninit::<[u8; ENCRYPTED_CHUNK_LENGTH]>::uninit().assume_init()
            },
            crypto_state: None,
            key: *key,
            finished: false,
        }
    }

    fn init_crypto(&mut self) -> Result<()> {
        let mut state = MaybeUninit::<crypto_secretstream_xchacha20poly1305_state>::uninit();
        let mut header = [0u8; crypto_secretstream_xchacha20poly1305_HEADERBYTES as usize];

        if unsafe {
            crypto_secretstream_xchacha20poly1305_init_push(
                state.as_mut_ptr(),
                header.as_mut_ptr() as *mut u8,
                self.key.as_ptr(),
            )
        } != 0
        {
            return Err(QuocoError::EncryptionError(EncryptionErrorType::Header));
        }

        self.crypto_state = Some(unsafe { state.assume_init() });
        self.inner.as_mut().unwrap().write_all(&header)?;

        Ok(())
    }

    fn write_chunk(&mut self, tag: u8) -> Result<()> {
        if self.finished {
            return Err(QuocoError::EncryptionError(EncryptionErrorType::Other(
                "Attempted to write chunk after final tag.",
            )));
        }

        if self.crypto_state.is_none() {
            self.init_crypto()?;
        }

        let mut out_len: u64 = 0;

        unsafe {
            if crypto_secretstream_xchacha20poly1305_push(
                self.crypto_state.as_mut().unwrap(),
                self.chunk_buf.as_mut_ptr(),
                &mut out_len as *mut u64,
                self.buf[..self.buf_len].as_mut_ptr(),
                self.buf_len as u64,
                null(),
                0,
                tag,
            ) != 0
            {
                return Err(QuocoError::EncryptionError(EncryptionErrorType::Body));
            }
        }

        self.buf_len = 0;
        self.inner
            .as_mut()
            .unwrap()
            .write_all(&self.chunk_buf[..out_len as usize])?;

        Ok(())
    }

    pub fn into_inner(mut self) -> W {
        self.inner.take().unwrap()
    }
}

impl<W: Write> Finish<W> for EncrypterWriter<W> {
    fn finish(mut self) -> io::Result<W> {
        self.write_chunk(crypto_secretstream_xchacha20poly1305_TAG_FINAL as u8)?;
        self.flush()?;
        self.finished = true;
        Ok(self.into_inner())
    }
}

impl<W: Write> Write for EncrypterWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.finished {
            // TODO: Determine if this is the right kind of error for this
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        // TODO: For now each read call can only fill as much data as the internal buffer can
        //  contain. Would it be worth it to do the extra lifting to figure out how to loop to fill
        //  larger buffers? Probably not to me (vinhowe) right now.
        let rem = CHUNK_LENGTH - self.buf_len;
        let nwritten = cmp::min(rem, buf.len());
        self.buf[self.buf_len..self.buf_len + nwritten].copy_from_slice(&buf[..nwritten]);
        self.buf_len += nwritten;
        if nwritten >= rem {
            debug_assert!(nwritten == rem);
            self.write_chunk(crypto_secretstream_xchacha20poly1305_TAG_MESSAGE as u8)?;
        }

        Ok(nwritten)
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.finished {
            return Err(io::ErrorKind::BrokenPipe.into());
        }

        self.inner.as_mut().unwrap().flush()?;
        Ok(())
    }
}

impl<W: Write> Drop for EncrypterWriter<W> {
    fn drop(&mut self) {
        // Make sure consumer called finish()
        assert!(self.finished, "You must call finish()")
    }
}
