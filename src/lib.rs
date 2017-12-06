extern crate vte;

use std::io::{self, IntoInnerError, LineWriter, Write};
use vte::{Parser, Perform};

pub struct Writer<W>
    where W: Write,
{
    performer: Performer<W>,
    parser: Parser,
}

struct Performer<W>
    where W: Write,
{
    writer: LineWriter<W>,
    err: Option<io::Error>,
}

impl<W> Writer<W>
    where W: Write,
{
    pub fn new(inner: W) -> Writer<W> {
        Writer {
            performer: Performer {
                writer: LineWriter::new(inner),
                err: None,
            },
            parser: Parser::new(),
        }
    }

    pub fn into_inner(self) -> Result<W, IntoInnerError<LineWriter<W>>> {
        self.performer.into_inner()
    }
}

impl<W> Write for Writer<W>
    where W: Write,
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        for b in buf.iter() {
            self.parser.advance(&mut self.performer, *b)
        }
        match self.performer.err.take() {
            Some(e) => Err(e),
            None => Ok(buf.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> { self.performer.flush() }
}

impl<W> Performer<W>
    where W: Write,
{
    pub fn flush(&mut self) -> io::Result<()> { self.writer.flush() }

    pub fn into_inner(self) -> Result<W, IntoInnerError<LineWriter<W>>> {
        self.writer.into_inner()
    }
}

impl<W> Perform for Performer<W>
    where W: Write,
{
    fn print(&mut self, c: char) {
        self.err = write!(self.writer, "{}", c).err();
    }
    fn execute(&mut self, byte: u8) {
        if byte == b'\n' {
            self.err = writeln!(self.writer, "").err();
        }
    }
    fn hook(&mut self, _params: &[i64], _intermediates: &[u8], _ignore: bool) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]]) {}
    fn csi_dispatch(
        &mut self,
        _params: &[i64],
        _intermediates: &[u8],
        _ignore: bool,
        _: char
    ) {}
    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        _intermediates: &[u8],
        _ignore: bool,
        _byte: u8
    ) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn assert_parsed(input: &[u8], expected: &[u8]) {
        let c = Cursor::new(Vec::new());
        let mut writer = Writer::new(c);
        writer.write_all(input).unwrap();
        let bytes = writer.into_inner().unwrap().into_inner();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn test_simple() {
        assert_parsed(b"\x1b[m\x1b[m\x1b[32m\x1b[1m    Finished\x1b[m dev [unoptimized + debuginfo] target(s) in 0.0 secs",
                      b"    Finished dev [unoptimized + debuginfo] target(s) in 0.0 secs");
    }

    #[test]
    fn test_newlines() {
        assert_parsed(b"foo\nbar\n", b"foo\nbar\n");
    }

    #[test]
    fn test_escapes_newlines() {
        assert_parsed(b"\x1b[m\x1b[m\x1b[32m\x1b[1m   Compiling\x1b[m utf8parse v0.1.0
\x1b[m\x1b[m\x1b[32m\x1b[1m   Compiling\x1b[m vte v0.3.2
\x1b[m\x1b[m\x1b[32m\x1b[1m   Compiling\x1b[m strip-ansi-escapes v0.1.0-pre (file:///build/strip-ansi-escapes)
\x1b[m\x1b[m\x1b[32m\x1b[1m    Finished\x1b[m dev [unoptimized + debuginfo] target(s) in 0.66 secs
",
                      b"   Compiling utf8parse v0.1.0
   Compiling vte v0.3.2
   Compiling strip-ansi-escapes v0.1.0-pre (file:///build/strip-ansi-escapes)
    Finished dev [unoptimized + debuginfo] target(s) in 0.66 secs
");
    }
}
