//! Receives one forkdrift report and works the divergence, staging records, applying and undoing patches in its own workspace, and building a task for each divergence.
//!
//! Detection hands crosscurrent two recorded branch states
//! and the divergence records a detector printed over one
//! pair of sources. Crosscurrent cross-applies the two
//! states in a staging area and hands every divergence in
//! the record back to the two authors as one sized
//! reconciliation task. It reconciles only the pairs it is
//! handed. Detecting divergence across the fleet is
//! forkdrift's work, and crosscurrent never takes it over.

pub mod errors;
pub mod pynumber;
pub mod pytext;
pub mod records;
pub mod staging;
pub mod states;
pub mod tasks;

pub use errors::{CrosscurrentError, InterfaceMismatchError, RecordError, StateError, TaskError};
pub use records::{Divergence, QuotedLine, bundle_divergences, parse_divergence_record};
pub use staging::{CrossApply, cross_apply, format_stage_report};
pub use states::{BranchState, Statement, parse_branch_state, values_equal};
pub use tasks::{Task, build_tasks, build_tasks_bundle, format_tasks};
