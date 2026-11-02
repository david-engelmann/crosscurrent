"""Parsing for recorded branch states.

A branch state is the recorded text one author's branch left behind
for one interface, written in the line based format CONTRACT.md
defines. The statement forms are the forms forkdrift defines for its
recorded artifacts, extended with an author line and breaking lines.
"""

import re
from dataclasses import dataclass
from decimal import Decimal, InvalidOperation

from .errors import StateError

IDENTIFIER = re.compile(r"[A-Za-z_][A-Za-z0-9_.\-]*")


@dataclass(frozen=True)
class Statement:
    name: str
    value: str
    line: str


@dataclass(frozen=True)
class BranchState:
    source: str
    interface: str
    author: str
    terms: dict
    operations: dict
    labels: dict
    excludes: frozenset
    excludes_line: str
    breaking: frozenset


def normalize_value(value):
    """Collapse every run of whitespace in a value to a single space."""
    return " ".join(value.split())


def values_equal(left, right):
    """Compare two normalized values under the XC-5 normalizations."""
    if left == right:
        return True
    try:
        left_number = Decimal(left)
        right_number = Decimal(right)
    except InvalidOperation:
        return False
    if not (left_number.is_finite() and right_number.is_finite()):
        return False
    return left_number == right_number


def parse_statement_form(line):
    """Classify one statement line from a recorded text.

    Returns a tuple of kind and name and value for a term, operation,
    or label line, or a tuple of kind and names for an excludes line,
    or None when the line matches no statement form.
    """
    if line.startswith("label "):
        parsed = _split_form(line[len("label "):])
        if parsed is None:
            return None
        name, value = parsed
        return ("label", name, value)
    if line.startswith("operation "):
        parsed = _split_form(line[len("operation "):])
        if parsed is None:
            return None
        name, value = parsed
        return ("operation", name, value)
    if "=" in line and line.split("=", 1)[0].strip() == "excludes":
        names = [part.strip() for part in line.split("=", 1)[1].split(",")]
        if not names or any(not IDENTIFIER.fullmatch(n) for n in names):
            return None
        return ("excludes", tuple(names))
    if "=" in line:
        parsed = _split_form(line)
        if parsed is None:
            return None
        name, value = parsed
        return ("term", name, value)
    return None


def _split_form(text):
    if "=" not in text:
        return None
    name, raw_value = text.split("=", 1)
    name = name.strip()
    value = raw_value.strip()
    if not IDENTIFIER.fullmatch(name) or not value:
        return None
    return name, value


def parse_branch_state(text, source):
    """Parse one recorded branch state or raise StateError naming why."""
    meaningful = []
    for lineno, raw in enumerate(text.splitlines(), start=1):
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        meaningful.append((lineno, stripped))
    if not meaningful:
        raise StateError(
            f"{source} is malformed, missing interface line, "
            "the branch state holds no meaningful line"
        )
    first_lineno, first = meaningful[0]
    head = first.split()
    if len(head) != 2 or head[0] != "interface" or not IDENTIFIER.fullmatch(head[1]):
        raise StateError(
            f"{source} is malformed, missing interface line, "
            f"line {first_lineno} does not name an interface"
        )
    interface = head[1]
    if len(meaningful) < 2:
        raise StateError(
            f"{source} is malformed, missing author line, "
            "the branch state holds no second meaningful line"
        )
    second_lineno, second = meaningful[1]
    head = second.split()
    if len(head) != 2 or head[0] != "author" or not IDENTIFIER.fullmatch(head[1]):
        raise StateError(
            f"{source} is malformed, missing author line, "
            f"line {second_lineno} does not name an author role"
        )
    author = head[1]
    terms = {}
    operations = {}
    labels = {}
    excludes = frozenset()
    excludes_line = None
    breaking = set()
    breaking_lines = {}
    for lineno, line in meaningful[2:]:
        if line.startswith("author "):
            raise StateError(
                f"{source} is malformed, author line out of place "
                f"at line {lineno}, the author line is the second "
                "meaningful line"
            )
        if line.startswith("interface "):
            raise StateError(
                f"{source} is malformed, more than one interface line, "
                f"second one at line {lineno}"
            )
        if line.startswith("breaking "):
            parts = line.split()
            if len(parts) != 2 or not IDENTIFIER.fullmatch(parts[1]):
                raise StateError(
                    f"{source} is malformed, line {lineno} matches no "
                    f"statement form, {line}"
                )
            name = parts[1]
            if name in breaking:
                raise StateError(
                    f"{source} is malformed, duplicate breaking line "
                    f"for {name} at line {lineno}"
                )
            breaking.add(name)
            breaking_lines[name] = lineno
            continue
        form = parse_statement_form(line)
        if form is None and "=" in line and not line.split("=", 1)[1].strip():
            raise StateError(
                f"{source} is malformed, line {lineno} states an "
                "empty value"
            )
        if form is None:
            raise StateError(
                f"{source} is malformed, line {lineno} matches no "
                f"statement form, {line}"
            )
        if form[0] == "excludes":
            if excludes_line is not None:
                raise StateError(
                    f"{source} is malformed, more than one excludes line, "
                    f"second one at line {lineno}"
                )
            excludes = frozenset(form[1])
            excludes_line = line
        else:
            kind, name, value = form
            table = {"term": terms, "operation": operations, "label": labels}[kind]
            _store(table, kind, name, value, line, source, lineno)
    for name in sorted(breaking):
        if name not in terms and name not in operations:
            raise StateError(
                f"{source} is malformed, breaking line at line "
                f"{breaking_lines[name]} names {name}, which the state "
                "does not define"
            )
    for name in sorted(excludes):
        if name in terms or name in operations:
            raise StateError(
                f"{source} is malformed, it defines {name} and also "
                "excludes it"
            )
    return BranchState(
        source=source,
        interface=interface,
        author=author,
        terms=terms,
        operations=operations,
        labels=labels,
        excludes=excludes,
        excludes_line=excludes_line,
        breaking=frozenset(breaking),
    )


def _store(table, kind, name, value, line, source, lineno):
    if name in table:
        raise StateError(
            f"{source} is malformed, duplicate {kind} name {name} "
            f"at line {lineno}"
        )
    table[name] = Statement(name=name, value=normalize_value(value), line=line)
