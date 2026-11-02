# Contract - crosscurrent

This contract is the executable spec for crosscurrent. Every obligation below is stated so it can fail. The full contract is the first build act of this repo and it extends the stub obligations written here.

## Stub obligations

XC-S1. Every live branch in a reconciliation session is cross-applied against the shared staging branch, and a branch that was never applied is reported as missing rather than assumed integrated.

XC-S2. A divergence found on the staging branch is handed back to the two authors who caused it as one sized reconciliation task, and a divergence with no task is a defect the tests must catch.

XC-S3. A reconciliation task states the concept that diverged, both conflicting forms, and an estimate of the work to reconcile them, and a task missing any of those is rejected as incomplete.

XC-S4. Crosscurrent reconciles only the pairs it is handed by detection, and it never scans the whole fleet looking for new divergence on its own behalf.

XC-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.
