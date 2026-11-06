//! Sized reconciliation tasks handed back to the two
//! authors.

use crate::errors::{CrosscurrentError, TaskError};
use crate::records::{Divergence, RESULT_TYPE, bundle_divergences, printed_row};
use crate::staging::{CrossApply, check_session_interface};
use crate::states::{BranchState, parse_statement_form};

/// One sized reconciliation task of a handback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    /// The interface of the session.
    pub interface: String,
    /// The diverged name the task reconciles.
    pub name: String,
    /// The divergence kind of the task.
    pub kind: String,
    /// The author role of the left state.
    pub left_author: String,
    /// The author role of the right state.
    pub right_author: String,
    /// Every quoted row of the divergence, in record order from
    /// one record and in the canonical printed form order from a
    /// bundle.
    pub rows: Vec<crate::records::QuotedLine>,
    /// The number of quoted defining lines.
    pub definitions: usize,
    /// Whether a result type is among the conflicting forms.
    pub result_type: bool,
    /// Whether either state marks the name breaking.
    pub breaking: bool,
    /// The score the sizing rule gives the task.
    pub score: usize,
    /// The size band of the task.
    pub band: String,
    /// The clean count of the session staging summary.
    pub clean_count: usize,
    /// The conflicting count of the session staging summary.
    pub conflicting_count: usize,
    /// The one-sided count of the session staging summary.
    pub one_sided_count: usize,
    /// The missing count of the session staging summary.
    pub missing_count: usize,
    /// The disagreement note of XC-14, when it applies.
    pub note: Option<String>,
}

/// The size band of a score under the sizing rule.
fn band(score: usize) -> &'static str {
    if score <= 2 {
        "small"
    } else if score <= 4 {
        "medium"
    } else {
        "large"
    }
}

/// The quoted rows of a divergence in the form the handback
/// prints them, the one printed form of records::printed_row
/// that the canonical order of XC-34 also reads.
fn printed_rows(divergence: &Divergence) -> Vec<String> {
    divergence.rows.iter().map(printed_row).collect()
}

/// Build one sized task per divergence, in handback order.
pub fn build_tasks(
    left: &BranchState,
    right: &BranchState,
    divergences: &[Divergence],
    staging: &CrossApply,
) -> Result<Vec<Task>, CrosscurrentError> {
    check_session_interface(left, right, divergences)?;
    if staging.interface != left.interface
        || staging.left_author != left.author
        || staging.right_author != right.author
    {
        return Err(TaskError::new(
            "a reconciliation task cannot be stated, the staging \
             summary belongs to a different session"
                .to_string(),
        )
        .into());
    }
    let mut ordered: Vec<&Divergence> = divergences.iter().collect();
    ordered.sort_by(|a, b| {
        (
            a.interface.as_str(),
            a.name.as_str(),
            a.kind.as_str(),
            printed_rows(a),
        )
            .cmp(&(
                b.interface.as_str(),
                b.name.as_str(),
                b.kind.as_str(),
                printed_rows(b),
            ))
    });
    ordered
        .into_iter()
        .map(|divergence| build_task(left, right, divergence, staging))
        .collect()
}

/// Build the tasks of a bundle handback under XC-19 to
/// XC-21, with the set identity of XC-33 and the canonical
/// row order of XC-34 the union applies.
///
/// The records argument pairs each record name with the
/// divergences parsed from that record. The tasks are built
/// from the union of the bundle exactly as build_tasks
/// builds them from one record, so a bundle of one record
/// states the same tasks as that record on its own, with
/// its rows in the canonical order where they stood out of
/// it.
pub fn build_tasks_bundle(
    left: &BranchState,
    right: &BranchState,
    records: &[(String, Vec<Divergence>)],
    staging: &CrossApply,
) -> Result<Vec<Task>, CrosscurrentError> {
    build_tasks(left, right, &bundle_divergences(records)?, staging)
}

/// Build the one task of one divergence.
fn build_task(
    left: &BranchState,
    right: &BranchState,
    divergence: &Divergence,
    staging: &CrossApply,
) -> Result<Task, CrosscurrentError> {
    if divergence.interface.is_empty() || divergence.name.is_empty() || divergence.kind.is_empty() {
        return Err(TaskError::new(
            "a reconciliation task cannot be stated, the divergence \
             is missing its interface, name, or kind"
                .to_string(),
        )
        .into());
    }
    let definitions = divergence.definitions();
    if divergence.rows.is_empty() || definitions.is_empty() {
        return Err(TaskError::new(format!(
            "a reconciliation task for {} cannot be \
             stated, the divergence quotes no defining line",
            divergence.name
        ))
        .into());
    }
    if divergence.kind == "excluded-but-defined" && divergence.excludes_row.is_none() {
        return Err(TaskError::new(format!(
            "a reconciliation task for {} cannot be \
             stated, the divergence quotes no excludes line",
            divergence.name
        ))
        .into());
    }
    let result_type = divergence.kind == RESULT_TYPE
        || definitions.iter().any(|row| {
            parse_statement_form(&row.line).is_some_and(|form| form.kind() == "operation")
        });
    let breaking =
        left.breaking.contains(&divergence.name) || right.breaking.contains(&divergence.name);
    let definition_count = definitions.len();
    let score =
        definition_count + (if result_type { 2 } else { 0 }) + (if breaking { 3 } else { 0 });
    Ok(Task {
        interface: divergence.interface.clone(),
        name: divergence.name.clone(),
        kind: divergence.kind.clone(),
        left_author: left.author.clone(),
        right_author: right.author.clone(),
        rows: divergence.rows.clone(),
        definitions: definition_count,
        result_type,
        breaking,
        score,
        band: band(score).to_string(),
        clean_count: staging.clean.len(),
        conflicting_count: staging.conflicting.len(),
        one_sided_count: staging.one_sided.len(),
        missing_count: staging.missing.len(),
        note: if staging.clean.contains(&divergence.name) {
            Some(
                "the staging cross-apply classifies this name clean \
                 and this task follows the record"
                    .to_string(),
            )
        } else {
            None
        },
    })
}

/// A flag as the factors line prints it.
fn yes_no(flag: bool) -> &'static str {
    if flag { "yes" } else { "no" }
}

/// Render the handback document as deterministic text.
pub fn format_tasks(tasks: &[Task]) -> String {
    let mut blocks = Vec::new();
    for task in tasks {
        let mut lines = vec![
            format!("task {} {}", task.interface, task.name),
            format!("authors {} {}", task.left_author, task.right_author),
            format!("kind {}", task.kind),
        ];
        for row in &task.rows {
            lines.push(printed_row(row));
        }
        lines.push(format!("size {} score {}", task.band, task.score));
        lines.push(format!(
            "factors definitions {}, result-type {}, breaking {}",
            task.definitions,
            yes_no(task.result_type),
            yes_no(task.breaking)
        ));
        if let Some(note) = &task.note {
            lines.push(format!("note {note}"));
        }
        lines.push(format!(
            "staging clean {}, conflicting {}, one-sided {}, missing {}",
            task.clean_count, task.conflicting_count, task.one_sided_count, task.missing_count
        ));
        blocks.push(lines.join("\n"));
    }
    if blocks.is_empty() {
        return String::new();
    }
    blocks.join("\n") + "\n"
}
