use crate::document::{Key, CHUNK_LENGTH, ENCRYPTED_CHUNK_LENGTH};
use crate::error::{EncryptionErrorType, QuocoError};
use libsodium_sys::{
    crypto_secretstream_xchacha20poly1305_HEADERBYTES,
    crypto_secretstream_xchacha20poly1305_TAG_FINAL,
    crypto_secretstream_xchacha20poly1305_init_pull, crypto_secretstream_xchacha20poly1305_pull,
    crypto_secretstream_xchacha20poly1305_state,
};
use std::io::{BufRead, Read};
use std::mem::MaybeUninit;
use std::ptr::null;
use std::{cmp, io};

pub struct DecryptReader<R: Read> {
    inner: R,
    in_buf: [u8; ENCRYPTED_CHUNK_LENGTH],
    out_buf: [u8; CHUNK_LENGTH],
    // pos, cap method based on BufReader implementation
    pos: usize,
    cap: usize,
    crypto_state: Option<crypto_secretstream_xchacha20poly1305_state>,
    key: Key,
    final_tag: bool,
}

impl<R: Read> DecryptReader<R> {
    pub fn new(reader: R, key: &Key) -> Self {
        #[allow(clippy::uninit_assumed_init)]
        DecryptReader {
            inner: reader,
            // TODO: Should I be using zeroed memory here instead? Clippy absolutely hates this.
            in_buf: unsafe { MaybeUninit::<[u8; ENCRYPTED_CHUNK_LENGTH]>::uninit().assume_init() },
            out_buf: unsafe { MaybeUninit::<[u8; CHUNK_LENGTH]>::uninit().assume_init() },
            pos: 0,
            cap: 0,
            crypto_state: None,
            key: *key,
            final_tag: false,
        }
    }

    fn init_crypto(&mut self) -> Result<(), QuocoError> {
        let mut state = MaybeUninit::<crypto_secretstream_xchacha20poly1305_state>::uninit();

        #[allow(clippy::uninit_assumed_init)]
        let mut header = unsafe {
            MaybeUninit::<[u8; crypto_secretstream_xchacha20poly1305_HEADERBYTES as usize]>::uninit(
            )
            .assume_init()
        };

        self.inner.read_exact(&mut header)?;

        unsafe {
            if crypto_secretstream_xchacha20poly1305_init_pull(
                state.as_mut_ptr(),
                header.as_mut_ptr() as *mut u8,
                self.key.as_ptr(),
            ) != 0
            {
                return Err(QuocoError::DecryptionError(EncryptionErrorType::Header));
            }
        }

        self.crypto_state = Some(unsafe { state.assume_init() });

        Ok(())
    }

    fn read_next_chunk(&mut self) -> Result<usize, QuocoError> {
        // TODO: Once the encoder supports this, handle multiple compressed/encrypted blocks in one
        //  file with a header index.

        // What this write function will need to do is decode the input in chunks, filling the input
        // buffer until it reaches the beginning of a new block, then decoding the filled buffer and
        // flushing it.

        if self.crypto_state.is_none() {
            self.init_crypto()?;
        }

        let mut out_len: u64 = 0;
        let mut tag: u8 = 0;

        let bytes_read = self.inner.read(&mut self.in_buf)?;

        if bytes_read == 0 {
            return Ok(bytes_read);
        }

        if self.final_tag {
            return Err(QuocoError::EncryptionError(EncryptionErrorType::Other(
                "Unexpected final tag during decryption.",
            )));
        }

        // TODO: See if we're making any bad assumptions here
        unsafe {
            if crypto_secretstream_xchacha20poly1305_pull(
                self.crypto_state.as_mut().unwrap(),
                self.out_buf.as_mut_ptr(),
                &mut out_len as *mut u64,
                &mut tag as *mut u8,
                self.in_buf[..bytes_read].as_ptr(),
                bytes_read as u64,
                null(),
                0,
            ) != 0
            {
                return Err(QuocoError::DecryptionError(EncryptionErrorType::Body));
            }
        }

        // TODO: Figure out how to check whether we've gotten a final tag too soon.
        //  It doesn't work to just check that this chunk is less than ENCRYPTED_CHUNK_LENGTH bytes
        //  because the last chunk could fit perfectly into that size. We need a way to determine if
        if tag == crypto_secretstream_xchacha20poly1305_TAG_FINAL as u8 {
            self.final_tag = true;
        }

        Ok(out_len as usize)
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

/// Based on BufReader's implementation
impl<R: Read> BufRead for DecryptReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.pos >= self.cap {
            debug_assert!(self.pos == self.cap);
            self.cap = self.read_next_chunk()?;
            self.pos = 0;
        }
        Ok(&self.out_buf[self.pos..self.cap])
    }

    fn consume(&mut self, amt: usize) {
        self.pos = cmp::min(self.pos + amt, self.cap);
    }
}

impl<R: Read> Read for DecryptReader<R> {
    /// Based on BufReader's implementation
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // TODO: For now each read call can only fill as much data as the internal buffer can
        //  contain. Would it be worth it to do the extra lifting to figure out how to loop to fill
        //  larger buffers? Probably not to me (vinhowe) right now.
        let nread = {
            let mut rem = self.fill_buf()?;
            rem.read(buf)?
        };
        self.consume(nread);
        Ok(nread)
    }
}
