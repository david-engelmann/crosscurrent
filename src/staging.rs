//! The staging area model over two recorded branch states.

use std::collections::BTreeSet;

use crate::errors::{CrosscurrentError, InterfaceMismatchError};
use crate::records::Divergence;
use crate::states::{BranchState, values_equal};

/// The cross-apply of two branch states over one session,
/// the classification of every name either state defines
/// or the record reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossApply {
    /// The interface of the session.
    pub interface: String,
    /// The author role of the left state, as handed in.
    pub left_author: String,
    /// The author role of the right state, as handed in.
    pub right_author: String,
    /// The names both states define in the same form.
    pub clean: Vec<String>,
    /// The names both states define in differing form.
    pub conflicting: Vec<String>,
    /// The names exactly one state defines, with the side.
    pub one_sided: Vec<(String, String)>,
    /// The names the record reports that neither state
    /// defines.
    pub missing: Vec<String>,
}

/// Refuse a session whose inputs name different interfaces.
pub fn check_session_interface(
    left: &BranchState,
    right: &BranchState,
    divergences: &[Divergence],
) -> Result<(), CrosscurrentError> {
    if left.interface != right.interface {
        return Err(InterfaceMismatchError::new(format!(
            "branch states name different interfaces, \
             {} names {} and {} names {}",
            left.source, left.interface, right.source, right.interface
        ))
        .into());
    }
    for divergence in divergences {
        if divergence.interface != left.interface {
            return Err(InterfaceMismatchError::new(format!(
                "the divergence record names a different interface, \
                 the branch states name {} and the \
                 record names {}",
                left.interface, divergence.interface
            ))
            .into());
        }
    }
    Ok(())
}

/// The forms one state states for one name, its term value
/// and its operation value where it defines them.
fn side_forms<'a>(state: &'a BranchState, name: &str) -> Vec<(&'static str, &'a str)> {
    let mut forms = Vec::new();
    if let Some(statement) = state.terms.get(name) {
        forms.push(("term", statement.value.as_str()));
    }
    if let Some(statement) = state.operations.get(name) {
        forms.push(("operation", statement.value.as_str()));
    }
    forms
}

/// Classify every defined name between the two branch
/// states.
pub fn cross_apply(
    left: &BranchState,
    right: &BranchState,
    divergences: &[Divergence],
) -> Result<CrossApply, CrosscurrentError> {
    check_session_interface(left, right, divergences)?;
    let mut names: BTreeSet<&str> = BTreeSet::new();
    names.extend(left.terms.keys().map(String::as_str));
    names.extend(left.operations.keys().map(String::as_str));
    names.extend(right.terms.keys().map(String::as_str));
    names.extend(right.operations.keys().map(String::as_str));
    let mut clean = Vec::new();
    let mut conflicting = Vec::new();
    let mut one_sided = Vec::new();
    for name in &names {
        let left_forms = side_forms(left, name);
        let right_forms = side_forms(right, name);
        if !left_forms.is_empty() && !right_forms.is_empty() {
            let left_kinds: Vec<&str> = left_forms.iter().map(|(kind, _)| *kind).collect();
            let right_kinds: Vec<&str> = right_forms.iter().map(|(kind, _)| *kind).collect();
            let agrees = left_kinds == right_kinds
                && left_forms.iter().all(|(kind, value)| {
                    let right_value = right_forms
                        .iter()
                        .find(|(right_kind, _)| right_kind == kind)
                        .map(|(_, right_value)| *right_value);
                    right_value.is_some_and(|right_value| values_equal(value, right_value))
                });
            if agrees {
                clean.push((*name).to_string());
            } else {
                conflicting.push((*name).to_string());
            }
        } else if !left_forms.is_empty() {
            one_sided.push(((*name).to_string(), "left".to_string()));
        } else {
            one_sided.push(((*name).to_string(), "right".to_string()));
        }
    }
    let reported: BTreeSet<&str> = divergences.iter().map(|d| d.name.as_str()).collect();
    let missing: Vec<String> = reported
        .difference(&names)
        .map(|name| (*name).to_string())
        .collect();
    Ok(CrossApply {
        interface: left.interface.clone(),
        left_author: left.author.clone(),
        right_author: right.author.clone(),
        clean,
        conflicting,
        one_sided,
        missing,
    })
}

/// A name list as the report prints it, or the word none.
fn name_list(names: &[String]) -> String {
    if names.is_empty() {
        "none".to_string()
    } else {
        names.join(", ")
    }
}

/// Render the cross-apply report as deterministic text.
pub fn format_stage_report(result: &CrossApply) -> String {
    let one_sided = result
        .one_sided
        .iter()
        .map(|(name, side)| format!("{name} ({side} only)"))
        .collect::<Vec<_>>()
        .join(", ");
    let lines = [
        format!("interface {}", result.interface),
        format!("authors {} {}", result.left_author, result.right_author),
        format!("clean: {}", name_list(&result.clean)),
        format!("conflicting: {}", name_list(&result.conflicting)),
        format!(
            "one-sided: {}",
            if one_sided.is_empty() {
                "none".to_string()
            } else {
                one_sided
            }
        ),
        format!("missing: {}", name_list(&result.missing)),
    ];
    lines.join("\n") + "\n"
}
