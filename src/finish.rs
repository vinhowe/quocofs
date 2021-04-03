use std::io;
use std::io::Write;

pub trait Finish<W: Write> {
    fn finish(self) -> io::Result<W>;
}
