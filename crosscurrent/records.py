"""Parsing for recorded divergence records.

A divergence record is the report forkdrift's comparison prints,
recorded to a file. Each block names one interface, one diverged
name, and one divergence kind, and quotes the defining lines with
the source each line came from. A presence block also quotes the
interface line of the side that defines nothing for the name, and
that row is not a definition.
"""

import re
from dataclasses import dataclass

from .errors import RecordError
from .states import IDENTIFIER, parse_statement_form

TERM_VALUE = "term-value"
RESULT_TYPE = "result-type"
EXCLUDED_BUT_DEFINED = "excluded-but-defined"
PRESENCE = "presence"
KINDS = (TERM_VALUE, RESULT_TYPE, EXCLUDED_BUT_DEFINED, PRESENCE)

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
    interface_row: object = None

    @property
    def definitions(self):
        """The defining lines among the quoted rows, derived never stored."""
        return tuple(
            row
            for row in self.rows
            if row != self.excludes_row and row != self.interface_row
        )


def parse_divergence_record(text, source):
    """Parse one recorded divergence record or raise RecordError."""
    blocks = []
    pending = None
    side_sources = {}
    for lineno, raw in enumerate(text.splitlines(), start=1):
        if not raw.strip():
            continue
        if raw.startswith("divergence "):
            if pending is not None:
                block = _finish_block(pending, source)
                _check_row_sources(block, pending, side_sources, source)
                blocks.append(block)
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
        block = _finish_block(pending, source)
        _check_row_sources(block, pending, side_sources, source)
        blocks.append(block)
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


def _check_row_sources(block, pending, side_sources, source):
    """Hold a finished block to the record's row sources under XC-18.

    The record's first block establishes the source of each side, and
    every later row names the source of its side. The block's own
    shape checks have already run, so this pass sees only rows the
    Round 1 and Round 2 grammar admitted.
    """
    where = (
        f"block for {block.name} of interface {block.interface} "
        f"at line {pending['lineno']}"
    )
    for row in block.rows:
        known = side_sources.get(row.side)
        if known is None:
            side_sources[row.side] = row.source
        elif known != row.source:
            raise RecordError(
                f"{source} is malformed, {where} names source "
                f"{row.source} on the {row.side}, which is not the "
                f"{row.side} source of the record, {known}"
            )
    left = side_sources.get("left")
    if left is not None and left == side_sources.get("right"):
        raise RecordError(
            f"{source} is malformed, {where} names the one source "
            f"{left} on both sides"
        )


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
    if kind == PRESENCE:
        return _finish_presence_block(pending, rows, source)
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


def _finish_presence_block(pending, rows, source):
    interface = pending["interface"]
    name = pending["name"]
    where = f"block for {name} at line {pending['lineno']}"
    if len(rows) not in (2, 3):
        raise RecordError(
            f"{source} is malformed, {where} must quote one left "
            "row and one right row and at most one further row"
        )
    if [row.side for row in rows[:2]] != ["left", "right"]:
        raise RecordError(
            f"{source} is malformed, {where} must quote one "
            "left line and one right line"
        )
    interface_line = f"interface {interface}"
    side_sources = {row.side: row.source for row in rows[:2]}
    defining = {"left": [], "right": []}
    interface_row = None
    for position, row in enumerate(rows):
        if position == 2 and row.source != side_sources[row.side]:
            raise RecordError(
                f"{source} is malformed, {where} labels its further "
                f"row {row.side} but names source {row.source}, "
                f"which is not the {row.side} source"
            )
        if row.line == interface_line:
            if position == 2:
                raise RecordError(
                    f"{source} is malformed, {where} quotes the "
                    "interface line as its further row, and a "
                    f"further row quotes a defining line for {name}"
                )
            interface_row = row
            continue
        form = parse_statement_form(row.line)
        if (
            form is None
            or form[0] not in ("term", "operation")
            or form[1] != name
        ):
            raise RecordError(
                f"{source} is malformed, {where} quotes a line "
                f"that is neither a defining line for {name} nor "
                f"the interface line, {row.line}"
            )
        defining[row.side].append(form[0])
    for side in ("left", "right"):
        forms = defining[side]
        side_row = rows[0] if side == "left" else rows[1]
        if side_row.line == interface_line and forms:
            raise RecordError(
                f"{source} is malformed, {where} quotes the "
                f"interface line on the {side} and a defining "
                f"line for {name} from the {side} as well"
            )
        if len(forms) == 2 and forms != ["term", "operation"]:
            raise RecordError(
                f"{source} is malformed, {where} quotes defining "
                f"lines for {name} from the {side} that are not a "
                "term line then an operation line"
            )
    shared = set(defining["left"]) & set(defining["right"])
    if shared:
        raise RecordError(
            f"{source} is malformed, {where} quotes a "
            f"{sorted(shared)[0]} line for {name} from both sides, "
            "and a presence block quotes each form from one "
            "side only"
        )
    if not defining["left"] and not defining["right"]:
        raise RecordError(
            f"{source} is malformed, {where} quotes no defining "
            f"line for {name}"
        )
    return Divergence(
        interface, name, PRESENCE, tuple(rows), None, interface_row
    )
