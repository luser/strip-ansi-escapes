//! A crate for stripping ANSI escape sequences from byte sequences.
//!
//! This can be used to take output from a program that includes escape sequences and write
//! it somewhere that does not easily support them, such as a log file.
//!
//! The simplest interface provided is the [`strip`] function, which takes a byte slice and returns
//! a `Vec` of bytes with escape sequences removed. For writing bytes directly to a writer, you
//! may prefer using the [`Writer`] struct, which implements `Write` and strips escape sequences
//! as they are written.
//!
//! [`strip`]: fn.strip.html
//! [`Writer`]: struct.Writer.html
//!
//! # Example
//!
//! ```
//! use std::io::{self, Write};
//!
//! # fn foo() -> io::Result<()> {
//! let bytes_with_colors = b"\x1b[32mfoo\x1b[m bar";
//! let plain_bytes = strip_ansi_escapes::strip(&bytes_with_colors)?;
//! io::stdout().write_all(&plain_bytes)?;
//! # Ok(())
//! # }
//! ```

extern crate vte;

use std::io::{self, Cursor, IntoInnerError, LineWriter, Write};
use vte::{Parser, Perform};

/// `Writer` wraps an underlying type that implements `Write`, stripping ANSI escape sequences
/// from bytes written to it before passing them to the underlying writer.
///
/// # Example
/// ```
/// use std::io::{self, Write};
/// use strip_ansi_escapes::Writer;
///
/// # fn foo() -> io::Result<()> {
/// let bytes_with_colors = b"\x1b[32mfoo\x1b[m bar";
/// let mut writer = Writer::new(io::stdout());
/// // Only `foo bar` will be written to stdout
/// writer.write_all(bytes_with_colors)?;
/// # Ok(())
/// # }
/// ```

pub struct Writer<W>
    where W: Write,
{
    performer: Performer<W>,
    parser: Parser,
}

/// Strip ANSI escapes from `data` and return the remaining bytes as a `Vec<u8>`.
///
/// See [the module documentation][mod] for an example.
///
/// [mod]: index.html
pub fn strip<T>(data: T) -> io::Result<Vec<u8>>
    where T: AsRef<[u8]>,
{
    let c = Cursor::new(Vec::new());
    let mut writer = Writer::new(c);
    writer.write_all(data.as_ref())?;
    Ok(writer.into_inner()?.into_inner())
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
    /// Create a new `Writer` that writes to `inner`.
    pub fn new(inner: W) -> Writer<W> {
        Writer {
            performer: Performer {
                writer: LineWriter::new(inner),
                err: None,
            },
            parser: Parser::new(),
        }
    }

    /// Unwraps this `Writer`, returning the underlying writer.
    ///
    /// The internal buffer is written out before returning the writer, which
    /// may produce an [`IntoInnerError`].
    ///
    /// [IntoInnerError]: https://doc.rust-lang.org/std/io/struct.IntoInnerError.html
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
        // Just print bytes to the inner writer.
        self.err = write!(self.writer, "{}", c).err();
    }
    fn execute(&mut self, byte: u8) {
        // We only care about executing linefeeds.
        if byte == b'\n' {
            self.err = writeln!(self.writer, "").err();
        }
    }
    // Since we're not actually implementing a terminal, we just ignore everything else.
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

    use std::env;
    use std::env::consts::EXE_EXTENSION;
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn readme_test() {
        let rustdoc = Path::new("rustdoc").with_extension(EXE_EXTENSION);
        let readme = Path::new(file!()).parent().unwrap().parent().unwrap().join("README.md");
        let exe = env::current_exe().unwrap();
        let outdir = exe.parent().unwrap();
        let mut cmd = Command::new(rustdoc);
        cmd.args(&["--verbose", "--test", "-L"])
            .arg(&outdir)
            .arg(&readme);
        println!("{:?}", cmd);
        let result = cmd.spawn()
            .expect("Failed to spawn process")
            .wait()
            .expect("Failed to run process");
        assert!(result.success(), "Failed to run rustdoc tests on README.md!");
    }

    fn assert_parsed(input: &[u8], expected: &[u8]) {
        let bytes = strip(input).expect("Failed to strip escapes");
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
