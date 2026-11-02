import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

LEFT = """interface shard-store
author engineer-a

timeout = 30
operation store = ack
"""

RIGHT = """interface shard-store
author engineer-b

timeout = 45
operation store = ack
"""

RECORD = """divergence shard-store timeout term-value
  left (run-a.tape) timeout = 30
  right (run-b.tape) timeout = 45
"""

MALFORMED_STATE = "timeout = 30\n"

OTHER_INTERFACE_STATE = """interface shard-cache
author engineer-b

timeout = 45
operation store = ack
"""

EXPECTED_STAGE = (
    "interface shard-store\n"
    "authors engineer-a engineer-b\n"
    "clean: store\n"
    "conflicting: timeout\n"
    "one-sided: none\n"
    "missing: none\n"
)

EXPECTED_HANDBACK = (
    "task shard-store timeout\n"
    "authors engineer-a engineer-b\n"
    "kind term-value\n"
    "  left (run-a.tape) timeout = 30\n"
    "  right (run-b.tape) timeout = 45\n"
    "size small score 2\n"
    "factors definitions 2, result-type no, breaking no\n"
    "staging clean 1, conflicting 1, one-sided 0, missing 0\n"
)


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


def session(tmp_path, left=LEFT, right=RIGHT, record=RECORD):
    return (
        write(tmp_path, "left.state", left),
        write(tmp_path, "right.state", right),
        write(tmp_path, "record.txt", record),
    )


def test_cli_stage_prints_the_cross_apply_report(tmp_path):
    left, right, record = session(tmp_path)
    result = run_cli("stage", left, right, record)
    assert result.returncode == 0
    assert result.stdout == EXPECTED_STAGE
    assert result.stderr == ""


def test_cli_handback_prints_the_sized_task(tmp_path):
    left, right, record = session(tmp_path)
    result = run_cli("handback", left, right, record)
    assert result.returncode == 0
    assert result.stdout == EXPECTED_HANDBACK
    second = run_cli("handback", left, right, record)
    assert second.stdout == result.stdout


def test_cli_handback_over_an_empty_record_prints_nothing(tmp_path):
    left, right, record = session(tmp_path, record="")
    result = run_cli("handback", left, right, record)
    assert result.returncode == 0
    assert result.stdout == ""


def test_cli_malformed_input_exits_2_without_traceback(tmp_path):
    left, right, record = session(tmp_path, left=MALFORMED_STATE)
    result = run_cli("stage", left, right, record)
    assert result.returncode == 2
    assert result.stderr.startswith("error:")
    assert "missing interface line" in result.stderr
    assert "Traceback" not in result.stderr
    assert result.stdout == ""


def test_cli_missing_file_exits_2_without_traceback(tmp_path):
    _, right, record = session(tmp_path)
    missing = str(tmp_path / "nope.state")
    result = run_cli("stage", missing, right, record)
    assert result.returncode == 2
    assert "Traceback" not in result.stderr


def test_cli_interface_mismatch_exits_2(tmp_path):
    left, right, record = session(tmp_path, right=OTHER_INTERFACE_STATE)
    result = run_cli("handback", left, right, record)
    assert result.returncode == 2
    assert "shard-cache" in result.stderr
    assert "Traceback" not in result.stderr


PRESENCE_RECORD = """divergence shard-store cache-ttl presence
  left (run-a.tape) cache-ttl = 300
  right (run-b.tape) interface shard-store
"""

EXPECTED_PRESENCE_HANDBACK = (
    "task shard-store cache-ttl\n"
    "authors engineer-a engineer-b\n"
    "kind presence\n"
    "  left (run-a.tape) cache-ttl = 300\n"
    "  right (run-b.tape) interface shard-store\n"
    "size small score 1\n"
    "factors definitions 1, result-type no, breaking no\n"
    "staging clean 1, conflicting 1, one-sided 0, missing 1\n"
)

MISLABELLED_PRESENCE_RECORD = """divergence shard-store dual presence
  left (run-a.tape) dual = 1
  right (run-b.tape) interface shard-store
  right (run-a.tape) operation dual = ack
"""


def test_cli_handback_over_a_presence_only_record(tmp_path):
    left, right, record = session(tmp_path, record=PRESENCE_RECORD)
    result = run_cli("handback", left, right, record)
    assert result.returncode == 0
    assert result.stdout == EXPECTED_PRESENCE_HANDBACK
    assert result.stderr == ""


def test_cli_mislabelled_presence_record_exits_2_without_traceback(tmp_path):
    left, right, record = session(tmp_path, record=MISLABELLED_PRESENCE_RECORD)
    result = run_cli("handback", left, right, record)
    assert result.returncode == 2
    assert result.stderr.startswith("error:")
    assert "not the right source" in result.stderr
    assert "Traceback" not in result.stderr
    assert result.stdout == ""


def test_cli_output_ignores_unrelated_files_beside_the_inputs(tmp_path):
    left, right, record = session(tmp_path)
    baseline = run_cli("handback", left, right, record, cwd=tmp_path)
    write(
        tmp_path,
        "unrelated.txt",
        "divergence shard-store timeout term-value\n"
        "  left (x.tape) timeout = 99\n"
        "  right (y.tape) timeout = 100\n",
    )
    write(tmp_path, "extra.state", OTHER_INTERFACE_STATE)
    after = run_cli("handback", left, right, record, cwd=tmp_path)
    assert after.returncode == 0
    assert after.stdout == baseline.stdout == EXPECTED_HANDBACK
