use executable_path::executable_path;
use std::{
    io::Write,
    process::{Command, Stdio},
    str,
};

extern crate executable_path;

#[test]
fn pass_normal_text_through() {
    let mut child = Command::new(executable_path("strip-ansi-escapes"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all("hello".as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());

    assert_eq!(str::from_utf8(&output.stdout).unwrap(), "hello");
}

#[test]
fn strip_escape_sequences() {
    let mut child = Command::new(executable_path("strip-ansi-escapes"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all("foo\x1B7bar".as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());

    assert_eq!(str::from_utf8(&output.stdout).unwrap(), "foobar");
}
