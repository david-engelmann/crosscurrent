"""Error types for crosscurrent."""


class CrosscurrentError(Exception):
    """Base class for every refusal crosscurrent raises."""


class StateError(CrosscurrentError):
    """A recorded branch state is malformed."""


class RecordError(CrosscurrentError):
    """A recorded divergence record is malformed."""


class InterfaceMismatchError(CrosscurrentError):
    """Inputs in one session name different interfaces."""


class TaskError(CrosscurrentError):
    """A reconciliation task cannot be stated in full."""
