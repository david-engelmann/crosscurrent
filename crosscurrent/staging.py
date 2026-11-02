"""The staging area model over two recorded branch states."""

from dataclasses import dataclass

from .errors import InterfaceMismatchError
from .states import values_equal


@dataclass(frozen=True)
class CrossApply:
    interface: str
    left_author: str
    right_author: str
    clean: tuple
    conflicting: tuple
    one_sided: tuple
    missing: tuple


def check_session_interface(left, right, divergences):
    """Refuse a session whose inputs name different interfaces."""
    if left.interface != right.interface:
        raise InterfaceMismatchError(
            "branch states name different interfaces, "
            f"{left.source} names {left.interface} and "
            f"{right.source} names {right.interface}"
        )
    for divergence in divergences:
        if divergence.interface != left.interface:
            raise InterfaceMismatchError(
                "the divergence record names a different interface, "
                f"the branch states name {left.interface} and the "
                f"record names {divergence.interface}"
            )


def _side_forms(state, name):
    forms = {}
    if name in state.terms:
        forms["term"] = state.terms[name].value
    if name in state.operations:
        forms["operation"] = state.operations[name].value
    return forms


def cross_apply(left, right, divergences):
    """Classify every defined name between the two branch states."""
    check_session_interface(left, right, divergences)
    names = (
        set(left.terms) | set(left.operations)
        | set(right.terms) | set(right.operations)
    )
    clean = []
    conflicting = []
    one_sided = []
    for name in sorted(names):
        left_forms = _side_forms(left, name)
        right_forms = _side_forms(right, name)
        if left_forms and right_forms:
            agrees = set(left_forms) == set(right_forms) and all(
                values_equal(left_forms[kind], right_forms[kind])
                for kind in left_forms
            )
            if agrees:
                clean.append(name)
            else:
                conflicting.append(name)
        elif left_forms:
            one_sided.append((name, "left"))
        else:
            one_sided.append((name, "right"))
    missing = sorted({d.name for d in divergences} - names)
    return CrossApply(
        interface=left.interface,
        left_author=left.author,
        right_author=right.author,
        clean=tuple(clean),
        conflicting=tuple(conflicting),
        one_sided=tuple(one_sided),
        missing=tuple(missing),
    )


def _name_list(names):
    return ", ".join(names) if names else "none"


def format_stage_report(result):
    """Render the cross-apply report as deterministic text."""
    one_sided = ", ".join(
        f"{name} ({side} only)" for name, side in result.one_sided
    )
    lines = [
        f"interface {result.interface}",
        f"authors {result.left_author} {result.right_author}",
        f"clean: {_name_list(result.clean)}",
        f"conflicting: {_name_list(result.conflicting)}",
        f"one-sided: {one_sided if one_sided else 'none'}",
        f"missing: {_name_list(result.missing)}",
    ]
    return "\n".join(lines) + "\n"
