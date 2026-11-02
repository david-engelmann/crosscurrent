"""Sized reconciliation tasks handed back to the two authors."""

from dataclasses import dataclass

from .errors import TaskError
from .records import RESULT_TYPE
from .staging import check_session_interface
from .states import parse_statement_form


@dataclass(frozen=True)
class Task:
    interface: str
    name: str
    kind: str
    left_author: str
    right_author: str
    rows: tuple
    definitions: int
    result_type: bool
    breaking: bool
    score: int
    band: str
    clean_count: int
    conflicting_count: int
    one_sided_count: int
    missing_count: int
    note: object = None


def _band(score):
    if score <= 2:
        return "small"
    if score <= 4:
        return "medium"
    return "large"


def build_tasks(left, right, divergences, staging):
    """Build one sized task per divergence, in handback order."""
    check_session_interface(left, right, divergences)
    if (
        staging.interface != left.interface
        or staging.left_author != left.author
        or staging.right_author != right.author
    ):
        raise TaskError(
            "a reconciliation task cannot be stated, the staging "
            "summary belongs to a different session"
        )
    ordered = sorted(divergences, key=lambda d: (d.interface, d.name, d.kind))
    return tuple(_build_task(left, right, d, staging) for d in ordered)


def _build_task(left, right, divergence, staging):
    if not divergence.interface or not divergence.name or not divergence.kind:
        raise TaskError(
            "a reconciliation task cannot be stated, the divergence "
            "is missing its interface, name, or kind"
        )
    if not divergence.rows or not divergence.definitions:
        raise TaskError(
            f"a reconciliation task for {divergence.name} cannot be "
            "stated, the divergence quotes no defining line"
        )
    if divergence.kind == "excluded-but-defined" and divergence.excludes_row is None:
        raise TaskError(
            f"a reconciliation task for {divergence.name} cannot be "
            "stated, the divergence quotes no excludes line"
        )
    result_type = divergence.kind == RESULT_TYPE or any(
        (parse_statement_form(row.line) or ("",))[0] == "operation"
        for row in divergence.definitions
    )
    breaking = divergence.name in left.breaking or divergence.name in right.breaking
    definitions = len(divergence.definitions)
    score = definitions + (2 if result_type else 0) + (3 if breaking else 0)
    return Task(
        interface=divergence.interface,
        name=divergence.name,
        kind=divergence.kind,
        left_author=left.author,
        right_author=right.author,
        rows=divergence.rows,
        definitions=definitions,
        result_type=result_type,
        breaking=breaking,
        score=score,
        band=_band(score),
        clean_count=len(staging.clean),
        conflicting_count=len(staging.conflicting),
        one_sided_count=len(staging.one_sided),
        missing_count=len(staging.missing),
        note=(
            "the staging cross-apply classifies this name clean "
            "and this task follows the record"
            if divergence.name in staging.clean
            else None
        ),
    )


def _yes_no(flag):
    return "yes" if flag else "no"


def format_tasks(tasks):
    """Render the handback document as deterministic text."""
    blocks = []
    for task in tasks:
        lines = [
            f"task {task.interface} {task.name}",
            f"authors {task.left_author} {task.right_author}",
            f"kind {task.kind}",
        ]
        for row in task.rows:
            lines.append(f"  {row.side} ({row.source}) {row.line}")
        lines.append(f"size {task.band} score {task.score}")
        lines.append(
            f"factors definitions {task.definitions}, "
            f"result-type {_yes_no(task.result_type)}, "
            f"breaking {_yes_no(task.breaking)}"
        )
        if task.note is not None:
            lines.append(f"note {task.note}")
        lines.append(
            f"staging clean {task.clean_count}, "
            f"conflicting {task.conflicting_count}, "
            f"one-sided {task.one_sided_count}, "
            f"missing {task.missing_count}"
        )
        blocks.append("\n".join(lines))
    if not blocks:
        return ""
    return "\n".join(blocks) + "\n"
