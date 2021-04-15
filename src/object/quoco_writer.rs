use crate::object::finish::Finish;
use crate::object::{EncrypterWriter, Key, CHUNK_LENGTH};
use brotli::CompressorWriter;
use std::io;
use std::io::Write;

pub struct QuocoWriter<W: Write> {
    inner: CompressorWriter<EncrypterWriter<W>>,
}

impl<W: Write> QuocoWriter<W> {
    pub fn new(writer: W, key: &Key) -> Self {
        QuocoWriter {
            // TODO: 8 seems like a good balance based on
            //  https://blogs.akamai.com/2016/02/understanding-brotlis-potential.html
            //  but maybe this should be configurable? Or even context-aware?
            inner: CompressorWriter::new(EncrypterWriter::new(writer, &key), CHUNK_LENGTH, 8, 22),
        }
    }

    pub fn into_inner(self) -> W {
        self.inner.into_inner().into_inner()
    }
}

impl<W: Write> Finish<W> for QuocoWriter<W> {
    fn finish(mut self) -> io::Result<W> {
        self.inner.flush()?;
        let writer = self.inner.into_inner().finish()?;
        Ok(writer)
    }
}

impl<W: Write> Write for QuocoWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
