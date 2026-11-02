import pytest

from crosscurrent import (
    InterfaceMismatchError,
    cross_apply,
    format_stage_report,
    parse_branch_state,
    parse_divergence_record,
)

LEFT = """interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
cache-ttl = 300
excludes = legacy-mode
"""

RIGHT = """interface shard-store
author engineer-b

replicas = 3
timeout = 45
operation store = receipt
operation fetch = record
"""

GHOST_RECORD = """divergence shard-store ghost term-value
  left (run-a.tape) ghost = 1
  right (run-b.tape) ghost = 2
"""


def stage(left_text=LEFT, right_text=RIGHT, record_text=""):
    left = parse_branch_state(left_text, "left.state")
    right = parse_branch_state(right_text, "right.state")
    divergences = parse_divergence_record(record_text, "record.txt")
    return cross_apply(left, right, divergences)


def test_cross_apply_classifies_every_defined_name():
    result = stage()
    assert result.clean == ("replicas",)
    assert result.conflicting == ("store", "timeout")
    assert result.one_sided == (("cache-ttl", "left"), ("fetch", "right"))
    assert result.missing == ()
    assert result.left_author == "engineer-a"
    assert result.right_author == "engineer-b"


def test_numeric_worth_and_whitespace_normalizations_are_clean():
    left = LEFT.replace("timeout = 30", "timeout = 30")
    right = RIGHT.replace("timeout = 45", "timeout = 30.0")
    result = stage(left, right)
    assert "timeout" in result.clean
    left = "interface shard-store\nauthor a\n\nlimit = 1e3\n"
    right = "interface shard-store\nauthor b\n\nlimit = 1000\n"
    assert stage(left, right).clean == ("limit",)
    left = "interface shard-store\nauthor a\n\nnote = two   words\n"
    right = "interface shard-store\nauthor b\n\nnote = two words\n"
    assert stage(left, right).clean == ("note",)


def test_a_name_defined_in_different_namespaces_conflicts():
    left = "interface shard-store\nauthor a\n\nfetch = cached\n"
    right = "interface shard-store\nauthor b\n\noperation fetch = record\n"
    result = stage(left, right)
    assert result.conflicting == ("fetch",)
    assert result.clean == ()
    assert result.one_sided == ()


def test_a_name_defined_in_both_namespaces_can_be_clean():
    text_a = "interface shard-store\nauthor a\n\ndual = 1\noperation dual = ack\n"
    text_b = "interface shard-store\nauthor b\n\ndual = 1\noperation dual = ack\n"
    assert stage(text_a, text_b).clean == ("dual",)
    text_c = "interface shard-store\nauthor b\n\ndual = 1\noperation dual = receipt\n"
    assert stage(text_a, text_c).conflicting == ("dual",)


def test_labels_excludes_and_breaking_lines_never_change_classification():
    decorated_left = LEFT.replace(
        "cache-ttl = 300", "cache-ttl = 300\nlabel primary = node-a\nbreaking timeout"
    )
    plain = stage()
    decorated = stage(decorated_left, RIGHT)
    assert decorated.clean == plain.clean
    assert decorated.conflicting == plain.conflicting
    assert decorated.one_sided == plain.one_sided


def test_a_diverged_name_defined_nowhere_is_reported_missing():
    result = stage(record_text=GHOST_RECORD)
    assert result.missing == ("ghost",)
    assert "ghost" not in result.clean
    assert "ghost" not in result.conflicting
    assert all(name != "ghost" for name, _ in result.one_sided)


def test_a_presence_name_defined_nowhere_is_reported_missing():
    record = (
        "divergence shard-store ghost presence\n"
        "  left (run-a.tape) ghost = 1\n"
        "  right (run-b.tape) interface shard-store\n"
    )
    result = stage(record_text=record)
    assert result.missing == ("ghost",)
    assert "ghost" not in result.clean
    assert "ghost" not in result.conflicting
    assert all(name != "ghost" for name, _ in result.one_sided)


def test_a_presence_name_defined_on_one_side_is_one_sided():
    record = (
        "divergence shard-store cache-ttl presence\n"
        "  left (run-a.tape) cache-ttl = 300\n"
        "  right (run-b.tape) interface shard-store\n"
    )
    result = stage(record_text=record)
    assert ("cache-ttl", "left") in result.one_sided
    assert result.missing == ()


def test_a_diverged_name_defined_in_the_states_is_not_missing():
    record = (
        "divergence shard-store timeout term-value\n"
        "  left (run-a.tape) timeout = 30\n"
        "  right (run-b.tape) timeout = 45\n"
    )
    assert stage(record_text=record).missing == ()


def test_states_naming_different_interfaces_are_refused():
    other = RIGHT.replace("interface shard-store", "interface shard-cache")
    with pytest.raises(InterfaceMismatchError) as excinfo:
        stage(LEFT, other)
    assert "shard-store" in str(excinfo.value)
    assert "shard-cache" in str(excinfo.value)


def test_a_record_naming_a_different_interface_is_refused():
    record = GHOST_RECORD.replace("shard-store", "shard-cache")
    with pytest.raises(InterfaceMismatchError) as excinfo:
        stage(record_text=record)
    assert "shard-store" in str(excinfo.value)
    assert "shard-cache" in str(excinfo.value)


def test_the_stage_report_text_is_exact_and_byte_stable():
    result = stage(record_text=GHOST_RECORD)
    expected = (
        "interface shard-store\n"
        "authors engineer-a engineer-b\n"
        "clean: replicas\n"
        "conflicting: store, timeout\n"
        "one-sided: cache-ttl (left only), fetch (right only)\n"
        "missing: ghost\n"
    )
    assert format_stage_report(result) == expected
    assert format_stage_report(stage(record_text=GHOST_RECORD)) == expected


def test_an_empty_session_reports_none_on_every_list():
    left = "interface shard-store\nauthor a\n"
    right = "interface shard-store\nauthor b\n"
    result = stage(left, right)
    expected = (
        "interface shard-store\n"
        "authors a b\n"
        "clean: none\n"
        "conflicting: none\n"
        "one-sided: none\n"
        "missing: none\n"
    )
    assert format_stage_report(result) == expected
