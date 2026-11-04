//! Parsing for recorded branch states.
//!
//! A branch state is the recorded text one author's branch
//! left behind for one interface, written in the line based
//! format CONTRACT.md defines. The statement forms are the
//! forms forkdrift defines for its recorded artifacts,
//! extended with an author line and breaking lines.

use std::collections::{BTreeMap, BTreeSet};

use crate::errors::{CrosscurrentError, StateError};
use crate::pynumber::{ParsedNumber, parse_number};
use crate::pytext;

/// One recorded defining line of a branch state, a term, an
/// operation, or a label, with the value it states in
/// normalized form and the meaningful line it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Statement {
    /// The name the line defines.
    pub name: String,
    /// The value the line states, normalized under XC-5.
    pub value: String,
    /// The meaningful line itself, stripped.
    pub line: String,
}

/// One parsed branch state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchState {
    /// The name the state was handed in under.
    pub source: String,
    /// The interface the state names.
    pub interface: String,
    /// The author role the state names.
    pub author: String,
    /// The term definitions of the state, by name.
    pub terms: BTreeMap<String, Statement>,
    /// The operation definitions of the state, by name.
    pub operations: BTreeMap<String, Statement>,
    /// The label definitions of the state, by name.
    pub labels: BTreeMap<String, Statement>,
    /// The names the state's excludes line forbids.
    pub excludes: BTreeSet<String>,
    /// The excludes line itself, when the state carries one.
    pub excludes_line: Option<String>,
    /// The names the state marks as stated breaking changes.
    pub breaking: BTreeSet<String>,
}

/// Whether a text is an identifier of the format, a letter
/// or an underscore followed by letters, digits,
/// underscores, dashes, or dots, all ASCII.
pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Collapse every run of whitespace in a value to a single
/// space, under the whitespace set of XC-27.
pub fn normalize_value(value: &str) -> String {
    pytext::split(value).join(" ")
}

/// Compare two normalized values under the XC-5
/// normalizations. The values agree when their texts are
/// identical, or when both read as finite numbers of equal
/// worth under the decimal grammar of XC-28.
pub fn values_equal(left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }
    let (ParsedNumber::Finite(left_number), ParsedNumber::Finite(right_number)) =
        (parse_number(left), parse_number(right))
    else {
        return false;
    };
    left_number.equals(&right_number)
}

/// One statement form of a recorded text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementForm {
    /// A label line, its name and its value as written.
    Label(String, String),
    /// An operation line, its name and its value as written.
    Operation(String, String),
    /// A term line, its name and its value as written.
    Term(String, String),
    /// An excludes line, the names it forbids.
    Excludes(Vec<String>),
}

impl StatementForm {
    /// The kind word of the form, as the refusal texts and
    /// the parsers name it.
    pub fn kind(&self) -> &'static str {
        match self {
            StatementForm::Label(..) => "label",
            StatementForm::Operation(..) => "operation",
            StatementForm::Term(..) => "term",
            StatementForm::Excludes(_) => "excludes",
        }
    }

    /// The name the form defines, for the defining forms.
    pub fn name(&self) -> Option<&str> {
        match self {
            StatementForm::Label(name, _)
            | StatementForm::Operation(name, _)
            | StatementForm::Term(name, _) => Some(name),
            StatementForm::Excludes(_) => None,
        }
    }
}

/// Classify one statement line from a recorded text.
///
/// Returns the form the line takes, or None when the line
/// matches no statement form. The label and operation
/// prefixes are tried first, then a line whose part before
/// the equals sign strips to the excludes word takes the
/// excludes branch, then any other line with an equals sign
/// is a term line.
pub fn parse_statement_form(line: &str) -> Option<StatementForm> {
    if let Some(rest) = line.strip_prefix("label ") {
        let (name, value) = split_form(rest)?;
        return Some(StatementForm::Label(name, value));
    }
    if let Some(rest) = line.strip_prefix("operation ") {
        let (name, value) = split_form(rest)?;
        return Some(StatementForm::Operation(name, value));
    }
    if let Some(eq) = line.find('=') {
        if pytext::strip(&line[..eq]) == "excludes" {
            let names: Vec<String> = line[eq + 1..]
                .split(',')
                .map(|part| pytext::strip(part).to_string())
                .collect();
            if names.iter().any(|name| !is_identifier(name)) {
                return None;
            }
            return Some(StatementForm::Excludes(names));
        }
        let (name, value) = split_form(line)?;
        return Some(StatementForm::Term(name, value));
    }
    None
}

/// Split a defining line at its first equals sign, reading
/// the name stripped and the value stripped, where the name
/// must be an identifier and the value must be non-empty.
fn split_form(text: &str) -> Option<(String, String)> {
    let eq = text.find('=')?;
    let name = pytext::strip(&text[..eq]);
    let value = pytext::strip(&text[eq + 1..]);
    if !is_identifier(name) || value.is_empty() {
        return None;
    }
    Some((name.to_string(), value.to_string()))
}

/// Parse one recorded branch state or raise a refusal
/// naming why.
pub fn parse_branch_state(text: &str, source: &str) -> Result<BranchState, CrosscurrentError> {
    let mut meaningful: Vec<(usize, &str)> = Vec::new();
    for (index, raw) in pytext::splitlines(text).into_iter().enumerate() {
        let stripped = pytext::strip(raw);
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        meaningful.push((index + 1, stripped));
    }
    let Some(&(first_lineno, first)) = meaningful.first() else {
        return Err(StateError::new(format!(
            "{source} is malformed, missing interface line, \
             the branch state holds no meaningful line"
        ))
        .into());
    };
    let head = pytext::split(first);
    if head.len() != 2 || head[0] != "interface" || !is_identifier(head[1]) {
        return Err(StateError::new(format!(
            "{source} is malformed, missing interface line, \
             line {first_lineno} does not name an interface"
        ))
        .into());
    }
    let interface = head[1].to_string();
    if meaningful.len() < 2 {
        return Err(StateError::new(format!(
            "{source} is malformed, missing author line, \
             the branch state holds no second meaningful line"
        ))
        .into());
    }
    let (second_lineno, second) = meaningful[1];
    let head = pytext::split(second);
    if head.len() != 2 || head[0] != "author" || !is_identifier(head[1]) {
        return Err(StateError::new(format!(
            "{source} is malformed, missing author line, \
             line {second_lineno} does not name an author role"
        ))
        .into());
    }
    let author = head[1].to_string();
    let mut terms: BTreeMap<String, Statement> = BTreeMap::new();
    let mut operations: BTreeMap<String, Statement> = BTreeMap::new();
    let mut labels: BTreeMap<String, Statement> = BTreeMap::new();
    let mut excludes: BTreeSet<String> = BTreeSet::new();
    let mut excludes_line: Option<String> = None;
    let mut breaking: BTreeSet<String> = BTreeSet::new();
    let mut breaking_lines: BTreeMap<String, usize> = BTreeMap::new();
    for &(lineno, line) in &meaningful[2..] {
        if line.starts_with("author ") {
            return Err(StateError::new(format!(
                "{source} is malformed, author line out of place \
                 at line {lineno}, the author line is the second \
                 meaningful line"
            ))
            .into());
        }
        if line.starts_with("interface ") {
            return Err(StateError::new(format!(
                "{source} is malformed, more than one interface line, \
                 second one at line {lineno}"
            ))
            .into());
        }
        if line.starts_with("breaking ") {
            let parts = pytext::split(line);
            if parts.len() != 2 || !is_identifier(parts[1]) {
                return Err(StateError::new(format!(
                    "{source} is malformed, line {lineno} matches no \
                     statement form, {line}"
                ))
                .into());
            }
            let name = parts[1];
            if breaking.contains(name) {
                return Err(StateError::new(format!(
                    "{source} is malformed, duplicate breaking line \
                     for {name} at line {lineno}"
                ))
                .into());
            }
            breaking.insert(name.to_string());
            breaking_lines.insert(name.to_string(), lineno);
            continue;
        }
        let form = parse_statement_form(line);
        if form.is_none() {
            if let Some(eq) = line.find('=')
                && pytext::strip(&line[eq + 1..]).is_empty()
            {
                return Err(StateError::new(format!(
                    "{source} is malformed, line {lineno} states an \
                     empty value"
                ))
                .into());
            }
            return Err(StateError::new(format!(
                "{source} is malformed, line {lineno} matches no \
                 statement form, {line}"
            ))
            .into());
        }
        match form.expect("a form exists") {
            StatementForm::Excludes(names) => {
                if excludes_line.is_some() {
                    return Err(StateError::new(format!(
                        "{source} is malformed, more than one excludes line, \
                         second one at line {lineno}"
                    ))
                    .into());
                }
                excludes = names.into_iter().collect();
                excludes_line = Some(line.to_string());
            }
            StatementForm::Label(name, value) => {
                store(&mut labels, "label", &name, &value, line, source, lineno)?;
            }
            StatementForm::Operation(name, value) => {
                store(
                    &mut operations,
                    "operation",
                    &name,
                    &value,
                    line,
                    source,
                    lineno,
                )?;
            }
            StatementForm::Term(name, value) => {
                store(&mut terms, "term", &name, &value, line, source, lineno)?;
            }
        }
    }
    for name in &breaking {
        if !terms.contains_key(name) && !operations.contains_key(name) {
            let lineno = breaking_lines[name];
            return Err(StateError::new(format!(
                "{source} is malformed, breaking line at line \
                 {lineno} names {name}, which the state \
                 does not define"
            ))
            .into());
        }
    }
    for name in &excludes {
        if terms.contains_key(name) || operations.contains_key(name) {
            return Err(StateError::new(format!(
                "{source} is malformed, it defines {name} and also \
                 excludes it"
            ))
            .into());
        }
    }
    Ok(BranchState {
        source: source.to_string(),
        interface,
        author,
        terms,
        operations,
        labels,
        excludes,
        excludes_line,
        breaking,
    })
}

/// Store one defining line in its table, refusing a name
/// the table already holds.
fn store(
    table: &mut BTreeMap<String, Statement>,
    kind: &str,
    name: &str,
    value: &str,
    line: &str,
    source: &str,
    lineno: usize,
) -> Result<(), CrosscurrentError> {
    if table.contains_key(name) {
        return Err(StateError::new(format!(
            "{source} is malformed, duplicate {kind} name {name} \
             at line {lineno}"
        ))
        .into());
    }
    table.insert(
        name.to_string(),
        Statement {
            name: name.to_string(),
            value: normalize_value(value),
            line: line.to_string(),
        },
    );
    Ok(())
}
