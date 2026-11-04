//! The print failure discipline (XC-29), probed against
//! the Python tool with both streams closed from the first
//! round. Every stream behaves as an 8192 byte buffer, so
//! a document under that size fails only when the runtime
//! flushes at shutdown and a document at or over it fails
//! inside the print itself.

mod common;

use common::{
    run_cli, run_cli_on_closed_stderr, run_cli_on_closed_stdout, run_cli_on_closed_streams,
    temp_dir, write,
};

const LEFT: &str = "\
interface shard-store
author engineer-a

timeout = 30
operation store = ack
";

const RIGHT: &str = "\
interface shard-store
author engineer-b

timeout = 45
operation store = ack
";

const RECORD: &str = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";

const FLUSH_LINE: &[u8] = b"error: crosscurrent could not print its report\n";
const BROKEN_PIPE_LINE: &[u8] = b"error: cannot read None, Broken pipe\n";

fn session(dir: &std::path::Path, record_text: &str) -> (String, String, String) {
    (
        write(dir, "left.state", LEFT),
        write(dir, "right.state", RIGHT),
        write(dir, "record.txt", record_text),
    )
}

/// A handback record whose printed document runs well past
/// the 8192 byte buffer boundary, sixty sized tasks of
/// ordinary shape.
fn big_record() -> String {
    let mut text = String::new();
    for index in 0..60 {
        text.push_str(&format!(
            "divergence shard-store term-{index:04} term-value\n  left (run-a.tape) term-{index:04} = 1\n  right (run-b.tape) term-{index:04} = 2\n"
        ));
    }
    text
}

#[test]
fn big_document_control_run_on_open_streams() {
    let dir = temp_dir("of-control");
    let (left, right, record) = session(&dir, &big_record());
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert!(
        result.stdout.len() >= 8192,
        "the control document must pass the buffer boundary, got {} bytes",
        result.stdout.len()
    );
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn small_document_to_a_dead_stdout_exits_120_with_the_flush_line() {
    let dir = temp_dir("of-small-out");
    let (left, right, record) = session(&dir, RECORD);
    let control = run_cli(&["stage", &left, &right, &record], &dir);
    assert_eq!(control.code, 0);
    assert!(control.stdout.len() < 8192);
    let result = run_cli_on_closed_stdout(&["stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stderr, FLUSH_LINE);
}

#[test]
fn big_document_to_a_dead_stdout_exits_2_with_the_broken_pipe_line() {
    let dir = temp_dir("of-big-out");
    let (left, right, record) = session(&dir, &big_record());
    let result = run_cli_on_closed_stdout(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stderr, BROKEN_PIPE_LINE);
}

#[test]
fn big_document_to_both_dead_streams_exits_120() {
    let dir = temp_dir("of-big-both");
    let (left, right, record) = session(&dir, &big_record());
    let result = run_cli_on_closed_streams(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn refusal_to_a_dead_stderr_exits_120() {
    let dir = temp_dir("of-refusal-err");
    let (_, right, record) = session(&dir, RECORD);
    let missing = dir.join("nope.state").to_string_lossy().into_owned();
    let result = run_cli_on_closed_stderr(&["stage", &missing, &right, &record], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn usage_refusal_to_a_dead_stderr_exits_120() {
    let dir = temp_dir("of-usage-err");
    let result = run_cli_on_closed_stderr(&[], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn help_to_a_dead_stdout_exits_120() {
    let dir = temp_dir("of-help-out");
    let result = run_cli_on_closed_stdout(&["--help"], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stderr, FLUSH_LINE);
}

#[test]
fn success_with_a_dead_stderr_still_exits_0_with_the_document() {
    let dir = temp_dir("of-success-err");
    let (left, right, record) = session(&dir, RECORD);
    let control = run_cli(&["stage", &left, &right, &record], &dir);
    let result = run_cli_on_closed_stderr(&["stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, control.stdout);
}

#[test]
fn empty_handback_to_both_dead_streams_exits_0() {
    let dir = temp_dir("of-empty-both");
    let (left, right, record) = session(&dir, "");
    let result = run_cli_on_closed_streams(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
}
