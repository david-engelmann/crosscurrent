import os
import pathlib
import subprocess
import sys

import pytest

from crosscurrent import (
    RecordError,
    build_tasks,
    cross_apply,
    format_tasks,
    parse_branch_state,
    parse_divergence_record,
)

ROOT = pathlib.Path(__file__).resolve().parents[1]

LEFT = """interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
cache-ttl = 300
"""

RIGHT = """interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
"""

EARLIER_RECORD = """divergence shard-store store result-type
  left (run-a.tape) operation store = ack
  right (run-b.tape) operation store = receipt
divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 45
"""

LATER_RECORD = """divergence shard-store cache-ttl presence
  left (run-a.tape) cache-ttl = 300
  right (run-b.tape) interface shard-store
divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 45
"""

FOREIGN_RECORD = """divergence shard-store timeout term-value
  left (other-a.tape) timeout = 30
  right (other-b.tape) timeout = 45
"""

CHANGED_TIMEOUT_RECORD = """divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 50
"""

MISSING_RECORD = """divergence shard-store purge presence
  left (run-a.tape) operation purge = ack
  right (run-b.tape) interface shard-store
"""


def named_records(record_texts):
    return [
        (
            f"record-{index}.txt",
            parse_divergence_record(text, f"record-{index}.txt"),
        )
        for index, text in enumerate(record_texts)
    ]


def bundle_tasks(record_texts, left_text=LEFT, right_text=RIGHT):
    from crosscurrent import build_tasks_bundle, bundle_divergences

    left = parse_branch_state(left_text, "left.state")
    right = parse_branch_state(right_text, "right.state")
    records = named_records(record_texts)
    union = bundle_divergences(records)
    staging = cross_apply(left, right, union)
    return build_tasks_bundle(left, right, records, staging)


def single_tasks(record_text, left_text=LEFT, right_text=RIGHT):
    left = parse_branch_state(left_text, "left.state")
    right = parse_branch_state(right_text, "right.state")
    divergences = parse_divergence_record(record_text, "record.txt")
    staging = cross_apply(left, right, divergences)
    return build_tasks(left, right, divergences, staging)


def test_a_bundle_builds_tasks_from_the_union_of_its_records():
    found = bundle_tasks([EARLIER_RECORD, LATER_RECORD])
    assert [task.name for task in found] == ["cache-ttl", "store", "timeout"]
    assert [task.kind for task in found] == [
        "presence",
        "result-type",
        "term-value",
    ]


def test_an_exact_duplicate_divergence_collapses_to_one_task():
    found = bundle_tasks([EARLIER_RECORD, LATER_RECORD])
    timeouts = [task for task in found if task.name == "timeout"]
    assert len(timeouts) == 1
    doubled = bundle_tasks([EARLIER_RECORD, EARLIER_RECORD])
    assert format_tasks(doubled) == format_tasks(
        single_tasks(EARLIER_RECORD)
    )


def test_the_same_name_and_kind_with_different_rows_produces_two_tasks():
    found = bundle_tasks([EARLIER_RECORD, CHANGED_TIMEOUT_RECORD])
    timeouts = [task for task in found if task.name == "timeout"]
    assert len(timeouts) == 2
    text = format_tasks(found)
    assert text.index("timeout = 45") < text.index("timeout = 50")


def test_the_tiebreak_makes_record_order_change_nothing():
    forward = bundle_tasks([EARLIER_RECORD, CHANGED_TIMEOUT_RECORD])
    backward = bundle_tasks([CHANGED_TIMEOUT_RECORD, EARLIER_RECORD])
    assert format_tasks(forward) == format_tasks(backward)
    forward = bundle_tasks([EARLIER_RECORD, LATER_RECORD])
    backward = bundle_tasks([LATER_RECORD, EARLIER_RECORD])
    assert format_tasks(forward) == format_tasks(backward)


def test_a_bundle_of_one_record_prints_the_single_record_handback():
    bundled = bundle_tasks([LATER_RECORD])
    single = single_tasks(LATER_RECORD)
    assert format_tasks(bundled) == format_tasks(single)


def test_a_record_from_other_sources_refuses_the_bundle():
    from crosscurrent import bundle_divergences

    records = named_records([EARLIER_RECORD, FOREIGN_RECORD])
    with pytest.raises(RecordError) as caught:
        bundle_divergences(records)
    assert str(caught.value) == (
        "record-1.txt establishes left other-a.tape and right "
        "other-b.tape and record-0.txt establishes left run-a.tape "
        "and right run-b.tape, and the records of one bundle must "
        "establish the one pair of sources"
    )


def test_an_empty_record_contributes_nothing_and_binds_nothing():
    with_empty = bundle_tasks(["", LATER_RECORD])
    plain = bundle_tasks([LATER_RECORD])
    assert format_tasks(with_empty) == format_tasks(plain)
    assert bundle_tasks([""]) == ()
    foreign_after_empty = bundle_tasks(["", EARLIER_RECORD])
    assert format_tasks(foreign_after_empty) == format_tasks(
        single_tasks(EARLIER_RECORD)
    )


def test_the_bundle_staging_summary_counts_the_union():
    found = bundle_tasks([EARLIER_RECORD, MISSING_RECORD])
    assert found
    for task in found:
        text = format_tasks((task,))
        assert (
            "staging clean 1, conflicting 2, one-sided 1, missing 1"
            in text
        )
    single = single_tasks(EARLIER_RECORD)
    for task in single:
        text = format_tasks((task,))
        assert "missing 0" in text


def run_cli(*args, cwd=ROOT):
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    return subprocess.run(
        [sys.executable, "-m", "crosscurrent", *args],
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
    )


def write(tmp_path, name, text):
    path = tmp_path / name
    path.write_text(text)
    return str(path)


def test_cli_handback_accepts_several_record_files(tmp_path):
    left = write(tmp_path, "left.state", LEFT)
    right = write(tmp_path, "right.state", RIGHT)
    earlier = write(tmp_path, "earlier.txt", EARLIER_RECORD)
    later = write(tmp_path, "later.txt", LATER_RECORD)
    result = run_cli("handback", left, right, earlier, later)
    assert result.returncode == 0
    assert result.stderr == ""
    expected = format_tasks(bundle_tasks([EARLIER_RECORD, LATER_RECORD]))
    assert result.stdout == expected
    swapped = run_cli("handback", left, right, later, earlier)
    assert swapped.returncode == 0
    assert swapped.stdout == result.stdout


def test_cli_handback_with_no_record_file_is_a_usage_error(tmp_path):
    left = write(tmp_path, "left.state", LEFT)
    right = write(tmp_path, "right.state", RIGHT)
    result = run_cli("handback", left, right)
    assert result.returncode == 2
    assert "usage" in result.stderr.lower()
    assert result.stdout == ""


def test_cli_a_bundle_from_other_sources_exits_2_with_one_error_line(
    tmp_path,
):
    left = write(tmp_path, "left.state", LEFT)
    right = write(tmp_path, "right.state", RIGHT)
    earlier = write(tmp_path, "earlier.txt", EARLIER_RECORD)
    foreign = write(tmp_path, "foreign.txt", FOREIGN_RECORD)
    result = run_cli("handback", left, right, earlier, foreign)
    assert result.returncode == 2
    assert result.stderr.startswith("error:")
    assert result.stderr.count("\n") == 1
    assert "other-a.tape" in result.stderr
    assert "Traceback" not in result.stderr
    assert result.stdout == ""


def test_cli_a_duplicate_record_file_collapses_like_one(tmp_path):
    left = write(tmp_path, "left.state", LEFT)
    right = write(tmp_path, "right.state", RIGHT)
    later = write(tmp_path, "later.txt", LATER_RECORD)
    once = run_cli("handback", left, right, later)
    twice = run_cli("handback", left, right, later, later)
    assert once.returncode == 0
    assert twice.returncode == 0
    assert twice.stdout == once.stdout
    assert once.stdout == format_tasks(bundle_tasks([LATER_RECORD]))


BUNDLE_LEFT = ROOT / "examples" / "presence-branch-a.state"
BUNDLE_RIGHT = ROOT / "examples" / "presence-branch-b.state"
BUNDLE_RECORD_A = ROOT / "examples" / "bundle-divergences-a.txt"
BUNDLE_RECORD_B = ROOT / "examples" / "bundle-divergences-b.txt"
BUNDLE_COMMITTED = ROOT / "examples" / "bundle-handback.txt"


def test_bundle_example_records_establish_the_one_pair_of_sources():
    from crosscurrent import record_sources

    for path in (BUNDLE_RECORD_A, BUNDLE_RECORD_B):
        found = parse_divergence_record(
            path.read_text(), f"examples/{path.name}"
        )
        assert record_sources(found) == (
            "examples/presence-run-a.tape",
            "examples/presence-run-b.tape",
        )


def test_bundle_example_records_carry_a_shared_block_and_one_each_more():
    first = parse_divergence_record(
        BUNDLE_RECORD_A.read_text(), "examples/bundle-divergences-a.txt"
    )
    second = parse_divergence_record(
        BUNDLE_RECORD_B.read_text(), "examples/bundle-divergences-b.txt"
    )
    first_keys = {(d.name, d.kind) for d in first}
    second_keys = {(d.name, d.kind) for d in second}
    assert first_keys & second_keys == {("timeout", "term-value")}
    assert len(first_keys) == 2
    assert len(second_keys) == 2


def test_committed_bundle_example_matches_regeneration():
    env = dict(os.environ)
    env["PYTHONPATH"] = str(ROOT)
    result = subprocess.run(
        [sys.executable, "-m", "crosscurrent", "handback",
         "examples/presence-branch-a.state",
         "examples/presence-branch-b.state",
         "examples/bundle-divergences-a.txt",
         "examples/bundle-divergences-b.txt"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    assert result.returncode == 0
    assert result.stdout == BUNDLE_COMMITTED.read_bytes()
    text = result.stdout.decode()
    assert text.count("task shard-store ") == 3
    assert text.count("task shard-store timeout") == 1
    second = subprocess.run(
        [sys.executable, "-m", "crosscurrent", "handback",
         "examples/presence-branch-a.state",
         "examples/presence-branch-b.state",
         "examples/bundle-divergences-b.txt",
         "examples/bundle-divergences-a.txt"],
        cwd=ROOT,
        env=env,
        capture_output=True,
    )
    assert second.stdout == result.stdout
