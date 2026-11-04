//! The ported test_staging corpus, test for test.

mod common;

use common::{record, state};
use crosscurrent::CrosscurrentError;
use crosscurrent::staging::{CrossApply, cross_apply, format_stage_report};

const LEFT: &str = "\
interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
cache-ttl = 300
excludes = legacy-mode
";

const RIGHT: &str = "\
interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
operation fetch = record
";

const GHOST_RECORD: &str = "\
divergence shard-store ghost term-value
  left (run-a.tape) ghost = 1
  right (run-b.tape) ghost = 2
";

fn stage(left_text: &str, right_text: &str, record_text: &str) -> CrossApply {
    let left = state(left_text, "left.state");
    let right = state(right_text, "right.state");
    let divergences = record(record_text, "record.txt");
    cross_apply(&left, &right, &divergences).expect("cross apply runs")
}

fn stage_default() -> CrossApply {
    stage(LEFT, RIGHT, "")
}

#[test]
fn cross_apply_classifies_every_defined_name() {
    let result = stage_default();
    assert_eq!(result.clean, ["replicas"]);
    assert_eq!(result.conflicting, ["store", "timeout"]);
    assert_eq!(
        result.one_sided,
        [
            ("cache-ttl".to_string(), "left".to_string()),
            ("fetch".to_string(), "right".to_string())
        ]
    );
    assert_eq!(result.missing, Vec::<String>::new());
    assert_eq!(result.left_author, "engineer-a");
    assert_eq!(result.right_author, "engineer-b");
}

#[test]
fn numeric_worth_and_whitespace_normalizations_are_clean() {
    let right = RIGHT.replace("timeout = 45", "timeout = 30.0");
    let result = stage(LEFT, &right, "");
    assert!(result.clean.contains(&"timeout".to_string()));
    let left = "interface shard-store\nauthor a\n\nlimit = 1e3\n";
    let right = "interface shard-store\nauthor b\n\nlimit = 1000\n";
    assert_eq!(stage(left, right, "").clean, ["limit"]);
    let left = "interface shard-store\nauthor a\n\nnote = two   words\n";
    let right = "interface shard-store\nauthor b\n\nnote = two words\n";
    assert_eq!(stage(left, right, "").clean, ["note"]);
}

#[test]
fn a_name_defined_in_different_namespaces_conflicts() {
    let left = "interface shard-store\nauthor a\n\nfetch = cached\n";
    let right = "interface shard-store\nauthor b\n\noperation fetch = record\n";
    let result = stage(left, right, "");
    assert_eq!(result.conflicting, ["fetch"]);
    assert_eq!(result.clean, Vec::<String>::new());
    assert_eq!(result.one_sided, Vec::<(String, String)>::new());
}

#[test]
fn a_name_defined_in_both_namespaces_can_be_clean() {
    let text_a = "interface shard-store\nauthor a\n\ndual = 1\noperation dual = ack\n";
    let text_b = "interface shard-store\nauthor b\n\ndual = 1\noperation dual = ack\n";
    assert_eq!(stage(text_a, text_b, "").clean, ["dual"]);
    let text_c = "interface shard-store\nauthor b\n\ndual = 1\noperation dual = receipt\n";
    assert_eq!(stage(text_a, text_c, "").conflicting, ["dual"]);
}

#[test]
fn labels_excludes_and_breaking_lines_never_change_classification() {
    let decorated_left = LEFT.replace(
        "cache-ttl = 300",
        "cache-ttl = 300\nlabel primary = node-a\nbreaking timeout",
    );
    let plain = stage_default();
    let decorated = stage(&decorated_left, RIGHT, "");
    assert_eq!(decorated.clean, plain.clean);
    assert_eq!(decorated.conflicting, plain.conflicting);
    assert_eq!(decorated.one_sided, plain.one_sided);
}

#[test]
fn a_diverged_name_defined_nowhere_is_reported_missing() {
    let result = stage(LEFT, RIGHT, GHOST_RECORD);
    assert_eq!(result.missing, ["ghost"]);
    assert!(!result.clean.contains(&"ghost".to_string()));
    assert!(!result.conflicting.contains(&"ghost".to_string()));
    assert!(result.one_sided.iter().all(|(name, _)| name != "ghost"));
}

#[test]
fn a_presence_name_defined_nowhere_is_reported_missing() {
    let text = "divergence shard-store ghost presence\n  left (run-a.tape) ghost = 1\n  right (run-b.tape) interface shard-store\n";
    let result = stage(LEFT, RIGHT, text);
    assert_eq!(result.missing, ["ghost"]);
    assert!(!result.clean.contains(&"ghost".to_string()));
    assert!(!result.conflicting.contains(&"ghost".to_string()));
    assert!(result.one_sided.iter().all(|(name, _)| name != "ghost"));
}

#[test]
fn a_presence_name_defined_on_one_side_is_one_sided() {
    let text = "divergence shard-store cache-ttl presence\n  left (run-a.tape) cache-ttl = 300\n  right (run-b.tape) interface shard-store\n";
    let result = stage(LEFT, RIGHT, text);
    assert!(
        result
            .one_sided
            .contains(&("cache-ttl".to_string(), "left".to_string()))
    );
    assert_eq!(result.missing, Vec::<String>::new());
}

#[test]
fn a_diverged_name_defined_in_the_states_is_not_missing() {
    let text = "divergence shard-store timeout term-value\n  left (run-a.tape) timeout = 30\n  right (run-b.tape) timeout = 45\n";
    assert_eq!(stage(LEFT, RIGHT, text).missing, Vec::<String>::new());
}

#[test]
fn states_naming_different_interfaces_are_refused() {
    let other = RIGHT.replace("interface shard-store", "interface shard-cache");
    let left = state(LEFT, "left.state");
    let right = state(&other, "right.state");
    let error = cross_apply(&left, &right, &[]).expect_err("session refuses");
    let CrosscurrentError::InterfaceMismatch(inner) = &error else {
        panic!("expected an interface mismatch, got {error:?}");
    };
    assert!(inner.0.contains("shard-store"), "{}", inner.0);
    assert!(inner.0.contains("shard-cache"), "{}", inner.0);
}

#[test]
fn a_record_naming_a_different_interface_is_refused() {
    let text = GHOST_RECORD.replace("shard-store", "shard-cache");
    let left = state(LEFT, "left.state");
    let right = state(RIGHT, "right.state");
    let divergences = record(&text, "record.txt");
    let error = cross_apply(&left, &right, &divergences).expect_err("session refuses");
    let CrosscurrentError::InterfaceMismatch(inner) = &error else {
        panic!("expected an interface mismatch, got {error:?}");
    };
    assert!(inner.0.contains("shard-store"), "{}", inner.0);
    assert!(inner.0.contains("shard-cache"), "{}", inner.0);
}

#[test]
fn the_stage_report_text_is_exact_and_byte_stable() {
    let result = stage(LEFT, RIGHT, GHOST_RECORD);
    let expected = "interface shard-store\n\
                    authors engineer-a engineer-b\n\
                    clean: replicas\n\
                    conflicting: store, timeout\n\
                    one-sided: cache-ttl (left only), fetch (right only)\n\
                    missing: ghost\n";
    assert_eq!(format_stage_report(&result), expected);
    assert_eq!(
        format_stage_report(&stage(LEFT, RIGHT, GHOST_RECORD)),
        expected
    );
}

#[test]
fn an_empty_session_reports_none_on_every_list() {
    let left = "interface shard-store\nauthor a\n";
    let right = "interface shard-store\nauthor b\n";
    let result = stage(left, right, "");
    let expected = "interface shard-store\n\
                    authors a b\n\
                    clean: none\n\
                    conflicting: none\n\
                    one-sided: none\n\
                    missing: none\n";
    assert_eq!(format_stage_report(&result), expected);
}
