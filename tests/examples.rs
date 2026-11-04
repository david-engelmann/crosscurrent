//! The ported test_examples corpus, test for test.

mod common;

use common::{record, run_cli_at_root, state};
use crosscurrent::{cross_apply, format_stage_report};

fn example_text(name: &str) -> String {
    let path = common::root().join("examples").join(name);
    std::fs::read_to_string(path).expect("example readable")
}

fn example_bytes(name: &str) -> Vec<u8> {
    let path = common::root().join("examples").join(name);
    std::fs::read(path).expect("example readable")
}

#[test]
fn example_record_carries_exactly_one_divergence() {
    let found = record(&example_text("divergences.txt"), "examples/divergences.txt");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "store");
    assert_eq!(found[0].kind, "result-type");
}

#[test]
fn example_quoted_lines_appear_in_the_branch_states() {
    let left_lines: std::collections::HashSet<String> = example_text("branch-a.state")
        .lines()
        .map(str::to_string)
        .collect();
    let right_lines: std::collections::HashSet<String> = example_text("branch-b.state")
        .lines()
        .map(str::to_string)
        .collect();
    let found = record(&example_text("divergences.txt"), "examples/divergences.txt");
    for row in found[0].definitions() {
        assert!(
            left_lines.contains(&row.line) || right_lines.contains(&row.line),
            "quoted line {:?} stands in neither branch state",
            row.line
        );
    }
}

#[test]
fn example_staging_report_is_exact() {
    let left = state(&example_text("branch-a.state"), "examples/branch-a.state");
    let right = state(&example_text("branch-b.state"), "examples/branch-b.state");
    let found = record(&example_text("divergences.txt"), "examples/divergences.txt");
    let expected = "interface shard-store\n\
                    authors engineer-a engineer-b\n\
                    clean: replicas, timeout\n\
                    conflicting: store\n\
                    one-sided: cache-ttl (left only)\n\
                    missing: none\n";
    let staging = cross_apply(&left, &right, &found).expect("staging runs");
    assert_eq!(format_stage_report(&staging), expected);
}

#[test]
fn committed_example_output_matches_regeneration() {
    let result = run_cli_at_root(&[
        "handback",
        "examples/branch-a.state",
        "examples/branch-b.state",
        "examples/divergences.txt",
    ]);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, example_bytes("handback.txt"));
    let second = run_cli_at_root(&[
        "handback",
        "examples/branch-a.state",
        "examples/branch-b.state",
        "examples/divergences.txt",
    ]);
    assert_eq!(second.stdout, result.stdout);
}

#[test]
fn presence_example_record_carries_all_four_kinds() {
    let found = record(
        &example_text("presence-divergences.txt"),
        "examples/presence-divergences.txt",
    );
    let pairs: Vec<(&str, &str)> = found
        .iter()
        .map(|block| (block.name.as_str(), block.kind.as_str()))
        .collect();
    assert_eq!(
        pairs,
        [
            ("cache-ttl", "presence"),
            ("legacy-mode", "excluded-but-defined"),
            ("purge", "presence"),
            ("store", "result-type"),
            ("timeout", "term-value"),
        ]
    );
}

#[test]
fn presence_example_quoted_lines_appear_in_their_named_sources() {
    let found = record(
        &example_text("presence-divergences.txt"),
        "examples/presence-divergences.txt",
    );
    for block in &found {
        for row in &block.rows {
            let path = common::root().join(&row.source);
            let text = std::fs::read_to_string(path).expect("named source readable");
            let lines: std::collections::HashSet<String> =
                text.lines().map(str::to_string).collect();
            assert!(
                lines.contains(&row.line),
                "quoted line {:?} stands in {}",
                row.line,
                row.source
            );
        }
    }
}

#[test]
fn presence_example_staging_report_is_exact() {
    let left = state(
        &example_text("presence-branch-a.state"),
        "examples/presence-branch-a.state",
    );
    let right = state(
        &example_text("presence-branch-b.state"),
        "examples/presence-branch-b.state",
    );
    let found = record(
        &example_text("presence-divergences.txt"),
        "examples/presence-divergences.txt",
    );
    let expected = "interface shard-store\n\
                    authors engineer-a engineer-b\n\
                    clean: replicas, timeout\n\
                    conflicting: store\n\
                    one-sided: cache-ttl (left only), legacy-mode (right only)\n\
                    missing: purge\n";
    let staging = cross_apply(&left, &right, &found).expect("staging runs");
    assert_eq!(format_stage_report(&staging), expected);
}

#[test]
fn committed_presence_example_output_matches_regeneration() {
    let result = run_cli_at_root(&[
        "handback",
        "examples/presence-branch-a.state",
        "examples/presence-branch-b.state",
        "examples/presence-divergences.txt",
    ]);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, example_bytes("presence-handback.txt"));
    let second = run_cli_at_root(&[
        "handback",
        "examples/presence-branch-a.state",
        "examples/presence-branch-b.state",
        "examples/presence-divergences.txt",
    ]);
    assert_eq!(second.stdout, result.stdout);
}
