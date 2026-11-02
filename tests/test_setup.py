import hashlib
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]
CANONICAL_PREFIX_SHA256 = "63d2f04005dc8eb111d3efca1be55c0911f9ad46bc7cf3c74d9f9908f4a86f11"
EXPECTED_OBLIGATIONS = [
    'XC-S1. Every live branch in a reconciliation session is cross-applied against the shared staging branch, and a branch that was never applied is reported as missing rather than assumed integrated.',
    'XC-S2. A divergence found on the staging branch is handed back to the two authors who caused it as one sized reconciliation task, and a divergence with no task is a defect the tests must catch.',
    'XC-S3. A reconciliation task states the concept that diverged, both conflicting forms, and an estimate of the work to reconcile them, and a task missing any of those is rejected as incomplete.',
    'XC-S4. Crosscurrent reconciles only the pairs it is handed by detection, and it never scans the whole fleet looking for new divergence on its own behalf.',
    'XC-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.',
]
EXPECTED_PREFIX = "XC"
TEXT_SUFFIXES = {".md", ".yml", ".yaml", ".py", ".toml", ".txt", ".json", ".jsonl"}
ANY_DEPTH_SKIP = {".git", ".pytest_cache", "__pycache__"}
ROOT_SKIP = {".venv", "venv", "node_modules", ".tox"}


def test_contract_stub_obligations_exist():
    text = (ROOT / "CONTRACT.md").read_text()
    found = []
    for ln in text.splitlines():
        m = re.match(r"^([A-Z]{2})-S\d\.", ln)
        if m:
            found.append((m.group(1), ln))
    assert len(found) >= 5, f"expected at least 5 stub obligations, found {len(found)}"
    numbers = sorted(int(re.search(r"-S(\d)\.", ln).group(1)) for _, ln in found)
    texts = [ln for _, ln in found]
    assert texts == EXPECTED_OBLIGATIONS, "stub obligation text has changed without a contract amendment"
    assert numbers == [1, 2, 3, 4, 5], f"stub obligations must be exactly S1 to S5, found {numbers}"
    for prefix, ln in found:
        assert prefix == EXPECTED_PREFIX, f"obligation {ln[:12]} is not a {EXPECTED_PREFIX} obligation"
        words = len(ln.split())
        assert words >= 20, f"obligation too thin at {words} words, {ln[:40]}"


def test_agents_canonical_block_present():
    text = (ROOT / "AGENTS.md").read_text()
    assert "repo annex below this line" in text
    prefix = text.split("## Repo annex", 1)[0]
    digest = hashlib.sha256(prefix.encode()).hexdigest()
    assert digest == CANONICAL_PREFIX_SHA256, "canonical block above the annex line has drifted"
    annex = text.split("## Repo annex", 1)[1]
    assert "CONTRACT.md" in annex


def test_readme_status_reports_rounds():
    text = (ROOT / "README.md").read_text()
    counts = re.findall(r"Closed rounds (\d+) of 100(?!\d)", text)
    assert counts == ["2"], (
        "README status must report exactly one closed round count "
        "and it must be 0 of 100"
    )


def test_no_em_or_en_dash_in_text_files():
    bad = []
    for p in ROOT.rglob("*"):
        if not p.is_file() or p.suffix not in TEXT_SUFFIXES:
            continue
        parts = p.relative_to(ROOT).parts
        if any(part in ANY_DEPTH_SKIP for part in parts):
            continue
        if parts[0] in ROOT_SKIP:
            continue
        t = p.read_text()
        if "\u2014" in t or "\u2013" in t:
            bad.append(str(p.relative_to(ROOT)))
    assert not bad, f"em or en dash found in {bad}"
