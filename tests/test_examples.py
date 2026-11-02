import os
import pathlib
import subprocess
import sys

from crosscurrent import (
    cross_apply,
    format_stage_report,
    parse_branch_state,
    parse_divergence_record,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]
LEFT = ROOT / "examples" / "branch-a.state"
RIGHT = ROOT / "examples" / "branch-b.state"
RECORD = ROOT / "examples" / "divergences.txt"
COMMITTED = ROOT / "examples" / "handback.txt"


def run_example_handback():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "crosscurrent", "handback",
         "examples/branch-a.state", "examples/branch-b.state",
         "examples/divergences.txt"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )


def test_example_record_carries_exactly_one_divergence():
    found = parse_divergence_record(RECORD.read_text(), "examples/divergences.txt")
    assert len(found) == 1
    assert found[0].name == "store"
    assert found[0].kind == "result-type"


def test_example_quoted_lines_appear_in_the_branch_states():
    left_lines = set(LEFT.read_text().splitlines())
    right_lines = set(RIGHT.read_text().splitlines())
    found = parse_divergence_record(RECORD.read_text(), "examples/divergences.txt")
    for row in found[0].definitions:
        assert row.line in left_lines or row.line in right_lines


def test_example_staging_report_is_exact():
    left = parse_branch_state(LEFT.read_text(), "examples/branch-a.state")
    right = parse_branch_state(RIGHT.read_text(), "examples/branch-b.state")
    found = parse_divergence_record(RECORD.read_text(), "examples/divergences.txt")
    expected = (
        "interface shard-store\n"
        "authors engineer-a engineer-b\n"
        "clean: replicas, timeout\n"
        "conflicting: store\n"
        "one-sided: cache-ttl (left only)\n"
        "missing: none\n"
    )
    assert format_stage_report(cross_apply(left, right, found)) == expected


def test_committed_example_output_matches_regeneration():
    result = run_example_handback()
    assert result.returncode == 0
    assert result.stdout == COMMITTED.read_bytes()
    second = run_example_handback()
    assert second.stdout == result.stdout


PRESENCE_LEFT = ROOT / "examples" / "presence-branch-a.state"
PRESENCE_RIGHT = ROOT / "examples" / "presence-branch-b.state"
PRESENCE_RECORD = ROOT / "examples" / "presence-divergences.txt"
PRESENCE_COMMITTED = ROOT / "examples" / "presence-handback.txt"


def run_presence_example_handback():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "crosscurrent", "handback",
         "examples/presence-branch-a.state",
         "examples/presence-branch-b.state",
         "examples/presence-divergences.txt"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )


def test_presence_example_record_carries_all_four_kinds():
    found = parse_divergence_record(
        PRESENCE_RECORD.read_text(), "examples/presence-divergences.txt"
    )
    assert [(block.name, block.kind) for block in found] == [
        ("cache-ttl", "presence"),
        ("legacy-mode", "excluded-but-defined"),
        ("purge", "presence"),
        ("store", "result-type"),
        ("timeout", "term-value"),
    ]


def test_presence_example_quoted_lines_appear_in_their_named_sources():
    found = parse_divergence_record(
        PRESENCE_RECORD.read_text(), "examples/presence-divergences.txt"
    )
    for block in found:
        for row in block.rows:
            lines = (ROOT / row.source).read_text().splitlines()
            assert row.line in lines


def test_presence_example_staging_report_is_exact():
    left = parse_branch_state(
        PRESENCE_LEFT.read_text(), "examples/presence-branch-a.state"
    )
    right = parse_branch_state(
        PRESENCE_RIGHT.read_text(), "examples/presence-branch-b.state"
    )
    found = parse_divergence_record(
        PRESENCE_RECORD.read_text(), "examples/presence-divergences.txt"
    )
    expected = (
        "interface shard-store\n"
        "authors engineer-a engineer-b\n"
        "clean: replicas, timeout\n"
        "conflicting: store\n"
        "one-sided: cache-ttl (left only), legacy-mode (right only)\n"
        "missing: purge\n"
    )
    assert format_stage_report(cross_apply(left, right, found)) == expected


def test_committed_presence_example_output_matches_regeneration():
    result = run_presence_example_handback()
    assert result.returncode == 0
    assert result.stdout == PRESENCE_COMMITTED.read_bytes()
    second = run_presence_example_handback()
    assert second.stdout == result.stdout
