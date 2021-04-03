use crate::document::{DecryptReader, Key, CHUNK_LENGTH};
use std::io;
use std::io::Read;

pub struct QuocoReader<R: Read> {
    inner: brotli::Decompressor<DecryptReader<R>>,
}

impl<'a, R: Read> QuocoReader<R> {
    pub fn new(reader: R, key: &Key) -> Self {
        // TODO: Once chunked format is supported, we'll have to initialize a decompressor and
        //  decrypter for every chunk, using a chunk buffer to read from. For now, we can just
        //  initialize one decompressor/decrypter pair for the entire input reader.
        QuocoReader {
            inner: brotli::Decompressor::new(DecryptReader::new(reader, key), CHUNK_LENGTH),
        }
    }

    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}

impl<R: Read> Read for QuocoReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // TODO: Make this fill the buffer up to 2GB max chunks once we get chunked format working.
        //  For now the way it works is just to read everything in the reader to the buffer.
        //  But once we get that working, this function should be idempotent and based on the total
        //  number of bytes read. This total bytes read counter doesn't exist yet because for now we
        //  can just rely on Cursor's internal counter.
        self.inner.read(buf)
    }
}
