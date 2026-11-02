"""Command line entry point for crosscurrent."""

import argparse
import sys
from pathlib import Path

from .errors import CrosscurrentError
from .records import parse_divergence_record
from .staging import cross_apply, format_stage_report
from .states import parse_branch_state
from .tasks import build_tasks, format_tasks


def _read(path):
    try:
        return Path(path).read_text(encoding="utf-8")
    except UnicodeDecodeError:
        raise CrosscurrentError(f"{path} is not readable as utf-8 text")


def _load_session(args):
    left = parse_branch_state(_read(args.left), args.left)
    right = parse_branch_state(_read(args.right), args.right)
    divergences = parse_divergence_record(_read(args.divergences), args.divergences)
    return left, right, divergences


def main(argv=None):
    parser = argparse.ArgumentParser(
        prog="crosscurrent",
        description="Reconcile detected divergence between two "
                    "recorded branch states into sized tasks.",
    )
    sub = parser.add_subparsers(dest="command", required=True)
    for name, help_text in (
        ("stage", "Print the cross-apply report for two branch states."),
        ("handback", "Print one sized reconciliation task per divergence."),
    ):
        child = sub.add_parser(name, help=help_text)
        child.add_argument("left")
        child.add_argument("right")
        child.add_argument("divergences")
    args = parser.parse_args(argv)
    try:
        left, right, divergences = _load_session(args)
        staging = cross_apply(left, right, divergences)
        if args.command == "stage":
            sys.stdout.write(format_stage_report(staging))
        else:
            tasks = build_tasks(left, right, divergences, staging)
            sys.stdout.write(format_tasks(tasks))
    except CrosscurrentError as exc:
        sys.stderr.write(f"error: {exc}\n")
        return 2
    except OSError as exc:
        sys.stderr.write(
            f"error: cannot read {exc.filename}, {exc.strerror}\n"
        )
        return 2
    return 0
