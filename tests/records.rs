//! The ported test_records corpus, test for test.

mod common;

use common::{record, record_err};
use crosscurrent::CrosscurrentError;

const TERM_RECORD: &str = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";

const RESULT_RECORD: &str = "divergence shard-store store result-type\n  left (run-a.tape) operation store = ack\n  right (run-b.tape) operation store = receipt\n";

const EXCLUDED_RECORD: &str = "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) excludes = legacy-mode\n  right (run-b.tape) legacy-mode = on\n";

// The further row is labelled for the side its line comes
// from, as forkdrift's FD-18 renders it and XC-18 holds
// every row to. An earlier form of this fixture labelled
// that row right, which was forkdrift's rendering before
// FD-18 corrected it.
const EXCLUDED_DUAL_RECORD: &str = "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n  left (run-a.tape) operation dual = ack\n";

const PRESENCE_TERM_RECORD: &str = "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n";

const PRESENCE_OPERATION_RECORD: &str = "divergence shard-store purge presence\n  left (run-a.tape) interface shard-store\n  right (run-b.tape) operation purge = ack\n";

const PRESENCE_DUAL_LEFT_RECORD: &str = "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n  left (run-a.tape) operation dual = ack\n";

const PRESENCE_DUAL_RIGHT_RECORD: &str = "divergence shard-store dual presence\n  left (run-a.tape) interface shard-store\n  right (run-b.tape) dual = 1\n  right (run-b.tape) operation dual = ack\n";

const PRESENCE_CROSS_RECORD: &str = "divergence shard-store fetch presence\n  left (run-a.tape) fetch = cached\n  right (run-b.tape) operation fetch = record\n";

fn refused(text: &str, fragment: &str) {
    let error = record_err(text, "record.txt");
    let CrosscurrentError::Record(inner) = &error else {
        panic!("expected a record refusal, got {error:?}");
    };
    assert!(
        inner.0.contains(fragment),
        "refusal {:?} lacks {:?} for input {:?}",
        inner.0,
        fragment,
        text
    );
    assert!(inner.0.contains("record.txt"), "{}", inner.0);
}

#[test]
fn parses_a_term_value_block() {
    let found = record(TERM_RECORD, "record.txt");
    assert_eq!(found.len(), 1);
    let block = &found[0];
    assert_eq!(block.interface, "shard-store");
    assert_eq!(block.name, "timeout");
    assert_eq!(block.kind, "term-value");
    let sides: Vec<&str> = block.rows.iter().map(|row| row.side.as_str()).collect();
    assert_eq!(sides, ["left", "right"]);
    assert_eq!(block.rows[0].source, "run-a.tape");
    assert_eq!(block.rows[0].line, "timeout = 30");
    let definitions: Vec<&crosscurrent::QuotedLine> = block.definitions();
    assert_eq!(definitions.len(), block.rows.len());
    assert!(block.excludes_row.is_none());
}

#[test]
fn parses_a_result_type_block() {
    let found = record(RESULT_RECORD, "record.txt");
    assert_eq!(found[0].kind, "result-type");
    assert_eq!(found[0].definitions()[1].line, "operation store = receipt");
}

#[test]
fn parses_an_excluded_but_defined_block() {
    let found = record(EXCLUDED_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.kind, "excluded-but-defined");
    assert_eq!(
        block.excludes_row.as_ref().map(|row| row.line.as_str()),
        Some("excludes = legacy-mode")
    );
    let lines: Vec<&str> = block
        .definitions()
        .iter()
        .map(|row| row.line.as_str())
        .collect();
    assert_eq!(lines, ["legacy-mode = on"]);
}

#[test]
fn parses_a_dual_definition_excluded_block() {
    let found = record(EXCLUDED_DUAL_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.rows.len(), 3);
    let lines: Vec<&str> = block
        .definitions()
        .iter()
        .map(|row| row.line.as_str())
        .collect();
    assert_eq!(lines, ["dual = 1", "operation dual = ack"]);
    let sources: std::collections::HashSet<&str> = block
        .definitions()
        .iter()
        .map(|row| row.source.as_str())
        .collect();
    assert_eq!(sources, std::collections::HashSet::from(["run-a.tape"]));
    assert_eq!(
        block.excludes_row.as_ref().map(|row| row.source.as_str()),
        Some("run-b.tape")
    );
}

#[test]
fn parses_several_blocks_for_different_names() {
    let text = format!("{TERM_RECORD}\n{RESULT_RECORD}");
    let found = record(&text, "r");
    let names: Vec<&str> = found.iter().map(|block| block.name.as_str()).collect();
    assert_eq!(names, ["timeout", "store"]);
}

#[test]
fn an_empty_record_parses_to_no_divergences() {
    assert_eq!(record("", "r").len(), 0);
    assert_eq!(record("\n\n", "r").len(), 0);
}

#[test]
fn malformed_records_are_refused_by_name() {
    let cases: Vec<(String, &str)> = vec![
        ("divergence shard-store timeout\n".into(), "matches no form"),
        (
            "divergence shard-store timeout weird-kind\n".into(),
            "does not know",
        ),
        ("  left (run-a.tape) timeout = 30\n".into(), "matches no form"),
        ("a stray line\n".into(), "matches no form"),
        (
            "divergence shard-store timeout term-value\n  left (run-a.tape) operation timeout = ack\n  right (run-b.tape) timeout = 45\n".into(),
            "not a term line for timeout",
        ),
        (
            "divergence shard-store timeout term-value\n  left (run-a.tape) retries = 3\n  right (run-b.tape) timeout = 45\n".into(),
            "not a term line for timeout",
        ),
        (
            "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  left (run-b.tape) timeout = 45\n".into(),
            "one left line and one right line",
        ),
        (
            "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n".into(),
            "one left line and one right line",
        ),
        (
            "divergence shard-store store result-type\n  left (run-a.tape) store = ack\n  right (run-b.tape) operation store = receipt\n".into(),
            "not a operation line for store",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) legacy-mode = off\n  right (run-b.tape) legacy-mode = on\n".into(),
            "exactly one excludes line",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) excludes = other-name\n  right (run-b.tape) legacy-mode = on\n".into(),
            "neither an excludes line",
        ),
        (
            "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n  right (run-c.tape) operation dual = ack\n".into(),
            "two different sources",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) excludes = legacy-mode\n  right (run-a.tape) legacy-mode = on\n".into(),
            "the one source",
        ),
        (
            "divergence shard-store dual excluded-but-defined\n  left (run-a.tape) dual = 1\n  right (run-b.tape) excludes = dual\n  right (run-a.tape) dual = 2\n".into(),
            "two defining lines of the one form",
        ),
        (
            format!("{TERM_RECORD}\n{TERM_RECORD}"),
            "two blocks for name timeout",
        ),
        (
            format!(
                "{TERM_RECORD}\ndivergence shard-cache timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n"
            ),
            "two different interfaces",
        ),
    ];
    for (text, fragment) in cases {
        refused(&text, fragment);
    }
}

#[test]
fn parses_a_presence_block_with_a_term_line() {
    let found = record(PRESENCE_TERM_RECORD, "record.txt");
    assert_eq!(found.len(), 1);
    let block = &found[0];
    assert_eq!(block.interface, "shard-store");
    assert_eq!(block.name, "cache-ttl");
    assert_eq!(block.kind, "presence");
    let sides: Vec<&str> = block.rows.iter().map(|row| row.side.as_str()).collect();
    assert_eq!(sides, ["left", "right"]);
    assert_eq!(block.rows[0].line, "cache-ttl = 300");
    assert_eq!(block.rows[1].line, "interface shard-store");
    assert_eq!(block.definitions(), vec![&block.rows[0]]);
    assert_eq!(block.interface_row.as_ref(), Some(&block.rows[1]));
    assert!(block.excludes_row.is_none());
}

#[test]
fn parses_a_presence_block_with_an_operation_line_on_the_right() {
    let found = record(PRESENCE_OPERATION_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.kind, "presence");
    assert_eq!(block.definitions(), vec![&block.rows[1]]);
    assert_eq!(block.definitions()[0].line, "operation purge = ack");
    assert_eq!(block.interface_row.as_ref(), Some(&block.rows[0]));
}

#[test]
fn parses_a_presence_block_with_both_forms_on_the_left() {
    let found = record(PRESENCE_DUAL_LEFT_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.rows.len(), 3);
    assert_eq!(block.rows[2].side, "left");
    assert_eq!(block.rows[2].source, "run-a.tape");
    let lines: Vec<&str> = block
        .definitions()
        .iter()
        .map(|row| row.line.as_str())
        .collect();
    assert_eq!(lines, ["dual = 1", "operation dual = ack"]);
    assert_eq!(block.interface_row.as_ref(), Some(&block.rows[1]));
}

#[test]
fn parses_a_presence_block_with_both_forms_on_the_right() {
    let found = record(PRESENCE_DUAL_RIGHT_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.rows.len(), 3);
    assert_eq!(block.rows[2].side, "right");
    assert_eq!(block.rows[2].source, "run-b.tape");
    let lines: Vec<&str> = block
        .definitions()
        .iter()
        .map(|row| row.line.as_str())
        .collect();
    assert_eq!(lines, ["dual = 1", "operation dual = ack"]);
    assert_eq!(block.interface_row.as_ref(), Some(&block.rows[0]));
}

#[test]
fn parses_a_presence_block_with_a_form_on_each_side() {
    let found = record(PRESENCE_CROSS_RECORD, "record.txt");
    let block = &found[0];
    assert_eq!(block.rows.len(), 2);
    assert_eq!(block.definitions().len(), block.rows.len());
    assert!(block.interface_row.is_none());
}

#[test]
fn parses_a_mixed_record_with_all_four_kinds() {
    let text = format!("{TERM_RECORD}\n{RESULT_RECORD}\n{EXCLUDED_RECORD}\n{PRESENCE_TERM_RECORD}");
    let found = record(&text, "record.txt");
    let kinds: Vec<&str> = found.iter().map(|block| block.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "term-value",
            "result-type",
            "excluded-but-defined",
            "presence"
        ]
    );
}

#[test]
fn malformed_presence_blocks_are_refused_by_name() {
    let cases: [(&str, &str); 13] = [
        (
            "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n",
            "at most one further row",
        ),
        (
            "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n  left (run-a.tape) operation dual = ack\n  right (run-b.tape) operation dual = ack\n",
            "at most one further row",
        ),
        (
            "divergence shard-store cache-ttl presence\n  right (run-b.tape) interface shard-store\n  left (run-a.tape) cache-ttl = 300\n",
            "one left line and one right line",
        ),
        (
            "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) interface shard-store\n  right (run-a.tape) operation dual = ack\n",
            "not the right source",
        ),
        (
            "divergence shard-store timeout presence\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n",
            "from both sides",
        ),
        (
            "divergence shard-store ghost presence\n  left (run-a.tape) interface shard-store\n  right (run-b.tape) interface shard-store\n",
            "quotes no defining line",
        ),
        (
            "divergence shard-store cache-ttl presence\n  left (run-a.tape) label cache-ttl = hot\n  right (run-b.tape) interface shard-store\n",
            "neither a defining line for cache-ttl",
        ),
        (
            "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-cache\n",
            "neither a defining line for cache-ttl",
        ),
        (
            "divergence shard-store legacy-mode presence\n  left (run-a.tape) excludes = legacy-mode\n  right (run-b.tape) interface shard-store\n",
            "neither a defining line for legacy-mode",
        ),
        (
            "divergence shard-store cache-ttl presence\n  left (run-a.tape) other = 1\n  right (run-b.tape) interface shard-store\n",
            "neither a defining line for cache-ttl",
        ),
        (
            "divergence shard-store dual presence\n  left (run-a.tape) operation dual = ack\n  right (run-b.tape) interface shard-store\n  left (run-a.tape) dual = 1\n",
            "not a term line then an operation line",
        ),
        (
            "divergence shard-store dual presence\n  left (run-a.tape) interface shard-store\n  right (run-b.tape) dual = 1\n  left (run-a.tape) operation dual = ack\n",
            "interface line on the left",
        ),
        (
            "divergence shard-store dual presence\n  left (run-a.tape) dual = 1\n  right (run-b.tape) dual = 2\n  left (run-a.tape) interface shard-store\n",
            "interface line as its further row",
        ),
    ];
    for (text, fragment) in cases {
        refused(text, fragment);
    }
}

#[test]
fn a_multi_block_record_with_further_and_excludes_rows_parses() {
    let text = format!(
        "{EXCLUDED_DUAL_RECORD}\n{PRESENCE_DUAL_LEFT_RECORD}\n{TERM_RECORD}\n{RESULT_RECORD}"
    );
    let found = record(&text, "record.txt");
    let kinds: Vec<&str> = found.iter().map(|block| block.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "excluded-but-defined",
            "presence",
            "term-value",
            "result-type"
        ]
    );
    let excluded = &found[0];
    let lines: Vec<&str> = excluded
        .definitions()
        .iter()
        .map(|row| row.line.as_str())
        .collect();
    assert_eq!(lines, ["dual = 1", "operation dual = ack"]);
    let sources: std::collections::HashSet<&str> = excluded
        .definitions()
        .iter()
        .map(|row| row.source.as_str())
        .collect();
    assert_eq!(sources, std::collections::HashSet::from(["run-a.tape"]));
    assert_eq!(excluded.rows[2].side, "left");
    assert_eq!(
        excluded
            .excludes_row
            .as_ref()
            .map(|row| row.source.as_str()),
        Some("run-b.tape")
    );
    assert_eq!(found[1].interface_row.as_ref(), Some(&found[1].rows[1]));
}

#[test]
fn inconsistent_row_sources_are_refused_by_name() {
    let cases: Vec<(String, &str, &str)> = vec![
        (
            format!(
                "{TERM_RECORD}\ndivergence shard-store cache-ttl presence\n  left (run-b.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n"
            ),
            "cache-ttl",
            "not the left source of the record",
        ),
        (
            format!(
                "{TERM_RECORD}\ndivergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-a.tape) interface shard-store\n"
            ),
            "cache-ttl",
            "not the right source of the record",
        ),
        (
            format!(
                "{TERM_RECORD}\ndivergence shard-store legacy-mode excluded-but-defined\n  left (run-c.tape) excludes = legacy-mode\n  right (run-b.tape) legacy-mode = on\n"
            ),
            "legacy-mode",
            "not the left source of the record",
        ),
        (
            "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-a.tape) timeout = 45\n".into(),
            "timeout",
            "names the one source run-a.tape on both sides",
        ),
        (
            format!(
                "{TERM_RECORD}\n{RESULT_RECORD}\ndivergence shard-store retries term-value\n  left (run-a.tape) retries = 3\n  right (run-a.tape) retries = 4\n"
            ),
            "retries",
            "not the right source of the record",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n  left (run-a.tape) excludes = legacy-mode\n  left (run-b.tape) legacy-mode = on\n".into(),
            "legacy-mode",
            "not the left source of the record",
        ),
        (
            "divergence shard-store cache-ttl presence\n  left (run-b.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n".into(),
            "cache-ttl",
            "names the one source run-b.tape on both sides",
        ),
    ];
    for (text, name, fragment) in cases {
        let error = record_err(&text, "record.txt");
        let CrosscurrentError::Record(inner) = &error else {
            panic!("expected a record refusal, got {error:?}");
        };
        assert!(inner.0.contains(name), "{}", inner.0);
        assert!(
            inner.0.contains(fragment),
            "refusal {:?} lacks {:?}",
            inner.0,
            fragment
        );
        assert!(inner.0.contains("record.txt"), "{}", inner.0);
    }
}
