//! The setup invariants in the crate's own form (XC-25),
//! ported from the Python corpus file of the same name. The
//! dash scan runs over the bytes of every file the walk
//! reaches rather than a suffix list, which is the stronger
//! form the port states, and it asserts the walk reached the
//! records it exists to cover so it cannot pass vacuously.

mod common;

use std::collections::BTreeSet;

use common::root;
use sha2::{Digest, Sha256};

const CANONICAL_PREFIX_SHA256: &str =
    "63d2f04005dc8eb111d3efca1be55c0911f9ad46bc7cf3c74d9f9908f4a86f11";

const EXPECTED_OBLIGATIONS: [&str; 5] = [
    "XC-S1. Every live branch in a reconciliation session is cross-applied against the shared staging branch, and a branch that was never applied is reported as missing rather than assumed integrated.",
    "XC-S2. A divergence found on the staging branch is handed back to the two authors who caused it as one sized reconciliation task, and a divergence with no task is a defect the tests must catch.",
    "XC-S3. A reconciliation task states the concept that diverged, both conflicting forms, and an estimate of the work to reconcile them, and a task missing any of those is rejected as incomplete.",
    "XC-S4. Crosscurrent reconciles only the pairs it is handed by detection, and it never scans the whole fleet looking for new divergence on its own behalf.",
    "XC-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.",
];

fn read_repo(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("repo file reads")
}

#[test]
fn contract_stub_obligations_exist() {
    let text = read_repo("CONTRACT.md");
    let mut found: Vec<(String, u32, &str)> = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() > 6
            && bytes[0].is_ascii_uppercase()
            && bytes[1].is_ascii_uppercase()
            && bytes[2] == b'-'
            && bytes[3] == b'S'
            && bytes[4].is_ascii_digit()
            && bytes[5] == b'.'
        {
            found.push((line[..2].to_string(), u32::from(bytes[4] - b'0'), line));
        }
    }
    assert!(
        found.len() >= 5,
        "expected at least 5 stub obligations, found {}",
        found.len()
    );
    let texts: Vec<&str> = found.iter().map(|(_, _, line)| *line).collect();
    assert_eq!(
        texts, EXPECTED_OBLIGATIONS,
        "stub obligation text has changed without a contract amendment"
    );
    let mut numbers: Vec<u32> = found.iter().map(|(_, number, _)| *number).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
    for (prefix, _, line) in &found {
        assert_eq!(prefix, "XC", "obligation is not an XC obligation");
        let words = crosscurrent::pytext::split(line).len();
        assert!(words >= 20, "obligation too thin at {words} words");
    }
}

#[test]
fn agents_canonical_block_present() {
    let text = read_repo("AGENTS.md");
    assert!(text.contains("repo annex below this line"));
    let at = text.find("## Repo annex").expect("annex heading stands");
    let prefix = &text[..at];
    let digest = Sha256::digest(prefix.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex, CANONICAL_PREFIX_SHA256,
        "canonical block above the annex line has drifted"
    );
    let annex = &text[at..];
    assert!(annex.contains("CONTRACT.md"));
}

#[test]
fn readme_status_reports_rounds() {
    let text = read_repo("README.md");
    let count_in = |text: &str| {
        let mut counts: Vec<String> = Vec::new();
        let mut search = text;
        while let Some(at) = search.find("Closed rounds ") {
            let rest = &search[at + "Closed rounds ".len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            let after = &rest[digits.len()..];
            if !digits.is_empty()
                && after.starts_with(" of 100")
                && !after[" of 100".len()..].starts_with(|c: char| c.is_ascii_digit())
            {
                counts.push(digits);
            }
            search = &search[at + 1..];
        }
        counts
    };
    assert_eq!(
        count_in(&text),
        ["8"],
        "README status must report exactly one closed round count and it must be 8"
    );
    let status_at = text
        .find("## Status")
        .expect("README carries a Status section");
    let status = &text[status_at..];
    let status = match status[1..].find("\n## ") {
        Some(end) => &status[..end + 1],
        None => status,
    };
    assert_eq!(
        count_in(status),
        ["8"],
        "the closed round count must stand in the Status section itself"
    );
}

const DESCRIPTION: &str = "Receives one forkdrift report and works the divergence, staging records, applying and undoing patches in its own workspace, and building a task for each divergence.";

#[test]
fn the_description_stands_on_every_surface() {
    let readme = read_repo("README.md");
    let lede = readme
        .lines()
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .expect("the README carries a lede line");
    assert_eq!(lede, DESCRIPTION, "the README lede must be the description");
    let manifest = read_repo("Cargo.toml");
    let description_lines: Vec<&str> = manifest
        .lines()
        .filter(|line| line.starts_with("description ="))
        .collect();
    assert_eq!(
        description_lines,
        vec![format!("description = \"{DESCRIPTION}\"").as_str()],
        "the manifest must carry exactly one description field, the description"
    );
    let lib = read_repo("src/lib.rs");
    let doc_line = lib.lines().next().expect("the crate carries doc lines");
    assert_eq!(
        doc_line,
        format!("//! {DESCRIPTION}"),
        "the first crate doc line must be the description"
    );
    let corpus = read_repo("tests/cli_corpus.json");
    assert!(
        corpus.contains(DESCRIPTION),
        "the corpus root help expectations must carry the description"
    );
    let main = read_repo("src/main.rs");
    assert!(
        main.contains(&format!("about = \"{DESCRIPTION}\"")),
        "the command line about must carry the description"
    );
}

/// The dependency names a manifest declares, split into
/// the set the shipped targets build against and the set
/// only the tests build against. A section whose name ends
/// in dev-dependencies feeds the second set and any other
/// section whose name ends in dependencies feeds the first,
/// so a target specific dependency section counts too. A
/// dependency may also stand as a table of its own, under
/// a header naming the dependency after the dependencies
/// segment; the header itself then carries the name and
/// the table's own keys carry none.
fn manifest_dependency_sets(text: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut shipped = BTreeSet::new();
    let mut tests_only = BTreeSet::new();
    let mut section = "";
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') {
            section = line.trim_start_matches('[').trim_end_matches(']').trim();
            let segments: Vec<&str> = section.split('.').map(|part| part.trim()).collect();
            for (index, segment) in segments.iter().enumerate() {
                let dotted = if *segment == "dev-dependencies" {
                    Some(&mut tests_only)
                } else if *segment == "dependencies" {
                    Some(&mut shipped)
                } else {
                    None
                };
                if let Some(target) = dotted
                    && let Some(name) = segments.get(index + 1)
                {
                    let name = name.trim_matches('"');
                    if !name.is_empty() {
                        target.insert(name.to_string());
                    }
                }
            }
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let target = if section == "dev-dependencies" || section.ends_with(".dev-dependencies") {
            &mut tests_only
        } else if section == "dependencies" || section.ends_with(".dependencies") {
            &mut shipped
        } else {
            continue;
        };
        let Some(at) = line.find('=') else { continue };
        let name = line[..at].trim().trim_matches('"');
        if !name.is_empty() {
            target.insert(name.to_string());
        }
    }
    (shipped, tests_only)
}

#[test]
fn manifest_dependency_sets_read_dotted_tables() {
    let text = "[dependencies.serde]\nversion = \"1\"\n\n[dev-dependencies.sha2]\nversion = \"0.10\"\n\n[target.'cfg(unix)'.dependencies.anyhow]\nversion = \"1\"\n";
    let (shipped, tests_only) = manifest_dependency_sets(text);
    let expected_shipped: BTreeSet<String> = ["anyhow", "serde"]
        .iter()
        .map(|name| name.to_string())
        .collect();
    let expected_tests_only: BTreeSet<String> =
        ["sha2"].iter().map(|name| name.to_string()).collect();
    assert_eq!(shipped, expected_shipped);
    assert_eq!(tests_only, expected_tests_only);
}

#[test]
fn manifest_dependency_placement_matches_the_contract() {
    let text = read_repo("Cargo.toml");
    let (shipped, tests_only) = manifest_dependency_sets(&text);
    let expected_shipped: BTreeSet<String> = ["anyhow", "clap", "thiserror"]
        .iter()
        .map(|name| name.to_string())
        .collect();
    let expected_tests_only: BTreeSet<String> = ["serde_json", "sha2"]
        .iter()
        .map(|name| name.to_string())
        .collect();
    assert_eq!(
        shipped, expected_shipped,
        "the shipped dependency set is not the set XC-23 states"
    );
    assert_eq!(
        tests_only, expected_tests_only,
        "the development dependency set is not the set XC-23 states"
    );
}

/// The code of a Rust source with its comments and its
/// string and character literals blanked out, so the import
/// scan below reads code and never prose. A blanked span
/// leaves one space behind, so tokens never join across it.
fn code_only(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        let next = bytes.get(index + 1).copied().unwrap_or(0);
        if byte == b'/' && next == b'/' {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            out.push(b' ');
        } else if byte == b'/' && next == b'*' {
            let mut depth = 1;
            index += 2;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            out.push(b' ');
        } else if byte == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index += 2;
                } else if bytes[index] == b'"' {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
            out.push(b' ');
        } else if byte == b'r'
            && (index == 0
                || !(bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_'))
            && (next == b'"' || next == b'#')
        {
            let mut hashes = 0;
            let mut cursor = index + 1;
            while bytes.get(cursor) == Some(&b'#') {
                hashes += 1;
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'"') {
                cursor += 1;
                while cursor < bytes.len() {
                    if bytes[cursor] == b'"' {
                        let mut matched = true;
                        for step in 0..hashes {
                            if bytes.get(cursor + 1 + step) != Some(&b'#') {
                                matched = false;
                                break;
                            }
                        }
                        if matched {
                            cursor += 1 + hashes;
                            break;
                        }
                    }
                    cursor += 1;
                }
                index = cursor;
                out.push(b' ');
            } else {
                out.push(byte);
                index += 1;
            }
        } else if byte == b'\'' {
            // A character literal is one character or one
            // escape between quotes. Anything longer before
            // the closing quote is a lifetime and stays code.
            let mut cursor = index + 1;
            if bytes.get(cursor) == Some(&b'\\') {
                cursor += 2;
            } else {
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'\'') {
                index = cursor + 1;
                out.push(b' ');
            } else {
                out.push(byte);
                index += 1;
            }
        } else {
            out.push(byte);
            index += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Split a brace group body at its top level commas.
fn split_top_level(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (index, ch) in text.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                parts.push(&text[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect()
}

/// Expand one use tree into the full paths it imports, so a
/// grouped or nested import names every path it reaches.
fn expand_use_tree(tree: &str, prefix: &str, out: &mut Vec<String>) {
    let tree = tree.trim();
    if let Some(open) = tree.find('{') {
        let head = tree[..open].trim().trim_end_matches(':').trim();
        let close = tree.rfind('}').expect("brace group closes");
        let joined = if prefix.is_empty() {
            head.to_string()
        } else if head.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}::{head}")
        };
        for item in split_top_level(&tree[open + 1..close]) {
            expand_use_tree(item, &joined, out);
        }
        return;
    }
    let leaf = tree.trim_end_matches(':').trim();
    let leaf = leaf.split(" as ").next().expect("path stands").trim();
    let full = if prefix.is_empty() {
        leaf.to_string()
    } else if leaf.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}::{leaf}")
    };
    if !full.is_empty() {
        out.push(full);
    }
}

/// Whether code uses a module, read off the code itself.
/// Two kinds of evidence count. An import in any of its
/// forms, plain, grouped, nested, aliased, glob, or the
/// bare module taken whole, fires when any segment of any
/// path it imports names the module. And a qualified path
/// standing in the code with no import above it fires on
/// its own, the module name at a token boundary with the
/// path separator right after it.
fn module_used(source: &str, module: &str) -> bool {
    let code = code_only(source);
    let code: String = code.split_whitespace().collect::<Vec<_>>().join(" ");
    let bytes = code.as_bytes();
    let name = module.as_bytes();
    let mut index = 0;
    while index + name.len() + 2 <= bytes.len() {
        let before_ok =
            index == 0 || !(bytes[index - 1].is_ascii_alphanumeric() || bytes[index - 1] == b'_');
        if before_ok
            && &bytes[index..index + name.len()] == name
            && &bytes[index + name.len()..index + name.len() + 2] == b"::"
        {
            return true;
        }
        index += 1;
    }
    let mut search = code.as_str();
    while let Some(at) = search.find("use") {
        let before_ok = at == 0
            || !(search.as_bytes()[at - 1].is_ascii_alphanumeric()
                || search.as_bytes()[at - 1] == b'_');
        let after = &search[at + 3..];
        let after_ok = after
            .chars()
            .next()
            .is_some_and(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            let Some(end) = after.find(';') else { break };
            let mut paths = Vec::new();
            expand_use_tree(&after[..end], "", &mut paths);
            for path in paths {
                if path.split("::").any(|segment| segment.trim() == module) {
                    return true;
                }
            }
            search = &after[end + 1..];
        } else {
            search = &search[at + 3..];
        }
    }
    false
}

#[test]
fn module_scan_fires_on_every_import_form() {
    let fires = [
        "use serde::Serialize;",
        "use serde::{Deserialize, Serialize};",
        "use serde::de::{self, Visitor};",
        "use serde as sd;",
        "use serde as sd;\nfn f(x: sd::Error) {}",
        "use serde::*;",
        "use serde;",
        "use serde;\nfn f(x: serde::Error) {}",
        "use other::serde;\nfn f(x: serde::Error) {}",
        "fn f(x: serde::de::Error) {}",
    ];
    for source in fires {
        assert!(module_used(source, "serde"), "the scan missed {source}");
    }
    let quiet = [
        "// serde is named in a comment and nowhere else\nfn f() {}",
        "fn f() { let s = \"serde::Serialize\"; }",
        "use serde_json::Value;",
        "use std::collections::HashMap;",
        "fn f(x: myserde::Error) {}",
    ];
    for source in quiet {
        assert!(!module_used(source, "serde"), "the scan fired on {source}");
    }
    assert!(module_used("use anyhow::Context;", "anyhow"));
    assert!(!module_used("use anyhow::Context;", "serde"));
    assert!(module_used(
        "use std::collections::HashMap;\nfn f() -> anyhow::Result<()> { Ok(()) }",
        "anyhow"
    ));
}

#[test]
fn shipped_sources_carry_no_serde_and_no_library_anyhow() {
    let base = root();
    let src = base.join("src");
    let mut files = Vec::new();
    collect_files(&src, &mut files);
    let rel_names: Vec<String> = files
        .iter()
        .map(|path| {
            path.strip_prefix(&base)
                .expect("under root")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for anchor in ["src/lib.rs", "src/main.rs", "src/records.rs"] {
        assert!(
            rel_names.iter().any(|name| name == anchor),
            "the scan never reached {anchor}"
        );
    }
    for path in &files {
        let text = std::fs::read_to_string(path).expect("source reads");
        assert!(
            !module_used(&text, "serde"),
            "serde stands used in {}",
            path.display()
        );
        if path.file_name().expect("name stands") != "main.rs" {
            assert!(
                !module_used(&text, "anyhow"),
                "anyhow stands used outside the binary entry point in {}",
                path.display()
            );
        }
    }
}

const ANY_DEPTH_SKIP: [&str; 6] = [
    ".git",
    "target",
    ".pytest_cache",
    "__pycache__",
    ".venv",
    "venv",
];
const ROOT_SKIP: [&str; 2] = ["node_modules", ".tox"];

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("directory readable") {
        let path = entry.expect("entry readable").path();
        let name = path
            .file_name()
            .expect("name stands")
            .to_string_lossy()
            .into_owned();
        if ANY_DEPTH_SKIP.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

#[test]
fn no_em_or_en_dash_in_text_files() {
    let base = root();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&base).expect("root readable") {
        let path = entry.expect("entry readable").path();
        let name = path
            .file_name()
            .expect("name stands")
            .to_string_lossy()
            .into_owned();
        if ROOT_SKIP.contains(&name.as_str()) || ANY_DEPTH_SKIP.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, &mut files);
        } else if path.is_file() {
            files.push(path);
        }
    }
    let anchors = [
        "CONTRACT.md",
        "README.md",
        "AGENTS.md",
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "examples/divergences.txt",
        "tests/cli_corpus.json",
    ];
    let rel_names: Vec<String> = files
        .iter()
        .map(|path| {
            path.strip_prefix(&base)
                .expect("under root")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for anchor in anchors {
        assert!(
            rel_names.iter().any(|name| name == anchor),
            "the scan never reached {anchor}"
        );
    }
    let em_dash: [u8; 3] = [0xE2, 0x80, 0x94];
    let en_dash: [u8; 3] = [0xE2, 0x80, 0x93];
    let mut bad = Vec::new();
    for path in &files {
        let bytes = std::fs::read(path).expect("file readable");
        if bytes.windows(3).any(|w| w == em_dash || w == en_dash) {
            bad.push(
                path.strip_prefix(&base)
                    .expect("under root")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    assert!(bad.is_empty(), "em or en dash found in {bad:?}");
}
