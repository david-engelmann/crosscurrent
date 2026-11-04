//! The ported test_bundle corpus, test for test, with the
//! XC-20 identity pins standing after it. A divergence is
//! its interface, name, kind, and full quoted row tuple, so
//! a block that differs only in its excludes row, its
//! further row, or its interface row never collapses into
//! another.

mod common;

use common::{record, run_cli, run_cli_at_root, state, temp_dir, write};
use crosscurrent::records::{Divergence, QuotedLine, bundle_divergences, record_sources};
use crosscurrent::tasks::build_tasks_bundle;
use crosscurrent::{CrosscurrentError, Task, build_tasks, cross_apply, format_tasks};

const LEFT: &str = "\
interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
cache-ttl = 300
";

const RIGHT: &str = "\
interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
";

const EARLIER_RECORD: &str = "divergence shard-store store result-type\n  left (run-a.tape) operation store = ack\n  right (run-b.tape) operation store = receipt\ndivergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";

const LATER_RECORD: &str = "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\ndivergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";

const FOREIGN_RECORD: &str = "divergence shard-store timeout term-value\n  left (other-a.tape) timeout = 30\n  right (other-b.tape) timeout = 45\n";

const CHANGED_TIMEOUT_RECORD: &str = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 50\n";

const MISSING_RECORD: &str = "divergence shard-store purge presence\n  left (run-a.tape) operation purge = ack\n  right (run-b.tape) interface shard-store\n";

fn named_records(record_texts: &[&str]) -> Vec<(String, Vec<crosscurrent::Divergence>)> {
    record_texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let name = format!("record-{index}.txt");
            (name.clone(), record(text, &name))
        })
        .collect()
}

fn bundle_tasks(record_texts: &[&str]) -> Vec<Task> {
    bundle_tasks_lr(record_texts, LEFT, RIGHT)
}

fn bundle_tasks_lr(record_texts: &[&str], left_text: &str, right_text: &str) -> Vec<Task> {
    let left = state(left_text, "left.state");
    let right = state(right_text, "right.state");
    let records = named_records(record_texts);
    let union = bundle_divergences(&records).expect("bundle unions");
    let staging = cross_apply(&left, &right, &union).expect("staging runs");
    build_tasks_bundle(&left, &right, &records, &staging).expect("tasks build")
}

fn single_tasks(record_text: &str) -> Vec<Task> {
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let divergences = record(record_text, "record.txt");
    let staging = cross_apply(&left, &right, &divergences).expect("staging runs");
    build_tasks(&left, &right, &divergences, &staging).expect("tasks build")
}

#[test]
fn a_bundle_builds_tasks_from_the_union_of_its_records() {
    let found = bundle_tasks(&[EARLIER_RECORD, LATER_RECORD]);
    let names: Vec<&str> = found.iter().map(|task| task.name.as_str()).collect();
    assert_eq!(names, ["cache-ttl", "store", "timeout"]);
    let kinds: Vec<&str> = found.iter().map(|task| task.kind.as_str()).collect();
    assert_eq!(kinds, ["presence", "result-type", "term-value"]);
}

#[test]
fn an_exact_duplicate_divergence_collapses_to_one_task() {
    let found = bundle_tasks(&[EARLIER_RECORD, LATER_RECORD]);
    let timeouts: Vec<&Task> = found.iter().filter(|task| task.name == "timeout").collect();
    assert_eq!(timeouts.len(), 1);
    let doubled = bundle_tasks(&[EARLIER_RECORD, EARLIER_RECORD]);
    assert_eq!(
        format_tasks(&doubled),
        format_tasks(&single_tasks(EARLIER_RECORD))
    );
}

#[test]
fn the_same_name_and_kind_with_different_rows_produces_two_tasks() {
    let found = bundle_tasks(&[EARLIER_RECORD, CHANGED_TIMEOUT_RECORD]);
    let timeouts: Vec<&Task> = found.iter().filter(|task| task.name == "timeout").collect();
    assert_eq!(timeouts.len(), 2);
    let text = format_tasks(&found);
    let first = text.find("timeout = 45").expect("first row text");
    let second = text.find("timeout = 50").expect("second row text");
    assert!(first < second, "{text}");
}

#[test]
fn the_tiebreak_makes_record_order_change_nothing() {
    let forward = bundle_tasks(&[EARLIER_RECORD, CHANGED_TIMEOUT_RECORD]);
    let backward = bundle_tasks(&[CHANGED_TIMEOUT_RECORD, EARLIER_RECORD]);
    assert_eq!(format_tasks(&forward), format_tasks(&backward));
    let forward = bundle_tasks(&[EARLIER_RECORD, LATER_RECORD]);
    let backward = bundle_tasks(&[LATER_RECORD, EARLIER_RECORD]);
    assert_eq!(format_tasks(&forward), format_tasks(&backward));
}

#[test]
fn a_bundle_of_one_record_prints_the_single_record_handback() {
    let bundled = bundle_tasks(&[LATER_RECORD]);
    let single = single_tasks(LATER_RECORD);
    assert_eq!(format_tasks(&bundled), format_tasks(&single));
}

#[test]
fn a_record_from_other_sources_refuses_the_bundle() {
    let records = named_records(&[EARLIER_RECORD, FOREIGN_RECORD]);
    let error = bundle_divergences(&records).expect_err("bundle refuses");
    let CrosscurrentError::Record(inner) = &error else {
        panic!("expected a record refusal, got {error:?}");
    };
    assert_eq!(
        inner.0,
        "record-1.txt establishes left other-a.tape and right \
         other-b.tape and record-0.txt establishes left run-a.tape \
         and right run-b.tape, and the records of one bundle must \
         establish the one pair of sources"
    );
}

#[test]
fn an_empty_record_contributes_nothing_and_binds_nothing() {
    let with_empty = bundle_tasks(&["", LATER_RECORD]);
    let plain = bundle_tasks(&[LATER_RECORD]);
    assert_eq!(format_tasks(&with_empty), format_tasks(&plain));
    assert_eq!(bundle_tasks(&[""]).len(), 0);
    let foreign_after_empty = bundle_tasks(&["", EARLIER_RECORD]);
    assert_eq!(
        format_tasks(&foreign_after_empty),
        format_tasks(&single_tasks(EARLIER_RECORD))
    );
}

#[test]
fn the_bundle_staging_summary_counts_the_union() {
    let found = bundle_tasks(&[EARLIER_RECORD, MISSING_RECORD]);
    assert!(!found.is_empty());
    for task in &found {
        let text = format_tasks(std::slice::from_ref(task));
        assert!(
            text.contains("staging clean 1, conflicting 2, one-sided 1, missing 1"),
            "{text}"
        );
    }
    let single = single_tasks(EARLIER_RECORD);
    for task in &single {
        let text = format_tasks(std::slice::from_ref(task));
        assert!(text.contains("missing 0"), "{text}");
    }
}

#[test]
fn cli_handback_accepts_several_record_files() {
    let dir = temp_dir("bundle-cli");
    let left = write(&dir, "left.state", LEFT);
    let right = write(&dir, "right.state", RIGHT);
    let earlier = write(&dir, "earlier.txt", EARLIER_RECORD);
    let later = write(&dir, "later.txt", LATER_RECORD);
    let result = run_cli(&["handback", &left, &right, &earlier, &later], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stderr, Vec::<u8>::new());
    let expected = format_tasks(&bundle_tasks(&[EARLIER_RECORD, LATER_RECORD]));
    assert_eq!(result.stdout_text(), expected);
    let swapped = run_cli(&["handback", &left, &right, &later, &earlier], &dir);
    assert_eq!(swapped.code, 0);
    assert_eq!(swapped.stdout, result.stdout);
}

#[test]
fn cli_handback_with_no_record_file_is_a_usage_error() {
    let dir = temp_dir("bundle-cli-none");
    let left = write(&dir, "left.state", LEFT);
    let right = write(&dir, "right.state", RIGHT);
    let result = run_cli(&["handback", &left, &right], &dir);
    assert_eq!(result.code, 2);
    assert!(result.stderr_text().to_lowercase().contains("usage"));
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_a_bundle_from_other_sources_exits_2_with_one_error_line() {
    let dir = temp_dir("bundle-cli-foreign");
    let left = write(&dir, "left.state", LEFT);
    let right = write(&dir, "right.state", RIGHT);
    let earlier = write(&dir, "earlier.txt", EARLIER_RECORD);
    let foreign = write(&dir, "foreign.txt", FOREIGN_RECORD);
    let result = run_cli(&["handback", &left, &right, &earlier, &foreign], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert_eq!(stderr.matches('\n').count(), 1, "{stderr}");
    assert!(stderr.contains("other-a.tape"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_a_duplicate_record_file_collapses_like_one() {
    let dir = temp_dir("bundle-cli-dupe");
    let left = write(&dir, "left.state", LEFT);
    let right = write(&dir, "right.state", RIGHT);
    let later = write(&dir, "later.txt", LATER_RECORD);
    let once = run_cli(&["handback", &left, &right, &later], &dir);
    let twice = run_cli(&["handback", &left, &right, &later, &later], &dir);
    assert_eq!(once.code, 0);
    assert_eq!(twice.code, 0);
    assert_eq!(twice.stdout, once.stdout);
    assert_eq!(
        once.stdout_text(),
        format_tasks(&bundle_tasks(&[LATER_RECORD]))
    );
}

fn example_text(name: &str) -> String {
    let path = common::root().join("examples").join(name);
    std::fs::read_to_string(path).expect("example readable")
}

#[test]
fn bundle_example_records_establish_the_one_pair_of_sources() {
    for name in ["bundle-divergences-a.txt", "bundle-divergences-b.txt"] {
        let found = record(&example_text(name), &format!("examples/{name}"));
        assert_eq!(
            record_sources(&found),
            Some((
                Some("examples/presence-run-a.tape".to_string()),
                Some("examples/presence-run-b.tape".to_string())
            ))
        );
    }
}

#[test]
fn bundle_example_records_carry_a_shared_block_and_one_each_more() {
    let first = record(
        &example_text("bundle-divergences-a.txt"),
        "examples/bundle-divergences-a.txt",
    );
    let second = record(
        &example_text("bundle-divergences-b.txt"),
        "examples/bundle-divergences-b.txt",
    );
    let first_keys: std::collections::HashSet<(&str, &str)> = first
        .iter()
        .map(|d| (d.name.as_str(), d.kind.as_str()))
        .collect();
    let second_keys: std::collections::HashSet<(&str, &str)> = second
        .iter()
        .map(|d| (d.name.as_str(), d.kind.as_str()))
        .collect();
    let shared: Vec<&(&str, &str)> = first_keys.intersection(&second_keys).collect();
    assert_eq!(
        shared,
        [("timeout", "term-value")].iter().collect::<Vec<_>>()
    );
    assert_eq!(first_keys.len(), 2);
    assert_eq!(second_keys.len(), 2);
}

#[test]
fn committed_bundle_example_matches_regeneration() {
    let result = run_cli_at_root(&[
        "handback",
        "examples/presence-branch-a.state",
        "examples/presence-branch-b.state",
        "examples/bundle-divergences-a.txt",
        "examples/bundle-divergences-b.txt",
    ]);
    assert_eq!(result.code, 0);
    let committed = std::fs::read(common::root().join("examples").join("bundle-handback.txt"))
        .expect("committed handback readable");
    assert_eq!(result.stdout, committed);
    let text = result.stdout_text();
    assert_eq!(text.matches("task shard-store ").count(), 3);
    assert_eq!(text.matches("task shard-store timeout").count(), 1);
    let second = run_cli_at_root(&[
        "handback",
        "examples/presence-branch-a.state",
        "examples/presence-branch-b.state",
        "examples/bundle-divergences-b.txt",
        "examples/bundle-divergences-a.txt",
    ]);
    assert_eq!(second.stdout, result.stdout);
}

#[test]
fn identity_differing_only_in_the_excludes_row_does_not_collapse() {
    let wide = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) excludes = dual, legacy-mode\n  right (run-b.tape) dual = 1\n";
    let narrow = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) excludes = dual\n  right (run-b.tape) dual = 1\n";
    let found = bundle_tasks(&[wide, narrow]);
    assert_eq!(found.len(), 2, "{}", format_tasks(&found));
    let same = bundle_tasks(&[narrow, narrow]);
    assert_eq!(same.len(), 1);
}

#[test]
fn identity_differing_only_in_the_further_row_does_not_collapse() {
    let with_further = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n  left (run-a.tape) operation dual = ack\n";
    let without_further = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n";
    let found = bundle_tasks(&[with_further, without_further]);
    assert_eq!(found.len(), 2, "{}", format_tasks(&found));
}

#[test]
fn identity_differing_only_in_the_interface_row_does_not_collapse() {
    let left_defines = "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n";
    let right_defines = "divergence shard-store dual presence\n  left (run-a.tape) interface shard-store\n  right (run-b.tape) dual = 1\n";
    let found = bundle_tasks(&[left_defines, right_defines]);
    assert_eq!(found.len(), 2, "{}", format_tasks(&found));
    let same = bundle_tasks(&[left_defines, left_defines]);
    assert_eq!(same.len(), 1);
}

/// The pure form of the pin above, built from divergence
/// values so nothing but the interface row's quoted text
/// differs. The two blocks name the same interface, term,
/// and kind, quote the same defining row, and carry their
/// interface row on the same side from the same source.
/// Only the line the interface row quotes differs, so an
/// identity that left the interface row out of the quoted
/// rows would collapse them, and the union must keep both.
#[test]
fn identity_differing_only_in_the_interface_rows_quoted_text_does_not_collapse() {
    let row = |side: &str, source: &str, line: &str| QuotedLine {
        side: side.to_string(),
        source: source.to_string(),
        line: line.to_string(),
    };
    let presence = |interface_line: &str| Divergence {
        interface: "shard-store".to_string(),
        name: "dual".to_string(),
        kind: "presence".to_string(),
        rows: vec![
            row("left", "run-a.tape", "dual = 1"),
            row("right", "run-b.tape", interface_line),
        ],
        excludes_row: None,
        interface_row: Some(row("right", "run-b.tape", interface_line)),
    };
    let records = vec![
        (
            "one.txt".to_string(),
            vec![presence("interface shard-store")],
        ),
        (
            "two.txt".to_string(),
            vec![presence("interface shard-cache")],
        ),
    ];
    let union = bundle_divergences(&records).expect("bundle unions");
    assert_eq!(union.len(), 2);
    let same = vec![
        (
            "one.txt".to_string(),
            vec![presence("interface shard-store")],
        ),
        (
            "two.txt".to_string(),
            vec![presence("interface shard-store")],
        ),
    ];
    let union = bundle_divergences(&same).expect("bundle unions");
    assert_eq!(union.len(), 1);
}
