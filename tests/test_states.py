import pytest

from crosscurrent import StateError, parse_branch_state

FULL = """interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
label primary = node-a
excludes = legacy-mode
breaking timeout
"""


def test_parses_a_full_branch_state():
    state = parse_branch_state(FULL, "branch-a.state")
    assert state.interface == "shard-store"
    assert state.author == "engineer-a"
    assert state.terms["timeout"].value == "30"
    assert state.operations["store"].value == "ack"
    assert state.labels["primary"].value == "node-a"
    assert state.excludes == frozenset({"legacy-mode"})
    assert state.breaking == frozenset({"timeout"})
    assert state.terms["timeout"].line == "timeout = 30"


def test_comments_and_blank_lines_are_not_meaningful():
    text = "# a recorded note\n\ninterface shard-store\n# another\nauthor engineer-a\n\n"
    state = parse_branch_state(text, "s")
    assert state.interface == "shard-store"
    assert state.author == "engineer-a"


def test_statement_order_never_matters():
    first = "interface shard-store\nauthor engineer-a\n\ntimeout = 30\nreplicas = 3\n"
    second = "interface shard-store\nauthor engineer-a\n\nreplicas = 3\ntimeout = 30\n"
    a = parse_branch_state(first, "a")
    b = parse_branch_state(second, "b")
    assert a.terms == b.terms


def test_value_whitespace_is_collapsed_at_parse():
    text = "interface shard-store\nauthor engineer-a\n\nnote = two   words\n"
    state = parse_branch_state(text, "s")
    assert state.terms["note"].value == "two words"


def test_label_names_are_never_dereferenced():
    text = (
        "interface shard-store\nauthor engineer-a\n\n"
        "label primary = node-a\nnote = primary\n"
    )
    state = parse_branch_state(text, "s")
    assert state.terms["note"].value == "primary"


@pytest.mark.parametrize(
    "text, fragment",
    [
        ("", "missing interface line"),
        ("timeout = 30\nauthor engineer-a\n", "missing interface line"),
        ("interface 9bad\nauthor engineer-a\n", "missing interface line"),
        ("interface shard-store\n", "missing author line"),
        ("interface shard-store\ntimeout = 30\n", "missing author line"),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nauthor engineer-b\n",
            "author line out of place",
        ),
        (
            "interface shard-store\nauthor engineer-a\ninterface shard-cache\n",
            "more than one interface line",
        ),
        (
            "interface shard-store\nauthor engineer-a\nthis is not a statement\n",
            "matches no statement form",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\ntimeout = 45\n",
            "duplicate term name timeout",
        ),
        (
            "interface shard-store\nauthor engineer-a\noperation store = ack\noperation store = receipt\n",
            "duplicate operation name store",
        ),
        (
            "interface shard-store\nauthor engineer-a\nlabel a = x\nlabel a = y\n",
            "duplicate label name a",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout =\n",
            "empty value",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nbreaking ghost\n",
            "names ghost, which the state does not define",
        ),
        (
            "interface shard-store\nauthor engineer-a\ntimeout = 30\nbreaking timeout\nbreaking timeout\n",
            "duplicate breaking line for timeout",
        ),
        (
            "interface shard-store\nauthor engineer-a\nlegacy-mode = on\nexcludes = legacy-mode\n",
            "defines legacy-mode and also excludes it",
        ),
        (
            "interface shard-store\nauthor engineer-a\nexcludes = a\nexcludes = b\n",
            "more than one excludes line",
        ),
    ],
)
def test_malformed_branch_states_are_refused_by_name(text, fragment):
    with pytest.raises(StateError) as excinfo:
        parse_branch_state(text, "branch.state")
    assert fragment in str(excinfo.value)
    assert "branch.state" in str(excinfo.value)
