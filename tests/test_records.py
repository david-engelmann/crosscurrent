import pytest

from crosscurrent import RecordError, parse_divergence_record

TERM_RECORD = """divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 45
"""

RESULT_RECORD = """divergence shard-store store result-type
  left (run-a.tape) operation store = ack
  right (run-b.tape) operation store = receipt
"""

EXCLUDED_RECORD = """divergence shard-store legacy-mode excluded-but-defined
  left (run-a.tape) excludes = legacy-mode
  right (run-b.tape) legacy-mode = on
"""

EXCLUDED_DUAL_RECORD = """divergence shard-store dual excluded-but-defined
  left (run-a.tape) dual = 1
  right (run-b.tape) excludes = dual
  right (run-a.tape) operation dual = ack
"""


def test_parses_a_term_value_block():
    found = parse_divergence_record(TERM_RECORD, "record.txt")
    assert len(found) == 1
    block = found[0]
    assert block.interface == "shard-store"
    assert block.name == "timeout"
    assert block.kind == "term-value"
    assert [row.side for row in block.rows] == ["left", "right"]
    assert block.rows[0].source == "run-a.tape"
    assert block.rows[0].line == "timeout = 30"
    assert block.definitions == block.rows
    assert block.excludes_row is None


def test_parses_a_result_type_block():
    found = parse_divergence_record(RESULT_RECORD, "record.txt")
    assert found[0].kind == "result-type"
    assert found[0].definitions[1].line == "operation store = receipt"


def test_parses_an_excluded_but_defined_block():
    found = parse_divergence_record(EXCLUDED_RECORD, "record.txt")
    block = found[0]
    assert block.kind == "excluded-but-defined"
    assert block.excludes_row.line == "excludes = legacy-mode"
    assert [row.line for row in block.definitions] == ["legacy-mode = on"]


def test_parses_a_dual_definition_excluded_block():
    found = parse_divergence_record(EXCLUDED_DUAL_RECORD, "record.txt")
    block = found[0]
    assert len(block.rows) == 3
    assert [row.line for row in block.definitions] == [
        "dual = 1",
        "operation dual = ack",
    ]
    assert {row.source for row in block.definitions} == {"run-a.tape"}
    assert block.excludes_row.source == "run-b.tape"


def test_parses_several_blocks_for_different_names():
    found = parse_divergence_record(TERM_RECORD + "\n" + RESULT_RECORD, "r")
    assert [block.name for block in found] == ["timeout", "store"]


def test_an_empty_record_parses_to_no_divergences():
    assert parse_divergence_record("", "r") == ()
    assert parse_divergence_record("\n\n", "r") == ()


@pytest.mark.parametrize(
    "text, fragment",
    [
        ("divergence shard-store timeout\n", "matches no form"),
        ("divergence shard-store timeout weird-kind\n", "does not know"),
        ("  left (run-a.tape) timeout = 30\n", "matches no form"),
        ("a stray line\n", "matches no form"),
        (
            "divergence shard-store timeout term-value\n"
            "  left (run-a.tape) operation timeout = ack\n"
            "  right (run-b.tape) timeout = 45\n",
            "not a term line for timeout",
        ),
        (
            "divergence shard-store timeout term-value\n"
            "  left (run-a.tape) retries = 3\n"
            "  right (run-b.tape) timeout = 45\n",
            "not a term line for timeout",
        ),
        (
            "divergence shard-store timeout term-value\n"
            "  left (run-a.tape) timeout = 30\n"
            "  left (run-b.tape) timeout = 45\n",
            "one left line and one right line",
        ),
        (
            "divergence shard-store timeout term-value\n"
            "  left (run-a.tape) timeout = 30\n",
            "one left line and one right line",
        ),
        (
            "divergence shard-store store result-type\n"
            "  left (run-a.tape) store = ack\n"
            "  right (run-b.tape) operation store = receipt\n",
            "not a operation line for store",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n"
            "  left (run-a.tape) legacy-mode = off\n"
            "  right (run-b.tape) legacy-mode = on\n",
            "exactly one excludes line",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n"
            "  left (run-a.tape) excludes = other-name\n"
            "  right (run-b.tape) legacy-mode = on\n",
            "neither an excludes line",
        ),
        (
            "divergence shard-store dual excluded-but-defined\n"
            "  left (run-a.tape) dual = 1\n"
            "  right (run-b.tape) excludes = dual\n"
            "  right (run-c.tape) operation dual = ack\n",
            "two different sources",
        ),
        (
            "divergence shard-store legacy-mode excluded-but-defined\n"
            "  left (run-a.tape) excludes = legacy-mode\n"
            "  right (run-a.tape) legacy-mode = on\n",
            "the one source",
        ),
        (
            "divergence shard-store dual excluded-but-defined\n"
            "  left (run-a.tape) dual = 1\n"
            "  right (run-b.tape) excludes = dual\n"
            "  right (run-a.tape) dual = 2\n",
            "two defining lines of the one form",
        ),
        (TERM_RECORD + "\n" + TERM_RECORD, "two blocks for name timeout"),
        (
            TERM_RECORD + "\n"
            "divergence shard-cache timeout term-value\n"
            "  left (run-a.tape) timeout = 30\n"
            "  right (run-b.tape) timeout = 45\n",
            "two different interfaces",
        ),
    ],
)
def test_malformed_records_are_refused_by_name(text, fragment):
    with pytest.raises(RecordError) as excinfo:
        parse_divergence_record(text, "record.txt")
    assert fragment in str(excinfo.value)
    assert "record.txt" in str(excinfo.value)
