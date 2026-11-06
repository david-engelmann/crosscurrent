//! Pins for the bundle set identity of XC-33, the canonical
//! row order of XC-34, and the fence of XC-35.
//!
//! The shapes here are the ones the Round 6 probes produced
//! from the genuine renderer. An excluded-but-defined block
//! with two defining lines on the excluding row's other side
//! is the one block whose rows the record grammar leaves
//! free to stand in more than one order, so its permuted
//! copies are the duplicates the set identity exists to
//! collapse, and a presence block with a further row is the
//! other genuine rendering whose rows stand out of the
//! printed-form order.

mod common;

use common::{record, run_cli, state, temp_dir, write};
use crosscurrent::records::bundle_divergences;
use crosscurrent::staging::cross_apply;
use crosscurrent::tasks::{build_tasks, build_tasks_bundle, format_tasks};

// The branch states the probes staged, the left defining
// dual and timeout and the right defining timeout alone,
// so dual stands one-sided in the staging summary.
const LEFT: &str = "\
interface shard-store
author engineer-a

dual = 1
operation dual = ack
timeout = 30
";

const RIGHT: &str = "\
interface shard-store
author engineer-b

timeout = 30
";

// The dual excluded block as forkdrift's renderer emits it
// over the pair the probes taped, the defining side on the
// left, its rows standing out of printed-form order.
const DUAL_RECORD: &str = "\
divergence shard-store dual excluded-but-defined
  left (dual-a.tape) dual = 1
  right (dual-b.tape) excludes = dual
  left (dual-a.tape) operation dual = ack
";

// The same divergence with its rows permuted, a record the
// grammar admits on the same terms.
const DUAL_RECORD_PERMUTED: &str = "\
divergence shard-store dual excluded-but-defined
  left (dual-a.tape) operation dual = ack
  right (dual-b.tape) excludes = dual
  left (dual-a.tape) dual = 1
";

// The same divergence with one defining line changed, so
// its row set differs from the dual record's in one row.
const DUAL_RECORD_CHANGED: &str = "\
divergence shard-store dual excluded-but-defined
  left (dual-a.tape) dual = 1
  right (dual-b.tape) excludes = dual
  left (dual-a.tape) operation dual = reset
";

// A presence block with a further row, the renderer's
// order, the left operation row behind the right interface
// row. Its sources are the dual records' pair, as XC-19
// requires of one bundle.
const PRESENCE_RECORD: &str = "\
divergence shard-store purge presence
  left (dual-a.tape) purge = on
  right (dual-b.tape) interface shard-store
  left (dual-a.tape) operation purge = ack
";

fn bundle_document(records: &[(&str, &str)]) -> String {
    let pairs: Vec<(String, Vec<crosscurrent::records::Divergence>)> = records
        .iter()
        .map(|(name, text)| (name.to_string(), record(text, name)))
        .collect();
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let union = bundle_divergences(&pairs).expect("the bundle unions");
    let staging = cross_apply(&left, &right, &union).expect("staging runs");
    let tasks = build_tasks_bundle(&left, &right, &pairs, &staging).expect("bundle tasks build");
    format_tasks(&tasks)
}

#[test]
fn a_row_permuted_duplicate_collapses_to_one_task() {
    let document = bundle_document(&[("a.txt", DUAL_RECORD), ("b.txt", DUAL_RECORD_PERMUTED)]);
    let tasks = document.matches("task shard-store dual\n").count();
    assert_eq!(
        tasks, 1,
        "the permuted duplicate is one divergence, {document}"
    );
    let alone = bundle_document(&[("a.txt", DUAL_RECORD)]);
    assert_eq!(document, alone, "the collapse states the one task");
}

#[test]
fn an_exact_duplicate_still_collapses_to_one_task() {
    let document = bundle_document(&[("a.txt", DUAL_RECORD), ("b.txt", DUAL_RECORD)]);
    assert_eq!(document.matches("task shard-store dual\n").count(), 1);
}

#[test]
fn the_collapsed_task_prints_its_rows_in_printed_form_order() {
    let document = bundle_document(&[("a.txt", DUAL_RECORD)]);
    let expected = "\
task shard-store dual
authors engineer-a engineer-b
kind excluded-but-defined
  left (dual-a.tape) dual = 1
  left (dual-a.tape) operation dual = ack
  right (dual-b.tape) excludes = dual
size medium score 4
factors definitions 2, result-type yes, breaking no
staging clean 1, conflicting 0, one-sided 1, missing 0
";
    assert_eq!(document, expected);
}

#[test]
fn record_order_and_row_order_change_nothing_in_a_bundle() {
    // Two divergences over the pair, one of them carried by
    // two records in two row orders, so both the record
    // order and the row order are exercised at once.
    let forward = bundle_document(&[
        ("a.txt", DUAL_RECORD),
        ("b.txt", DUAL_RECORD_PERMUTED),
        ("c.txt", PRESENCE_RECORD),
    ]);
    let backward = bundle_document(&[
        ("c.txt", PRESENCE_RECORD),
        ("b.txt", DUAL_RECORD_PERMUTED),
        ("a.txt", DUAL_RECORD),
    ]);
    assert_eq!(forward, backward);
    let rows_permuted = bundle_document(&[("a.txt", DUAL_RECORD_PERMUTED)]);
    let rows_as_rendered = bundle_document(&[("a.txt", DUAL_RECORD)]);
    assert_eq!(rows_permuted, rows_as_rendered);
}

#[test]
fn two_divergences_differing_in_one_row_stay_two_tasks() {
    let document = bundle_document(&[("a.txt", DUAL_RECORD), ("b.txt", DUAL_RECORD_CHANGED)]);
    assert_eq!(document.matches("task shard-store dual\n").count(), 2);
}

#[test]
fn the_union_carries_the_divergence_with_its_rows_in_canonical_order() {
    let pairs = vec![
        ("a.txt".to_string(), record(DUAL_RECORD_PERMUTED, "a.txt")),
        ("b.txt".to_string(), record(DUAL_RECORD, "b.txt")),
    ];
    let union = bundle_divergences(&pairs).expect("the bundle unions");
    assert_eq!(union.len(), 1);
    let rows: Vec<String> = union[0]
        .rows
        .iter()
        .map(|row| format!("  {} ({}) {}", row.side, row.source, row.line))
        .collect();
    assert_eq!(
        rows,
        [
            "  left (dual-a.tape) dual = 1",
            "  left (dual-a.tape) operation dual = ack",
            "  right (dual-b.tape) excludes = dual",
        ]
    );
}

#[test]
fn a_presence_block_with_a_further_row_prints_its_rows_in_printed_form_order() {
    let document = bundle_document(&[("a.txt", PRESENCE_RECORD)]);
    // The printed form sorts the operation line ahead of
    // the term line here, because the quoted lines compare
    // as strings and operation precedes purge.
    let block = "\
task shard-store purge
authors engineer-a engineer-b
kind presence
  left (dual-a.tape) operation purge = ack
  left (dual-a.tape) purge = on
  right (dual-b.tape) interface shard-store
";
    assert!(
        document.starts_with(block),
        "the further row stands with its side, {document}"
    );
}

#[test]
fn the_single_record_paths_keep_record_order() {
    // XC-35: build_tasks over one record never passes
    // through the union, so the genuine rendering's row
    // order stands exactly as the record carries it.
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let divergences = record(DUAL_RECORD, "a.txt");
    let staging = cross_apply(&left, &right, &divergences).expect("staging runs");
    let tasks = build_tasks(&left, &right, &divergences, &staging).expect("tasks build");
    let document = format_tasks(&tasks);
    let rows = "\
  left (dual-a.tape) dual = 1
  right (dual-b.tape) excludes = dual
  left (dual-a.tape) operation dual = ack
";
    assert!(document.contains(rows), "record order stands, {document}");
}

#[test]
fn the_command_line_collapses_a_permuted_duplicate_in_a_bundle() {
    let dir = temp_dir("set-identity-cli");
    let left = write(&dir, "left.state", LEFT);
    let right = write(&dir, "right.state", RIGHT);
    let first = write(&dir, "a.txt", DUAL_RECORD);
    let second = write(&dir, "b.txt", DUAL_RECORD_PERMUTED);
    let out = run_cli(&["handback", &left, &right, &first, &second], &dir);
    assert_eq!(out.code, 0, "stderr: {}", out.stderr_text());
    assert_eq!(
        out.stdout_text().matches("task shard-store dual\n").count(),
        1
    );
    let alone = run_cli(&["handback", &left, &right, &first], &dir);
    assert_eq!(out.stdout_text(), alone.stdout_text());
}
