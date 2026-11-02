import pytest

from crosscurrent import (
    TaskError,
    build_tasks,
    cross_apply,
    format_tasks,
    parse_branch_state,
    parse_divergence_record,
)
from crosscurrent.records import Divergence, QuotedLine

LEFT = """interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
"""

RIGHT = """interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
"""

TERM_RECORD = """divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 45
"""

RESULT_RECORD = """divergence shard-store store result-type
  left (run-a.tape) operation store = ack
  right (run-b.tape) operation store = receipt
"""

EXCLUDED_TERM_RECORD = """divergence shard-store legacy-mode excluded-but-defined
  left (run-a.tape) excludes = legacy-mode
  right (run-b.tape) legacy-mode = on
"""

EXCLUDED_OPERATION_RECORD = """divergence shard-store sweeper excluded-but-defined
  left (run-a.tape) excludes = sweeper
  right (run-b.tape) operation sweeper = report
"""

EXCLUDED_DUAL_RECORD = """divergence shard-store dual excluded-but-defined
  left (run-a.tape) dual = 1
  right (run-b.tape) excludes = dual
  right (run-a.tape) operation dual = ack
"""


def tasks_for(record_text, left_text=LEFT, right_text=RIGHT):
    left = parse_branch_state(left_text, "left.state")
    right = parse_branch_state(right_text, "right.state")
    divergences = parse_divergence_record(record_text, "record.txt")
    staging = cross_apply(left, right, divergences)
    return build_tasks(left, right, divergences, staging)


def only_task(record_text, left_text=LEFT, right_text=RIGHT):
    found = tasks_for(record_text, left_text, right_text)
    assert len(found) == 1
    return found[0]


def test_every_divergence_produces_exactly_one_task():
    found = tasks_for(RESULT_RECORD + "\n" + TERM_RECORD)
    assert [task.name for task in found] == ["store", "timeout"]
    assert [task.kind for task in found] == ["result-type", "term-value"]


def test_tasks_are_sorted_by_name_then_kind_regardless_of_record_order():
    reversed_record = TERM_RECORD + "\n" + RESULT_RECORD
    found = tasks_for(reversed_record)
    assert [task.name for task in found] == ["store", "timeout"]


def test_a_task_states_authors_kind_and_quoted_forms():
    task = only_task(RESULT_RECORD)
    assert task.left_author == "engineer-a"
    assert task.right_author == "engineer-b"
    assert task.kind == "result-type"
    text = format_tasks((task,))
    assert "task shard-store store" in text
    assert "authors engineer-a engineer-b" in text
    assert "  left (run-a.tape) operation store = ack" in text
    assert "  right (run-b.tape) operation store = receipt" in text


def test_a_task_states_the_clean_apply_summary():
    task = only_task(TERM_RECORD)
    text = format_tasks((task,))
    assert "staging clean 1, conflicting 2, one-sided 0, missing 0" in text


def test_sizing_rule_scores_and_bands():
    task = only_task(TERM_RECORD)
    assert (task.definitions, task.result_type, task.breaking) == (2, False, False)
    assert (task.score, task.band) == (2, "small")

    task = only_task(RESULT_RECORD)
    assert (task.definitions, task.result_type, task.breaking) == (2, True, False)
    assert (task.score, task.band) == (4, "medium")

    task = only_task(EXCLUDED_TERM_RECORD)
    assert (task.definitions, task.result_type, task.breaking) == (1, False, False)
    assert (task.score, task.band) == (1, "small")

    task = only_task(EXCLUDED_OPERATION_RECORD)
    assert (task.definitions, task.result_type, task.breaking) == (1, True, False)
    assert (task.score, task.band) == (3, "medium")

    task = only_task(EXCLUDED_DUAL_RECORD)
    assert (task.definitions, task.result_type, task.breaking) == (2, True, False)
    assert (task.score, task.band) == (4, "medium")


def test_sizing_rule_breaking_factor_from_either_side():
    left_breaking = LEFT.replace(
        "operation store = ack", "operation store = ack\nbreaking timeout"
    )
    task = only_task(TERM_RECORD, left_text=left_breaking)
    assert (task.score, task.band) == (5, "large")

    right_breaking = RIGHT.replace(
        "operation store = receipt", "operation store = receipt\nbreaking store"
    )
    task = only_task(RESULT_RECORD, right_text=right_breaking)
    assert (task.score, task.band) == (7, "large")

    left_with_legacy = LEFT.replace(
        "operation store = ack",
        "operation store = ack\nlegacy-mode = off\nbreaking legacy-mode",
    )
    task = only_task(EXCLUDED_TERM_RECORD, left_text=left_with_legacy)
    assert (task.score, task.band) == (4, "medium")


def test_the_factors_line_prints_what_the_band_came_from():
    task = only_task(RESULT_RECORD)
    text = format_tasks((task,))
    assert "size medium score 4" in text
    assert "factors definitions 2, result-type yes, breaking no" in text


def test_an_empty_record_produces_no_tasks_and_empty_output():
    found = tasks_for("")
    assert found == ()
    assert format_tasks(found) == ""


def test_the_handback_is_byte_stable():
    record = RESULT_RECORD + "\n" + TERM_RECORD
    assert format_tasks(tasks_for(record)) == format_tasks(tasks_for(record))


def test_an_incomplete_divergence_is_refused_rather_than_printed_partial():
    left = parse_branch_state(LEFT, "left.state")
    right = parse_branch_state(RIGHT, "right.state")
    staging = cross_apply(left, right, ())
    no_rows = Divergence("shard-store", "timeout", "term-value", ())
    with pytest.raises(TaskError):
        build_tasks(left, right, (no_rows,), staging)
    rows = (
        QuotedLine("left", "run-a.tape", "timeout = 30"),
        QuotedLine("right", "run-b.tape", "timeout = 45"),
    )
    only_excludes = (QuotedLine("left", "run-a.tape", "excludes = timeout"),)
    no_definitions = Divergence(
        "shard-store", "timeout", "excluded-but-defined",
        only_excludes, only_excludes[0],
    )
    with pytest.raises(TaskError):
        build_tasks(left, right, (no_definitions,), staging)
    excluded_no_excludes_row = Divergence(
        "shard-store",
        "legacy-mode",
        "excluded-but-defined",
        rows,
        None,
    )
    with pytest.raises(TaskError):
        build_tasks(left, right, (excluded_no_excludes_row,), staging)


def test_task_notes_a_name_the_staging_calls_clean():
    agree_a = (
        "interface shard-store\n"
        "author engineer-a\n"
        "timeout = 30\n"
        "operation store = ack\n"
    )
    agree_b = (
        "interface shard-store\n"
        "author engineer-b\n"
        "timeout = 30\n"
        "operation store = ack\n"
    )
    left = parse_branch_state(agree_a, "a.state")
    right = parse_branch_state(agree_b, "b.state")
    forged = (
        "divergence shard-store timeout term-value\n"
        "  left (a.state) timeout = 30\n"
        "  right (b.state) timeout = 30\n"
    )
    divergences = parse_divergence_record(forged, "forged.txt")
    staging = cross_apply(left, right, divergences)
    tasks = build_tasks(left, right, divergences, staging)
    text = format_tasks(tasks)
    assert "note the staging cross-apply classifies this name clean" in text


def test_build_tasks_refuses_a_staging_summary_from_another_session():
    left = parse_branch_state(LEFT, "left.state")
    right = parse_branch_state(RIGHT, "right.state")
    other_left = parse_branch_state(
        LEFT.replace("engineer-a", "engineer-c"), "other.state"
    )
    foreign = cross_apply(other_left, right, ())
    with pytest.raises(TaskError):
        build_tasks(left, right, (), foreign)
