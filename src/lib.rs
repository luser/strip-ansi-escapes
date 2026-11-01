//! A crate for stripping ANSI escape sequences from byte sequences or UTF-8 strings.
//!
//! This can be used to take output from a program that includes escape sequences and write
//! it somewhere that does not easily support them, such as a log file.
//!
//! The simplest interface provided is the [`strip`] function, which takes a byte slice and returns
//! a `Vec` of bytes with escape sequences removed. For writing bytes directly to a writer, you may
//! prefer using the [`io::Writer`] struct, which implements [`std::io::Write`] and strips escape
//! sequences as they are written.
//!
//! # Example
//!
//! ```
//! use std::io::{self, Write as _};
//!
//! # fn work() -> io::Result<()> {
//! let bytes_with_colors = b"\x1b[32mfoo\x1b[m bar";
//! let plain_bytes = strip_ansi_escapes::strip(&bytes_with_colors);
//! io::stdout().write_all(&plain_bytes)?;
//! # Ok(())
//! # }
//! ```

extern crate vte;

use vte::{Parser, Perform};

pub use self::io::Writer;

/// Stripping ANSI escape sequences while writing a byte stream to a type implementing
/// [`std::io::Write`].  The [`io::Writer`] type is also exposed from the top level of this crate.
pub mod io {
    use std::io::{self, IntoInnerError, LineWriter};

    use super::WriterImpl;

    /// `Writer` wraps an underlying type that implements [`std::io::Write`], stripping ANSI escape
    /// sequences from bytes written to it before passing them to the underlying writer.
    ///
    /// # Example
    /// ```
    /// use std::io::{self, Write as _};
    /// use strip_ansi_escapes::io::Writer;
    ///
    /// # fn work() -> io::Result<()> {
    /// let bytes_with_colors = b"\x1b[32mfoo\x1b[m bar";
    /// let mut writer = Writer::new(io::stdout());
    /// // Only `foo bar` will be written to stdout
    /// writer.write_all(bytes_with_colors)?;
    /// # Ok(())
    /// # }
    /// ```
    pub struct Writer<W>(WriterImpl<LineWriter<W>, io::Error>)
    where
        W: io::Write;

    impl<W> Writer<W>
    where
        W: io::Write,
    {
        /// Create a new `Writer` that writes to `inner`.
        pub fn new(inner: W) -> Self {
            Self(WriterImpl::new(LineWriter::new(inner)))
        }

        /// Unwraps this `Writer`, returning the underlying writer.
        ///
        /// The internal buffer is written out before returning the writer, which
        /// may produce an [`IntoInnerError`].
        pub fn into_inner(self) -> Result<W, IntoInnerError<LineWriter<W>>> {
            self.0.performer.writer.into_inner()
        }
    }

    impl<W> io::Write for Writer<W>
    where
        W: io::Write,
    {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.parser.advance(&mut self.0.performer, buf);
            match self.0.performer.err.take() {
                Some(e) => Err(e),
                None => Ok(buf.len()),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.performer.writer.flush()
        }
    }
}

/// Stripping ANSI escape sequences while writing a UTF-8 stream to a type implementing
/// [`std::fmt::Write`].
pub mod fmt {
    use std::fmt;

    use super::WriterImpl;

    /// `Writer` wraps an underlying type that implements [`std::fmt::Write`], stripping ANSI escape
    /// sequences from bytes written to it before passing them to the underlying writer.
    ///
    /// # Example
    /// ```
    /// use std::fmt::{self, Write as _};
    /// use strip_ansi_escapes::fmt::Writer;
    ///
    /// # fn work() -> fmt::Result {
    /// let str_with_colors = "\x1b[32mfoo\x1b[m bar";
    /// // Only `foo bar` will be written to stdout
    /// print!("{}", fmt::from_fn(|f| Writer::new(f).write_str(str_with_colors)));
    /// # Ok(())
    /// # }
    /// ```
    pub struct Writer<W>(WriterImpl<W, fmt::Error>)
    where
        W: fmt::Write;

    impl<W> Writer<W>
    where
        W: fmt::Write,
    {
        /// Create a new `Writer` that writes to `inner`.
        pub fn new(inner: W) -> Self {
            Self(WriterImpl::new(inner))
        }

        /// Unwraps this `Writer`, returning the underlying writer.
        pub fn into_inner(self) -> W {
            self.0.performer.writer
        }
    }

    impl<W> fmt::Write for Writer<W>
    where
        W: fmt::Write,
    {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            self.0.parser.advance(&mut self.0.performer, s.as_bytes());
            match self.0.performer.err.take() {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
    }
}

/// Strip ANSI escapes from `data` and return the remaining bytes as a [`Vec<u8>`].
///
/// See [the module documentation][self] for an example.
pub fn strip<T>(data: T) -> Vec<u8>
where
    T: AsRef<[u8]>,
{
    fn strip_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
        use std::io::Write as _;
        let mut writer = io::Writer::new(Vec::new());
        writer.write_all(data)?;
        Ok(writer.into_inner()?)
    }

    strip_impl(data.as_ref()).expect("writing to a Vec<u8> cannot fail")
}

/// Strip ANSI escapes from `data` and return the remaining contents as a [`String`].
///
/// # Example
///
/// ```
/// let str_with_colors = "\x1b[32mfoo\x1b[m bar";
/// let string_without_colors = strip_ansi_escapes::strip_str(str_with_colors);
/// assert_eq!(string_without_colors, "foo bar");
/// ```
pub fn strip_str<T>(data: T) -> String
where
    T: AsRef<str>,
{
    fn strip_impl(data: &str) -> Result<String, std::fmt::Error> {
        use std::fmt::Write as _;
        let mut writer = fmt::Writer::new(String::new());
        writer.write_str(data)?;
        Ok(writer.into_inner())
    }

    strip_impl(data.as_ref()).expect("writing to a String cannot fail")
}

/// The implementation behind both [`io::Writer`] and [`fmt::Writer`].
struct WriterImpl<W, E> {
    performer: Performer<W, E>,
    parser: Parser,
}

struct Performer<W, E> {
    writer: W,
    err: Option<E>,
}

impl<W, E> WriterImpl<W, E> {
    /// Create a new `WriterImpl` that writes to `inner`.
    fn new(inner: W) -> Self {
        WriterImpl {
            performer: Performer {
                writer: inner,
                err: None,
            },
            parser: Parser::new(),
        }
    }
}

impl<W> Perform for Performer<W, std::io::Error>
where
    W: std::io::Write,
{
    fn print(&mut self, c: char) {
        // Just print bytes to the inner writer.
        self.err = write!(self.writer, "{}", c).err();
    }
    fn execute(&mut self, byte: u8) {
        // We only care about executing linefeeds.
        if byte == b'\n' {
            self.err = writeln!(self.writer).err();
        }
    }
}

impl<W> Perform for Performer<W, std::fmt::Error>
where
    W: std::fmt::Write,
{
    fn print(&mut self, c: char) {
        // Just print bytes to the inner writer.
        self.err = write!(self.writer, "{}", c).err();
    }
    fn execute(&mut self, byte: u8) {
        // We only care about executing linefeeds.
        if byte == b'\n' {
            self.err = writeln!(self.writer).err();
        }
    }
}

#[cfg(doctest)]
extern crate doc_comment;

#[cfg(doctest)]
doc_comment::doctest!("../README.md", readme);

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parsed(input: &[u8], expected: &[u8]) {
        let bytes = strip(input);
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
