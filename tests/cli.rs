//! The ported test_cli corpus, test for test.

mod common;

use common::{run_cli, temp_dir, write};

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

const MALFORMED_STATE: &str = "timeout = 30\n";

const OTHER_INTERFACE_STATE: &str = "\
interface shard-cache
author engineer-b

timeout = 45
operation store = ack
";

const EXPECTED_STAGE: &str = "interface shard-store\n\
                              authors engineer-a engineer-b\n\
                              clean: store\n\
                              conflicting: timeout\n\
                              one-sided: none\n\
                              missing: none\n";

const EXPECTED_HANDBACK: &str = "task shard-store timeout\n\
                                 authors engineer-a engineer-b\n\
                                 kind term-value\n\
                                 \u{20} left (run-a.tape) timeout = 30\n\
                                 \u{20} right (run-b.tape) timeout = 45\n\
                                 size small score 2\n\
                                 factors definitions 2, result-type no, breaking no\n\
                                 staging clean 1, conflicting 1, one-sided 0, missing 0\n";

const PRESENCE_RECORD: &str = "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n";

const EXPECTED_PRESENCE_HANDBACK: &str = "task shard-store cache-ttl\n\
                                          authors engineer-a engineer-b\n\
                                          kind presence\n\
                                          \u{20} left (run-a.tape) cache-ttl = 300\n\
                                          \u{20} right (run-b.tape) interface shard-store\n\
                                          size small score 1\n\
                                          factors definitions 1, result-type no, breaking no\n\
                                          staging clean 1, conflicting 1, one-sided 0, missing 1\n";

const MISLABELLED_PRESENCE_RECORD: &str = "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n  right (run-a.tape) operation dual = ack\n";

const SWAPPED_SOURCE_RECORD: &str = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n\ndivergence shard-store store result-type\n  left (run-b.tape) operation store = ack\n  right (run-b.tape) operation store = receipt\n";

fn session(
    dir: &std::path::Path,
    left: &str,
    right: &str,
    record_text: &str,
) -> (String, String, String) {
    (
        write(dir, "left.state", left),
        write(dir, "right.state", right),
        write(dir, "record.txt", record_text),
    )
}

fn session_default(dir: &std::path::Path) -> (String, String, String) {
    session(dir, LEFT, RIGHT, RECORD)
}

#[test]
fn cli_stage_prints_the_cross_apply_report() {
    let dir = temp_dir("cli-stage");
    let (left, right, record) = session_default(&dir);
    let result = run_cli(&["stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout_text(), EXPECTED_STAGE);
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn cli_handback_prints_the_sized_task() {
    let dir = temp_dir("cli-handback");
    let (left, right, record) = session_default(&dir);
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout_text(), EXPECTED_HANDBACK);
    let second = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(second.stdout, result.stdout);
}

#[test]
fn cli_handback_over_an_empty_record_prints_nothing() {
    let dir = temp_dir("cli-empty");
    let (left, right, record) = session(&dir, LEFT, RIGHT, "");
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_malformed_input_exits_2_without_traceback() {
    let dir = temp_dir("cli-malformed");
    let (left, right, record) = session(&dir, MALFORMED_STATE, RIGHT, RECORD);
    let result = run_cli(&["stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(stderr.contains("missing interface line"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_missing_file_exits_2_without_traceback() {
    let dir = temp_dir("cli-missing");
    let (_, right, record) = session_default(&dir);
    let missing = dir.join("nope.state").to_string_lossy().into_owned();
    let result = run_cli(&["stage", &missing, &right, &record], &dir);
    assert_eq!(result.code, 2);
    assert!(!result.stderr_text().contains("Traceback"));
}

#[test]
fn cli_interface_mismatch_exits_2() {
    let dir = temp_dir("cli-mismatch");
    let (left, right, record) = session(&dir, LEFT, OTHER_INTERFACE_STATE, RECORD);
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.contains("shard-cache"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}

#[test]
fn cli_inconsistent_row_sources_exit_2_without_traceback() {
    let dir = temp_dir("cli-swapped");
    let (left, right, record) = session(&dir, LEFT, RIGHT, SWAPPED_SOURCE_RECORD);
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(
        stderr.contains("not the left source of the record"),
        "{stderr}"
    );
    assert_eq!(stderr.matches('\n').count(), 1, "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_handback_over_a_presence_only_record() {
    let dir = temp_dir("cli-presence");
    let (left, right, record) = session(&dir, LEFT, RIGHT, PRESENCE_RECORD);
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout_text(), EXPECTED_PRESENCE_HANDBACK);
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn cli_mislabelled_presence_record_exits_2_without_traceback() {
    let dir = temp_dir("cli-mislabelled");
    let (left, right, record) = session(&dir, LEFT, RIGHT, MISLABELLED_PRESENCE_RECORD);
    let result = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(stderr.contains("not the right source"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_output_ignores_unrelated_files_beside_the_inputs() {
    let dir = temp_dir("cli-unrelated");
    let (left, right, record) = session_default(&dir);
    let baseline = run_cli(&["handback", &left, &right, &record], &dir);
    write(
        &dir,
        "unrelated.txt",
        "divergence shard-store timeout term-value\n  left (x.tape) timeout = 99\n  right (y.tape) timeout = 100\n",
    );
    write(&dir, "extra.state", OTHER_INTERFACE_STATE);
    let after = run_cli(&["handback", &left, &right, &record], &dir);
    assert_eq!(after.code, 0);
    assert_eq!(after.stdout, baseline.stdout);
    assert_eq!(after.stdout_text(), EXPECTED_HANDBACK);
}
