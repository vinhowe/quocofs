use std::io;
use std::io::Write;

/// Rationale for this trait at the beginning of EncrypterWriter
pub trait Finish<W: Write> {
    fn finish(self) -> io::Result<W>;
}
