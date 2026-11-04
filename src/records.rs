//! Parsing for recorded divergence records.
//!
//! A divergence record is the report forkdrift's comparison
//! prints, recorded to a file. Each block names one
//! interface, one diverged name, and one divergence kind,
//! and quotes the defining lines with the source each line
//! came from. A presence block also quotes the interface
//! line of the side that defines nothing for the name, and
//! that row is not a definition. The bundle helpers after
//! the parser state what several records over the one pair
//! of sources mean for a single session, under XC-19 and
//! XC-20.

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::errors::{CrosscurrentError, RecordError};
use crate::pytext;
use crate::states::{StatementForm, is_identifier, parse_statement_form};

/// The term-value divergence kind.
pub const TERM_VALUE: &str = "term-value";
/// The result-type divergence kind.
pub const RESULT_TYPE: &str = "result-type";
/// The excluded-but-defined divergence kind.
pub const EXCLUDED_BUT_DEFINED: &str = "excluded-but-defined";
/// The presence divergence kind.
pub const PRESENCE: &str = "presence";
/// The divergence kinds the format knows.
pub const KINDS: [&str; 4] = [TERM_VALUE, RESULT_TYPE, EXCLUDED_BUT_DEFINED, PRESENCE];

/// One quoted row of a divergence block, the side it came
/// from, the source it names, and the line it quotes with
/// its whitespace stripped.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuotedLine {
    /// The side the row belongs to, left or right.
    pub side: String,
    /// The source the row names for its side.
    pub source: String,
    /// The quoted line, stripped.
    pub line: String,
}

/// One parsed divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// The interface the block names.
    pub interface: String,
    /// The diverged name the block names.
    pub name: String,
    /// The divergence kind the block names.
    pub kind: String,
    /// Every quoted row of the block, in record order.
    pub rows: Vec<QuotedLine>,
    /// The excludes row of an excluded-but-defined block.
    pub excludes_row: Option<QuotedLine>,
    /// The interface row of a presence block.
    pub interface_row: Option<QuotedLine>,
}

impl Divergence {
    /// The defining lines among the quoted rows, derived
    /// never stored. A row is a definition unless it is the
    /// block's excludes row or its interface row, compared
    /// by value as the Python dataclass compares them.
    pub fn definitions(&self) -> Vec<&QuotedLine> {
        self.rows
            .iter()
            .filter(|row| {
                self.excludes_row.as_ref() != Some(*row)
                    && self.interface_row.as_ref() != Some(*row)
            })
            .collect()
    }
}

/// A block while it is being read, its header fields and
/// the rows collected under it.
struct PendingBlock {
    interface: String,
    name: String,
    kind: String,
    rows: Vec<QuotedLine>,
    lineno: usize,
}

/// Match one raw line against the row form, two spaces, a
/// side word, the source in parentheses, and the quoted
/// line. The source is the shortest text the form admits,
/// so it ends at the first closing parenthesis and space
/// pair the rest of the form can complete behind, exactly
/// as the oracle's row pattern reads it.
fn match_row(raw: &str) -> Option<(String, String, String)> {
    let rest = if let Some(rest) = raw.strip_prefix("  left (") {
        ("left", rest)
    } else if let Some(rest) = raw.strip_prefix("  right (") {
        ("right", rest)
    } else {
        return None;
    };
    let (side, rest) = rest;
    let bytes = rest.as_bytes();
    let mut index = 0usize;
    let mut source_end: Option<usize> = None;
    while index < bytes.len() {
        if bytes[index] == b')'
            && index + 1 < bytes.len()
            && bytes[index + 1] == b' '
            && index >= 1
            && index + 2 < bytes.len()
        {
            source_end = Some(index);
            break;
        }
        index += 1;
        while index < bytes.len() && (bytes[index] & 0xC0) == 0x80 {
            index += 1;
        }
    }
    let end = source_end?;
    Some((
        side.to_string(),
        rest[..end].to_string(),
        rest[end + 2..].to_string(),
    ))
}

/// Parse one recorded divergence record or raise a refusal.
pub fn parse_divergence_record(
    text: &str,
    source: &str,
) -> Result<Vec<Divergence>, CrosscurrentError> {
    let mut blocks: Vec<Divergence> = Vec::new();
    let mut pending: Option<PendingBlock> = None;
    let mut side_sources: HashMap<String, String> = HashMap::new();
    for (index, raw) in pytext::splitlines(text).into_iter().enumerate() {
        let lineno = index + 1;
        if pytext::strip(raw).is_empty() {
            continue;
        }
        if raw.starts_with("divergence ") {
            if let Some(pending_block) = pending.take() {
                let block = finish_block(&pending_block, source)?;
                check_row_sources(&block, &pending_block, &mut side_sources, source)?;
                blocks.push(block);
            }
            pending = Some(open_block(raw, source, lineno)?);
            continue;
        }
        if let (Some(row), Some(pending_block)) = (match_row(raw), pending.as_mut()) {
            let (side, row_source, line) = row;
            pending_block.rows.push(QuotedLine {
                side,
                source: row_source,
                line: pytext::strip(&line).to_string(),
            });
            continue;
        }
        return Err(RecordError::new(format!(
            "{source} is malformed, line {lineno} matches no form \
             in a divergence record, {}",
            pytext::strip(raw)
        ))
        .into());
    }
    if let Some(pending_block) = pending.take() {
        let block = finish_block(&pending_block, source)?;
        check_row_sources(&block, &pending_block, &mut side_sources, source)?;
        blocks.push(block);
    }
    let interfaces: HashSet<&str> = blocks
        .iter()
        .map(|block| block.interface.as_str())
        .collect();
    if interfaces.len() > 1 {
        let mut sorted: Vec<&str> = interfaces.into_iter().collect();
        sorted.sort_unstable();
        return Err(RecordError::new(format!(
            "{source} is malformed, blocks name two different \
             interfaces, {} and {}",
            sorted[0], sorted[1]
        ))
        .into());
    }
    let mut seen: HashSet<(&str, &str)> = HashSet::new();
    for block in &blocks {
        let key = (block.name.as_str(), block.kind.as_str());
        if !seen.insert(key) {
            return Err(RecordError::new(format!(
                "{source} is malformed, two blocks for name \
                 {} and kind {}",
                block.name, block.kind
            ))
            .into());
        }
    }
    Ok(blocks)
}

/// The left and right sources the rows of one record
/// establish.
///
/// Under XC-18 every row of a side names the one source of
/// that side, so the first row of each side speaks for the
/// record. A record carrying no divergence establishes no
/// sources and returns None. A side no row names stays
/// unset, as it does in the Python implementation, whose
/// bundle refusal prints an unset side as None.
pub fn record_sources(divergences: &[Divergence]) -> Option<(Option<String>, Option<String>)> {
    let mut left: Option<String> = None;
    let mut right: Option<String> = None;
    for divergence in divergences {
        for row in &divergence.rows {
            if row.side == "left" && left.is_none() {
                left = Some(row.source.clone());
            } else if row.side == "right" && right.is_none() {
                right = Some(row.source.clone());
            }
        }
    }
    if left.is_none() && right.is_none() {
        return None;
    }
    Some((left, right))
}

/// The text of one side of an established source pair, with
/// an unset side printed as the Python implementation
/// prints it.
fn source_text(side: &Option<String>) -> String {
    side.clone().unwrap_or_else(|| "None".to_string())
}

/// The two side sources one record names, either of which
/// the record may leave unnamed.
type SideSources = (Option<String>, Option<String>);

/// Union the divergences of a bundle of records under
/// XC-19 and XC-20.
///
/// The argument pairs each record name with the divergences
/// parsed from that record. Every record that carries a
/// divergence must establish the same left source and the
/// same right source, and a record that establishes
/// different sources refuses the bundle. An exact duplicate
/// of a divergence, identified by its interface and its
/// name and its kind and its quoted rows, enters the union
/// once.
pub fn bundle_divergences(
    records: &[(String, Vec<Divergence>)],
) -> Result<Vec<Divergence>, CrosscurrentError> {
    let mut established: Option<(SideSources, &str)> = None;
    let mut union: Vec<Divergence> = Vec::new();
    let mut seen: HashSet<(String, String, String, Vec<QuotedLine>)> = HashSet::new();
    for (name, divergences) in records {
        if let Some(sources) = record_sources(divergences) {
            match &established {
                None => established = Some((sources, name)),
                Some((known, known_name)) => {
                    if &sources != known {
                        return Err(RecordError::new(format!(
                            "{name} establishes left {} and \
                             right {} and {known_name} \
                             establishes left {} and right \
                             {}, and the records of one bundle \
                             must establish the one pair of sources",
                            source_text(&sources.0),
                            source_text(&sources.1),
                            source_text(&known.0),
                            source_text(&known.1)
                        ))
                        .into());
                    }
                }
            }
        }
        for divergence in divergences {
            let key = (
                divergence.interface.clone(),
                divergence.name.clone(),
                divergence.kind.clone(),
                divergence.rows.clone(),
            );
            if seen.insert(key) {
                union.push(divergence.clone());
            }
        }
    }
    Ok(union)
}

/// Hold a finished block to the record's row sources under
/// XC-18.
///
/// The record's first block establishes the source of each
/// side, and every later row names the source of its side.
/// The block's own shape checks have already run, so this
/// pass sees only rows the Round 1 and Round 2 grammar
/// admitted.
fn check_row_sources(
    block: &Divergence,
    pending: &PendingBlock,
    side_sources: &mut HashMap<String, String>,
    source: &str,
) -> Result<(), CrosscurrentError> {
    let where_ = format!(
        "block for {} of interface {} at line {}",
        block.name, block.interface, pending.lineno
    );
    for row in &block.rows {
        match side_sources.get(&row.side) {
            None => {
                side_sources.insert(row.side.clone(), row.source.clone());
            }
            Some(known) => {
                if *known != row.source {
                    return Err(RecordError::new(format!(
                        "{source} is malformed, {where_} names source \
                         {} on the {}, which is not the \
                         {} source of the record, {known}",
                        row.source, row.side, row.side
                    ))
                    .into());
                }
            }
        }
    }
    if let Some(left) = side_sources.get("left")
        && side_sources.get("right") == Some(left)
    {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} names the one source \
             {left} on both sides"
        ))
        .into());
    }
    Ok(())
}

/// Open one block from its header line, or refuse the
/// header.
fn open_block(raw: &str, source: &str, lineno: usize) -> Result<PendingBlock, CrosscurrentError> {
    let parts = pytext::split(raw);
    if parts.len() != 4 || !is_identifier(parts[1]) || !is_identifier(parts[2]) {
        return Err(RecordError::new(format!(
            "{source} is malformed, block header at line {lineno} \
             matches no form, {}",
            pytext::strip(raw)
        ))
        .into());
    }
    if !KINDS.contains(&parts[3]) {
        return Err(RecordError::new(format!(
            "{source} is malformed, block header at line {lineno} \
             names a divergence kind this format does not know, \
             {}",
            parts[3]
        ))
        .into());
    }
    Ok(PendingBlock {
        interface: parts[1].to_string(),
        name: parts[2].to_string(),
        kind: parts[3].to_string(),
        rows: Vec::new(),
        lineno,
    })
}

/// Finish one block, checking its rows against the shape
/// its kind requires.
fn finish_block(pending: &PendingBlock, source: &str) -> Result<Divergence, CrosscurrentError> {
    let interface = &pending.interface;
    let name = &pending.name;
    let kind = &pending.kind;
    let rows = &pending.rows;
    let where_ = format!("block for {name} at line {}", pending.lineno);
    if kind == TERM_VALUE || kind == RESULT_TYPE {
        let want = if kind == TERM_VALUE {
            "term"
        } else {
            "operation"
        };
        let sides: Vec<&str> = rows.iter().map(|row| row.side.as_str()).collect();
        if sides != ["left", "right"] {
            return Err(RecordError::new(format!(
                "{source} is malformed, {where_} must quote one \
                 left line and one right line"
            ))
            .into());
        }
        for row in rows {
            let form = parse_statement_form(&row.line);
            let matches =
                matches!(&form, Some(f) if f.kind() == want && f.name() == Some(name.as_str()));
            if !matches {
                return Err(RecordError::new(format!(
                    "{source} is malformed, {where_} quotes a line \
                     that is not a {want} line for {name}, {}",
                    row.line
                ))
                .into());
            }
        }
        return Ok(Divergence {
            interface: interface.clone(),
            name: name.clone(),
            kind: kind.clone(),
            rows: rows.clone(),
            excludes_row: None,
            interface_row: None,
        });
    }
    if kind == PRESENCE {
        return finish_presence_block(pending, source);
    }
    let mut excludes_rows: Vec<&QuotedLine> = Vec::new();
    let mut define_rows: Vec<&QuotedLine> = Vec::new();
    for row in rows {
        let form = parse_statement_form(&row.line);
        match &form {
            Some(StatementForm::Excludes(names)) if names.iter().any(|n| n == name) => {
                excludes_rows.push(row);
            }
            Some(f)
                if (f.kind() == "term" || f.kind() == "operation")
                    && f.name() == Some(name.as_str()) =>
            {
                define_rows.push(row);
            }
            _ => {
                return Err(RecordError::new(format!(
                    "{source} is malformed, {where_} quotes a line that \
                     is neither an excludes line naming {name} nor a \
                     defining line for {name}, {}",
                    row.line
                ))
                .into());
            }
        }
    }
    if excludes_rows.len() != 1 {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} must quote exactly one \
             excludes line naming {name}"
        ))
        .into());
    }
    if define_rows.is_empty() || define_rows.len() > 2 {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} must quote one or two \
             defining lines for {name}"
        ))
        .into());
    }
    let define_kinds: Vec<&str> = define_rows
        .iter()
        .map(|row| {
            parse_statement_form(&row.line)
                .expect("a defining row parses")
                .kind()
        })
        .collect();
    if define_kinds.iter().collect::<HashSet<_>>().len() != define_kinds.len() {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} quotes two defining \
             lines of the one form for {name}"
        ))
        .into());
    }
    if define_rows
        .iter()
        .map(|row| &row.source)
        .collect::<HashSet<_>>()
        .len()
        != 1
    {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} quotes defining lines \
             for {name} from two different sources"
        ))
        .into());
    }
    if define_rows[0].source == excludes_rows[0].source {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} quotes the excludes \
             line and the defining lines for {name} from the one \
             source, {}",
            define_rows[0].source
        ))
        .into());
    }
    Ok(Divergence {
        interface: interface.clone(),
        name: name.clone(),
        kind: kind.clone(),
        rows: rows.clone(),
        excludes_row: Some(excludes_rows[0].clone()),
        interface_row: None,
    })
}

/// Finish one presence block, checking its rows against
/// the shape XC-15 states.
fn finish_presence_block(
    pending: &PendingBlock,
    source: &str,
) -> Result<Divergence, CrosscurrentError> {
    let interface = &pending.interface;
    let name = &pending.name;
    let rows = &pending.rows;
    let where_ = format!("block for {name} at line {}", pending.lineno);
    if rows.len() != 2 && rows.len() != 3 {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} must quote one left \
             row and one right row and at most one further row"
        ))
        .into());
    }
    let first_sides: Vec<&str> = rows[..2].iter().map(|row| row.side.as_str()).collect();
    if first_sides != ["left", "right"] {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} must quote one \
             left line and one right line"
        ))
        .into());
    }
    let interface_line = format!("interface {interface}");
    let mut side_sources: BTreeMap<&str, &str> = BTreeMap::new();
    side_sources.insert(rows[0].side.as_str(), rows[0].source.as_str());
    side_sources.insert(rows[1].side.as_str(), rows[1].source.as_str());
    let mut defining: BTreeMap<&str, Vec<&str>> =
        BTreeMap::from([("left", Vec::new()), ("right", Vec::new())]);
    let mut interface_row: Option<QuotedLine> = None;
    for (position, row) in rows.iter().enumerate() {
        if position == 2 && side_sources[row.side.as_str()] != row.source.as_str() {
            return Err(RecordError::new(format!(
                "{source} is malformed, {where_} labels its further \
                 row {} but names source {}, \
                 which is not the {} source",
                row.side, row.source, row.side
            ))
            .into());
        }
        if row.line == interface_line {
            if position == 2 {
                return Err(RecordError::new(format!(
                    "{source} is malformed, {where_} quotes the \
                     interface line as its further row, and a \
                     further row quotes a defining line for {name}"
                ))
                .into());
            }
            interface_row = Some(row.clone());
            continue;
        }
        let form = parse_statement_form(&row.line);
        let matches = matches!(&form, Some(f) if (f.kind() == "term" || f.kind() == "operation") && f.name() == Some(name.as_str()));
        if !matches {
            return Err(RecordError::new(format!(
                "{source} is malformed, {where_} quotes a line \
                 that is neither a defining line for {name} nor \
                 the interface line, {}",
                row.line
            ))
            .into());
        }
        let form = form.expect("a defining row parses");
        defining
            .get_mut(row.side.as_str())
            .expect("a side is present")
            .push(form.kind());
    }
    for side in ["left", "right"] {
        let forms = &defining[side];
        let side_row = if side == "left" { &rows[0] } else { &rows[1] };
        if side_row.line == interface_line && !forms.is_empty() {
            return Err(RecordError::new(format!(
                "{source} is malformed, {where_} quotes the \
                 interface line on the {side} and a defining \
                 line for {name} from the {side} as well"
            ))
            .into());
        }
        if forms.len() == 2 && forms.as_slice() != ["term", "operation"] {
            return Err(RecordError::new(format!(
                "{source} is malformed, {where_} quotes defining \
                 lines for {name} from the {side} that are not a \
                 term line then an operation line"
            ))
            .into());
        }
    }
    let shared: Vec<&str> = defining["left"]
        .iter()
        .filter(|kind| defining["right"].contains(kind))
        .copied()
        .collect();
    if !shared.is_empty() {
        let mut sorted = shared.clone();
        sorted.sort_unstable();
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} quotes a \
             {} line for {name} from both sides, \
             and a presence block quotes each form from one \
             side only",
            sorted[0]
        ))
        .into());
    }
    if defining["left"].is_empty() && defining["right"].is_empty() {
        return Err(RecordError::new(format!(
            "{source} is malformed, {where_} quotes no defining \
             line for {name}"
        ))
        .into());
    }
    Ok(Divergence {
        interface: interface.clone(),
        name: name.clone(),
        kind: PRESENCE.to_string(),
        rows: rows.clone(),
        excludes_row: None,
        interface_row,
    })
}
