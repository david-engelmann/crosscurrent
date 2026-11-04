//! The refusal family of the tool.
//!
//! Every refusal the parsers, the staging cross-apply, or
//! the task builder raises is a CrosscurrentError, and the
//! command line answers it with one error line and exit
//! status 2. The four members are the four refusal classes
//! of the Python implementation, where each derived from
//! the one base error.

/// The base refusal of the tool, carrying the refusal text.
#[derive(Debug, thiserror::Error)]
pub enum CrosscurrentError {
    /// A refusal the branch state parser raises over a
    /// malformed branch state.
    #[error("{0}")]
    State(#[from] StateError),
    /// A refusal the record parser or the bundle union
    /// raises over a malformed divergence record or bundle.
    #[error("{0}")]
    Record(#[from] RecordError),
    /// A refusal the session raises when its inputs name
    /// different interfaces.
    #[error("{0}")]
    InterfaceMismatch(#[from] InterfaceMismatchError),
    /// A refusal the task builder raises when a task cannot
    /// be stated in full.
    #[error("{0}")]
    Task(#[from] TaskError),
}

/// A refusal over a malformed branch state, carrying its
/// text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct StateError(pub String);

impl StateError {
    pub(crate) fn new(text: String) -> Self {
        StateError(text)
    }
}

/// A refusal over a malformed divergence record, carrying
/// its text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct RecordError(pub String);

impl RecordError {
    pub(crate) fn new(text: String) -> Self {
        RecordError(text)
    }
}

/// A refusal over inputs that name different interfaces,
/// carrying its text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct InterfaceMismatchError(pub String);

impl InterfaceMismatchError {
    pub(crate) fn new(text: String) -> Self {
        InterfaceMismatchError(text)
    }
}

/// A refusal over a task that cannot be stated in full,
/// carrying its text.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TaskError(pub String);

impl TaskError {
    pub(crate) fn new(text: String) -> Self {
        TaskError(text)
    }
}
