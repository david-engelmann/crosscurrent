"""Parsing for recorded divergence records.

A divergence record is the report forkdrift's comparison prints,
recorded to a file. Each block names one interface, one diverged
name, and one divergence kind, and quotes the defining lines with
the source each line came from.
"""

import re
from dataclasses import dataclass

from .errors import RecordError
from .states import IDENTIFIER, parse_statement_form

TERM_VALUE = "term-value"
RESULT_TYPE = "result-type"
EXCLUDED_BUT_DEFINED = "excluded-but-defined"
KINDS = (TERM_VALUE, RESULT_TYPE, EXCLUDED_BUT_DEFINED)

ROW = re.compile(r"^  (left|right) \((.+?)\) (.+)$")


@dataclass(frozen=True)
class QuotedLine:
    side: str
    source: str
    line: str


@dataclass(frozen=True)
class Divergence:
    interface: str
    name: str
    kind: str
    rows: tuple
    excludes_row: object = None

    @property
    def definitions(self):
        """The defining lines among the quoted rows, derived never stored."""
        if self.kind == EXCLUDED_BUT_DEFINED:
            return tuple(
                row for row in self.rows if row != self.excludes_row
            )
        return tuple(self.rows)


def parse_divergence_record(text, source):
    """Parse one recorded divergence record or raise RecordError."""
    blocks = []
    pending = None
    for lineno, raw in enumerate(text.splitlines(), start=1):
        if not raw.strip():
            continue
        if raw.startswith("divergence "):
            if pending is not None:
                blocks.append(_finish_block(pending, source))
            pending = _open_block(raw, source, lineno)
            continue
        match = ROW.match(raw)
        if match is not None and pending is not None:
            side, row_source, line = match.groups()
            pending["rows"].append(QuotedLine(side, row_source, line.strip()))
            continue
        raise RecordError(
            f"{source} is malformed, line {lineno} matches no form "
            f"in a divergence record, {raw.strip()}"
        )
    if pending is not None:
        blocks.append(_finish_block(pending, source))
    interfaces = {block.interface for block in blocks}
    if len(interfaces) > 1:
        raise RecordError(
            f"{source} is malformed, blocks name two different "
            f"interfaces, {sorted(interfaces)[0]} and "
            f"{sorted(interfaces)[1]}"
        )
    seen = set()
    for block in blocks:
        key = (block.name, block.kind)
        if key in seen:
            raise RecordError(
                f"{source} is malformed, two blocks for name "
                f"{block.name} and kind {block.kind}"
            )
        seen.add(key)
    return tuple(blocks)


def _open_block(raw, source, lineno):
    parts = raw.split()
    if (
        len(parts) != 4
        or not IDENTIFIER.fullmatch(parts[1])
        or not IDENTIFIER.fullmatch(parts[2])
    ):
        raise RecordError(
            f"{source} is malformed, block header at line {lineno} "
            f"matches no form, {raw.strip()}"
        )
    if parts[3] not in KINDS:
        raise RecordError(
            f"{source} is malformed, block header at line {lineno} "
            f"names a divergence kind this format does not know, "
            f"{parts[3]}"
        )
    return {
        "interface": parts[1],
        "name": parts[2],
        "kind": parts[3],
        "rows": [],
        "lineno": lineno,
    }


def _finish_block(pending, source):
    interface = pending["interface"]
    name = pending["name"]
    kind = pending["kind"]
    rows = pending["rows"]
    where = f"block for {name} at line {pending['lineno']}"
    if kind in (TERM_VALUE, RESULT_TYPE):
        want = "term" if kind == TERM_VALUE else "operation"
        if [row.side for row in rows] != ["left", "right"]:
            raise RecordError(
                f"{source} is malformed, {where} must quote one "
                "left line and one right line"
            )
        for row in rows:
            form = parse_statement_form(row.line)
            if form is None or form[0] != want or form[1] != name:
                raise RecordError(
                    f"{source} is malformed, {where} quotes a line "
                    f"that is not a {want} line for {name}, {row.line}"
                )
        return Divergence(interface, name, kind, tuple(rows))
    excludes_rows = []
    define_rows = []
    for row in rows:
        form = parse_statement_form(row.line)
        if form is not None and form[0] == "excludes" and name in form[1]:
            excludes_rows.append(row)
        elif form is not None and form[0] in ("term", "operation") and form[1] == name:
            define_rows.append(row)
        else:
            raise RecordError(
                f"{source} is malformed, {where} quotes a line that "
                f"is neither an excludes line naming {name} nor a "
                f"defining line for {name}, {row.line}"
            )
    if len(excludes_rows) != 1:
        raise RecordError(
            f"{source} is malformed, {where} must quote exactly one "
            f"excludes line naming {name}"
        )
    if not 1 <= len(define_rows) <= 2:
        raise RecordError(
            f"{source} is malformed, {where} must quote one or two "
            f"defining lines for {name}"
        )
    define_kinds = [parse_statement_form(row.line)[0] for row in define_rows]
    if len(set(define_kinds)) != len(define_kinds):
        raise RecordError(
            f"{source} is malformed, {where} quotes two defining "
            f"lines of the one form for {name}"
        )
    if len({row.source for row in define_rows}) != 1:
        raise RecordError(
            f"{source} is malformed, {where} quotes defining lines "
            f"for {name} from two different sources"
        )
    if define_rows[0].source == excludes_rows[0].source:
        raise RecordError(
            f"{source} is malformed, {where} quotes the excludes "
            f"line and the defining lines for {name} from the one "
            f"source, {define_rows[0].source}"
        )
    return Divergence(
        interface, name, kind, tuple(rows), excludes_rows[0]
    )
