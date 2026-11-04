//! The command line surface corpus.
//!
//! Every case in cli_corpus.json was classified by
//! construction against the Python command line of the pre
//! port tree and the port's binary together. A case whose
//! exit code, streams, and final files all match the Python
//! tool byte for byte is marked oracle and carries the
//! Python expectation. A case whose exit code and final
//! files match the Python tool but whose stream text
//! differs is marked clap and carries the binary's text,
//! because help text and malformed invocation error text
//! are clap's presentation under XC-32. Nothing else can
//! enter the corpus, since a case that differs in code or
//! files is a functional divergence and stops the
//! regeneration. The split is pinned below by count, so a
//! case cannot change sides without the suite failing. Each
//! case runs the binary in a fresh directory seeded with
//! the case's own files and compares the exit code, both
//! streams, and the full final state of the directory.

mod common;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common::{run_cli, temp_dir};
use serde_json::Value;

const EXPECTED_CASES: usize = 88;
const EXPECTED_ORACLE_CASES: usize = 39;
const EXPECTED_CLAP_CASES: usize = 49;

fn string(value: &Value) -> &str {
    value.as_str().expect("corpus text is a string")
}

fn write_seed(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("seed directory created");
    }
    std::fs::write(&path, text).expect("seed written");
}

fn snapshot(dir: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current).expect("case dir readable") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let rel = path
                    .strip_prefix(dir)
                    .expect("path stands under the case dir")
                    .to_string_lossy()
                    .replace('\\', "/");
                let text = std::fs::read_to_string(&path).expect("case file is UTF-8");
                files.insert(rel, text);
            }
        }
    }
    files
}

fn run_cases(source: &str, expected: usize) {
    let corpus: Value =
        serde_json::from_str(include_str!("cli_corpus.json")).expect("corpus parses");
    let cases = corpus["cases"].as_array().expect("cases present");
    let selected: Vec<&Value> = cases
        .iter()
        .filter(|case| string(&case["source"]) == source)
        .collect();
    assert_eq!(
        selected.len(),
        expected,
        "{source} case count moved, regenerate the corpus and re-pin"
    );
    for case in selected {
        let name = string(&case["name"]);
        let dir = temp_dir("corpus");
        let setup = case["setup"].as_object().expect("setup present");
        for (rel, text) in setup {
            write_seed(&dir, rel, string(text));
        }
        let argv: Vec<&str> = case["argv"]
            .as_array()
            .expect("argv present")
            .iter()
            .map(string)
            .collect();
        let output = run_cli(&argv, &dir);
        let expect = &case["expect"];
        assert_eq!(
            output.code,
            expect["code"].as_i64().expect("code present") as i32,
            "{name} exit code"
        );
        assert_eq!(
            output.stdout,
            string(&expect["stdout"]).as_bytes(),
            "{name} stdout"
        );
        assert_eq!(
            output.stderr,
            string(&expect["stderr"]).as_bytes(),
            "{name} stderr"
        );
        let expected_files: BTreeMap<String, String> = expect["files"]
            .as_object()
            .expect("files present")
            .iter()
            .map(|(rel, text)| (rel.clone(), string(text).to_string()))
            .collect();
        assert_eq!(snapshot(&dir), expected_files, "{name} final files");
    }
}

#[test]
fn corpus_shape_is_pinned() {
    let corpus: Value =
        serde_json::from_str(include_str!("cli_corpus.json")).expect("corpus parses");
    let cases = corpus["cases"].as_array().expect("cases present");
    assert_eq!(cases.len(), EXPECTED_CASES);
    let oracle = cases
        .iter()
        .filter(|case| string(&case["source"]) == "oracle")
        .count();
    let clap = cases
        .iter()
        .filter(|case| string(&case["source"]) == "clap")
        .count();
    assert_eq!(oracle, EXPECTED_ORACLE_CASES);
    assert_eq!(clap, EXPECTED_CLAP_CASES);
}

#[test]
fn oracle_cases_match_the_python_tool_byte_for_byte() {
    run_cases("oracle", EXPECTED_ORACLE_CASES);
}

#[test]
fn clap_cases_match_on_outcome_and_carry_the_parsers_text() {
    run_cases("clap", EXPECTED_CLAP_CASES);
}
