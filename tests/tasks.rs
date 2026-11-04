//! The ported test_tasks corpus, test for test, with the
//! discriminating pins for the XC-9 weights and band edges
//! and the XC-20 identity standing in the bundle corpus.

mod common;

use common::{record, state};
use crosscurrent::records::{Divergence, QuotedLine};
use crosscurrent::tasks::build_tasks;
use crosscurrent::{CrosscurrentError, Task, cross_apply, format_tasks};

const LEFT: &str = "\
interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
";

const RIGHT: &str = "\
interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
";

const TERM_RECORD: &str = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";

const RESULT_RECORD: &str = "divergence shard-store store result-type\n  left (run-a.tape) operation store = ack\n  right (run-b.tape) operation store = receipt\n";

const EXCLUDED_TERM_RECORD: &str = "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) excludes = legacy-mode\n  right (run-b.tape) legacy-mode = on\n";

const EXCLUDED_OPERATION_RECORD: &str = "divergence shard-store sweeper excluded-but-defined\n  left (run-a.tape) excludes = sweeper\n  right (run-b.tape) operation sweeper = report\n";

// The further row is labelled for the side its line comes
// from, as forkdrift's FD-18 renders it and XC-18 holds
// every row to. An earlier form of this fixture labelled
// that row right, which was forkdrift's rendering before
// FD-18 corrected it.
const EXCLUDED_DUAL_RECORD: &str = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n  left (run-a.tape) operation dual = ack\n";

const PRESENCE_TERM_RECORD: &str = "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n";

const PRESENCE_OPERATION_RECORD: &str = "divergence shard-store purge presence\n  left (run-a.tape) operation purge = ack\n  right (run-b.tape) interface shard-store\n";

const PRESENCE_DUAL_RECORD: &str = "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n  left (run-a.tape) operation dual = ack\n";

const PRESENCE_CROSS_RECORD: &str = "divergence shard-store fetch presence\n  left (run-a.tape) fetch = cached\n  right (run-b.tape) operation fetch = record\n";

fn tasks_for(record_text: &str, left_text: &str, right_text: &str) -> Vec<Task> {
    let left = state(left_text, "left.state");
    let right = state(right_text, "right.state");
    let divergences = record(record_text, "record.txt");
    let staging = cross_apply(&left, &right, &divergences).expect("staging runs");
    build_tasks(&left, &right, &divergences, &staging).expect("tasks build")
}

fn only_task(record_text: &str, left_text: &str, right_text: &str) -> Task {
    let found = tasks_for(record_text, left_text, right_text);
    assert_eq!(found.len(), 1);
    found.into_iter().next().expect("one task")
}

fn task_default(record_text: &str) -> Task {
    only_task(record_text, LEFT, RIGHT)
}

#[test]
fn every_divergence_produces_exactly_one_task() {
    let text = format!("{RESULT_RECORD}\n{TERM_RECORD}");
    let found = tasks_for(&text, LEFT, RIGHT);
    let names: Vec<&str> = found.iter().map(|task| task.name.as_str()).collect();
    assert_eq!(names, ["store", "timeout"]);
    let kinds: Vec<&str> = found.iter().map(|task| task.kind.as_str()).collect();
    assert_eq!(kinds, ["result-type", "term-value"]);
}

#[test]
fn tasks_are_sorted_by_name_then_kind_regardless_of_record_order() {
    let text = format!("{TERM_RECORD}\n{RESULT_RECORD}");
    let found = tasks_for(&text, LEFT, RIGHT);
    let names: Vec<&str> = found.iter().map(|task| task.name.as_str()).collect();
    assert_eq!(names, ["store", "timeout"]);
}

#[test]
fn a_task_states_authors_kind_and_quoted_forms() {
    let task = task_default(RESULT_RECORD);
    assert_eq!(task.left_author, "engineer-a");
    assert_eq!(task.right_author, "engineer-b");
    assert_eq!(task.kind, "result-type");
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(text.contains("task shard-store store"), "{text}");
    assert!(text.contains("authors engineer-a engineer-b"), "{text}");
    assert!(
        text.contains("  left (run-a.tape) operation store = ack"),
        "{text}"
    );
    assert!(
        text.contains("  right (run-b.tape) operation store = receipt"),
        "{text}"
    );
}

#[test]
fn a_task_states_the_clean_apply_summary() {
    let task = task_default(TERM_RECORD);
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(
        text.contains("staging clean 1, conflicting 2, one-sided 0, missing 0"),
        "{text}"
    );
}

#[test]
fn sizing_rule_scores_and_bands() {
    let task = task_default(TERM_RECORD);
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (2, false, false)
    );
    assert_eq!((task.score, task.band.as_str()), (2, "small"));

    let task = task_default(RESULT_RECORD);
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (2, true, false)
    );
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));

    let task = task_default(EXCLUDED_TERM_RECORD);
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (1, false, false)
    );
    assert_eq!((task.score, task.band.as_str()), (1, "small"));

    let task = task_default(EXCLUDED_OPERATION_RECORD);
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (1, true, false)
    );
    assert_eq!((task.score, task.band.as_str()), (3, "medium"));

    let task = task_default(EXCLUDED_DUAL_RECORD);
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (2, true, false)
    );
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));
}

#[test]
fn sizing_rule_breaking_factor_from_either_side() {
    let left_breaking = LEFT.replace(
        "operation store = ack",
        "operation store = ack\nbreaking timeout",
    );
    let task = only_task(TERM_RECORD, &left_breaking, RIGHT);
    assert_eq!((task.score, task.band.as_str()), (5, "large"));

    let right_breaking = RIGHT.replace(
        "operation store = receipt",
        "operation store = receipt\nbreaking store",
    );
    let task = only_task(RESULT_RECORD, LEFT, &right_breaking);
    assert_eq!((task.score, task.band.as_str()), (7, "large"));

    let left_with_legacy = LEFT.replace(
        "operation store = ack",
        "operation store = ack\nlegacy-mode = off\nbreaking legacy-mode",
    );
    let task = only_task(EXCLUDED_TERM_RECORD, &left_with_legacy, RIGHT);
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));
}

#[test]
fn the_factors_line_prints_what_the_band_came_from() {
    let task = task_default(RESULT_RECORD);
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(text.contains("size medium score 4"), "{text}");
    assert!(
        text.contains("factors definitions 2, result-type yes, breaking no"),
        "{text}"
    );
}

#[test]
fn an_empty_record_produces_no_tasks_and_empty_output() {
    let found = tasks_for("", LEFT, RIGHT);
    assert_eq!(found.len(), 0);
    assert_eq!(format_tasks(&found), "");
}

#[test]
fn the_handback_is_byte_stable() {
    let text = format!("{RESULT_RECORD}\n{TERM_RECORD}");
    assert_eq!(
        format_tasks(&tasks_for(&text, LEFT, RIGHT)),
        format_tasks(&tasks_for(&text, LEFT, RIGHT))
    );
}

#[test]
fn an_incomplete_divergence_is_refused_rather_than_printed_partial() {
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let staging = cross_apply(&left, &right, &[]).expect("staging runs");
    let no_rows = Divergence {
        interface: "shard-store".to_string(),
        name: "timeout".to_string(),
        kind: "term-value".to_string(),
        rows: Vec::new(),
        excludes_row: None,
        interface_row: None,
    };
    let error = build_tasks(&left, &right, &[no_rows], &staging).expect_err("task refuses");
    assert!(
        matches!(error, CrosscurrentError::Task(_)),
        "expected a task refusal, got {error:?}"
    );
    let rows = vec![
        QuotedLine {
            side: "left".to_string(),
            source: "run-a.tape".to_string(),
            line: "timeout = 30".to_string(),
        },
        QuotedLine {
            side: "right".to_string(),
            source: "run-b.tape".to_string(),
            line: "timeout = 45".to_string(),
        },
    ];
    let only_excludes = vec![QuotedLine {
        side: "left".to_string(),
        source: "run-a.tape".to_string(),
        line: "excludes = timeout".to_string(),
    }];
    let no_definitions = Divergence {
        interface: "shard-store".to_string(),
        name: "timeout".to_string(),
        kind: "excluded-but-defined".to_string(),
        excludes_row: Some(only_excludes[0].clone()),
        rows: only_excludes,
        interface_row: None,
    };
    let error = build_tasks(&left, &right, &[no_definitions], &staging).expect_err("task refuses");
    assert!(matches!(error, CrosscurrentError::Task(_)));
    let excluded_no_excludes_row = Divergence {
        interface: "shard-store".to_string(),
        name: "legacy-mode".to_string(),
        kind: "excluded-but-defined".to_string(),
        rows,
        excludes_row: None,
        interface_row: None,
    };
    let error = build_tasks(&left, &right, &[excluded_no_excludes_row], &staging)
        .expect_err("task refuses");
    assert!(matches!(error, CrosscurrentError::Task(_)));
}

#[test]
fn task_notes_a_name_the_staging_calls_clean() {
    let agree_a = "interface shard-store\nauthor engineer-a\ntimeout = 30\noperation store = ack\n";
    let agree_b = "interface shard-store\nauthor engineer-b\ntimeout = 30\noperation store = ack\n";
    let left = state(agree_a, "a.state");
    let right = state(agree_b, "b.state");
    let forged = "divergence shard-store timeout term-value\n  left (a.state) timeout = 30\n  right (b.state) timeout = 30\n";
    let divergences = record(forged, "forged.txt");
    let staging = cross_apply(&left, &right, &divergences).expect("staging runs");
    let tasks = build_tasks(&left, &right, &divergences, &staging).expect("tasks build");
    let text = format_tasks(&tasks);
    assert!(
        text.contains("note the staging cross-apply classifies this name clean"),
        "{text}"
    );
}

#[test]
fn a_presence_task_counts_only_its_defining_lines() {
    let task = task_default(PRESENCE_TERM_RECORD);
    assert_eq!(task.kind, "presence");
    assert_eq!(
        (task.definitions, task.result_type, task.breaking),
        (1, false, false)
    );
    assert_eq!((task.score, task.band.as_str()), (1, "small"));
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(text.contains("kind presence"), "{text}");
    assert!(
        text.contains("  left (run-a.tape) cache-ttl = 300"),
        "{text}"
    );
    assert!(
        text.contains("  right (run-b.tape) interface shard-store"),
        "{text}"
    );
    assert!(
        text.contains("factors definitions 1, result-type no, breaking no"),
        "{text}"
    );
}

#[test]
fn presence_sizing_follows_the_contract_enumeration() {
    let task = task_default(PRESENCE_OPERATION_RECORD);
    assert_eq!((task.definitions, task.result_type), (1, true));
    assert_eq!((task.score, task.band.as_str()), (3, "medium"));

    let task = task_default(PRESENCE_DUAL_RECORD);
    assert_eq!((task.definitions, task.result_type), (2, true));
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));

    let task = task_default(PRESENCE_CROSS_RECORD);
    assert_eq!((task.definitions, task.result_type), (2, true));
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));
}

#[test]
fn a_presence_task_prints_every_quoted_row() {
    let task = task_default(PRESENCE_DUAL_RECORD);
    let text = format_tasks(std::slice::from_ref(&task));
    let rows = [
        "  left (run-a.tape) dual = 1",
        "  right (run-b.tape) interface shard-store",
        "  left (run-a.tape) operation dual = ack",
    ];
    let positions: Vec<usize> = rows
        .iter()
        .map(|row| text.find(row).expect("row printed"))
        .collect();
    let mut sorted = positions.clone();
    sorted.sort_unstable();
    assert_eq!(positions, sorted);
}

#[test]
fn a_presence_task_breaking_factor_reads_the_states() {
    let left_breaking = LEFT.replace(
        "operation store = ack",
        "operation store = ack\ncache-ttl = 300\nbreaking cache-ttl",
    );
    let task = only_task(PRESENCE_TERM_RECORD, &left_breaking, RIGHT);
    assert!(task.breaking);
    assert_eq!((task.score, task.band.as_str()), (4, "medium"));
}

#[test]
fn a_presence_name_staging_calls_one_sided_carries_no_note() {
    let left_one = LEFT.replace(
        "operation store = ack",
        "operation store = ack\ncache-ttl = 300",
    );
    let task = only_task(PRESENCE_TERM_RECORD, &left_one, RIGHT);
    assert!(task.note.is_none());
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(
        text.contains("staging clean 1, conflicting 2, one-sided 1, missing 0"),
        "{text}"
    );
    assert!(!text.contains("note "), "{text}");
}

#[test]
fn a_presence_name_one_sided_on_the_other_side_still_carries_no_note() {
    let right_one = RIGHT.replace(
        "operation store = receipt",
        "operation store = receipt\ncache-ttl = 300",
    );
    let task = only_task(PRESENCE_TERM_RECORD, LEFT, &right_one);
    assert!(task.note.is_none());
    assert!(!format_tasks(std::slice::from_ref(&task)).contains("note "));
}

#[test]
fn a_presence_name_staging_calls_conflicting_carries_no_note() {
    let left_one = LEFT.replace(
        "operation store = ack",
        "operation store = ack\ncache-ttl = 300",
    );
    let right_one = RIGHT.replace(
        "operation store = receipt",
        "operation store = receipt\ncache-ttl = 600",
    );
    let task = only_task(PRESENCE_TERM_RECORD, &left_one, &right_one);
    assert!(task.note.is_none());
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(
        text.contains("staging clean 1, conflicting 3, one-sided 0, missing 0"),
        "{text}"
    );
    assert!(!text.contains("note "), "{text}");
}

#[test]
fn a_presence_name_staging_calls_clean_carries_the_disagreement_note() {
    let left_one = LEFT.replace(
        "operation store = ack",
        "operation store = ack\ncache-ttl = 300",
    );
    let right_one = RIGHT.replace(
        "operation store = receipt",
        "operation store = receipt\ncache-ttl = 300",
    );
    let task = only_task(PRESENCE_TERM_RECORD, &left_one, &right_one);
    assert!(task.note.is_some());
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(
        text.contains("staging clean 2, conflicting 2, one-sided 0, missing 0"),
        "{text}"
    );
    assert!(
        text.contains(
            "note the staging cross-apply classifies this name clean and this task follows the record"
        ),
        "{text}"
    );
}

#[test]
fn a_presence_name_staging_calls_missing_carries_no_note() {
    let task = task_default(PRESENCE_TERM_RECORD);
    assert!(task.note.is_none());
    let text = format_tasks(std::slice::from_ref(&task));
    assert!(
        text.contains("staging clean 1, conflicting 2, one-sided 0, missing 1"),
        "{text}"
    );
    assert!(!text.contains("note "), "{text}");
}

#[test]
fn build_tasks_refuses_a_staging_summary_from_another_session() {
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let other_left = state(&LEFT.replace("engineer-a", "engineer-c"), "other.state");
    let foreign = cross_apply(&other_left, &right, &[]).expect("staging runs");
    let error = build_tasks(&left, &right, &[], &foreign).expect_err("task refuses");
    assert!(matches!(error, CrosscurrentError::Task(_)));
}
