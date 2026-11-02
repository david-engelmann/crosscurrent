"""crosscurrent reconciles detected divergence between branch states."""

from .errors import (
    CrosscurrentError,
    InterfaceMismatchError,
    RecordError,
    StateError,
    TaskError,
)
from .records import (
    Divergence,
    QuotedLine,
    bundle_divergences,
    parse_divergence_record,
    record_sources,
)
from .staging import CrossApply, cross_apply, format_stage_report
from .states import BranchState, Statement, parse_branch_state
from .tasks import Task, build_tasks, build_tasks_bundle, format_tasks

__all__ = [
    "BranchState",
    "CrossApply",
    "CrosscurrentError",
    "Divergence",
    "InterfaceMismatchError",
    "QuotedLine",
    "RecordError",
    "StateError",
    "Statement",
    "Task",
    "TaskError",
    "build_tasks",
    "build_tasks_bundle",
    "bundle_divergences",
    "cross_apply",
    "format_stage_report",
    "format_tasks",
    "parse_branch_state",
    "parse_divergence_record",
    "record_sources",
]
