use std::io;
use std::io::Write;

// TODO: Copy or move explanation on why this trait is necessary
pub trait Finish<W: Write> {
    fn finish(self) -> io::Result<W>;
}
