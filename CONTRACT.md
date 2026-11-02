# Contract - crosscurrent

This contract is the executable spec for crosscurrent. Every obligation below is stated so it can fail. The full contract is the first build act of this repo and it extends the stub obligations written here.

## Stub obligations

XC-S1. Every live branch in a reconciliation session is cross-applied against the shared staging branch, and a branch that was never applied is reported as missing rather than assumed integrated.

XC-S2. A divergence found on the staging branch is handed back to the two authors who caused it as one sized reconciliation task, and a divergence with no task is a defect the tests must catch.

XC-S3. A reconciliation task states the concept that diverged, both conflicting forms, and an estimate of the work to reconcile them, and a task missing any of those is rejected as incomplete.

XC-S4. Crosscurrent reconciles only the pairs it is handed by detection, and it never scans the whole fleet looking for new divergence on its own behalf.

XC-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.

## Round 1 obligations

Round 1 builds the recorded form of the reconciliation session the annex describes. No branch is live inside this repo, so the session runs over recorded files only. A session is handed two recorded branch states and one recorded divergence record, and it produces a cross-apply report and one sized reconciliation task per divergence. The handback is a document the tool prints, and the tool never edits a branch state, a record, or any branch.

A branch state is the recorded text one author's branch left behind for one interface. Its statement forms are the forms forkdrift defines for its recorded artifacts, extended with two session lines, an author line that names the author's role and a breaking line that marks one defined name as a stated breaking change on that branch. The format in brief is shown here once, and the obligations below state it as rules that can fail.

```
interface shard-store
author engineer-a

replicas = 3
timeout = 30
operation store = ack
label primary = node-a
breaking timeout
```

A meaningful line is a line that is neither blank nor a comment, and a comment line is a line whose first character that is not a space is the hash character. The first meaningful line names the interface and the second meaningful line names the author role. Every later meaningful line is a term line, an operation line, a label line, an excludes line, or a breaking line, in the shapes forkdrift's contract defines, and a branch state carries at most one excludes line. A breaking line is the word breaking followed by one name, and the name must be a term or an operation the same state defines. Term names, operation names, label names, interface names, and author roles are identifiers that start with a letter or an underscore and continue with letters, digits, underscores, dashes, or dots.

The divergence record is the report forkdrift's comparison prints, recorded to a file. A record is a sequence of blocks. Each block opens with a header line that names the interface, the diverged name, and the divergence kind, and the kind is one of term-value, result-type, or excluded-but-defined. Under the header, indented rows quote one defining line per side with the source each line came from, and an excluded-but-defined block whose name is defined both as a term and as an operation carries one further quoted row for the second defining line.

The sizing rule is mechanical and is stated here in full so the obligations can pin it. The score for one divergence is the number of defining lines quoted in its block, plus two when a result type is among the conflicting forms, plus three when either branch state marks the diverged name as a stated breaking change. A result type is among the conflicting forms when the kind is result-type or when any quoted defining line in the block is an operation line. The band is small at a score of two or less, medium at a score of three or four, and large at a score of five or more.

XC-1. A branch state parses only when its first meaningful line names exactly one interface, its second meaningful line names exactly one author role, and every later meaningful line matches one of the statement forms this contract defines, and a text that breaks that shape is not a branch state and is never staged or handed back over.

XC-2. A malformed branch state is refused with an error that names the source and the defect, and the refusal covers a missing interface line, a missing author line, an author line in any position but the second meaningful line, a meaningful line that matches no statement form, a duplicate term name or operation name or label name, an empty value, a breaking line that names a name the state does not define as a term or an operation, a second breaking line for one name, and a state that defines a name its own excludes line forbids, and no refusal ever surfaces as a traceback or as a partial report.

XC-3. A divergence record parses only when every block header names one interface, one diverged name, and one of the three divergence kinds, every quoted row names its side and its source, and the quoted lines are the statement forms the block's kind requires with the block's name, and a record that breaks that shape is refused with an error that names the record and the defect, and the refusal covers a header that matches no form, a kind the format does not know, a row that names no side or no source, a quoted line whose form or name does not match its block, an excluded-but-defined block with no excludes line among its rows or with defining lines from two different sources or whose excludes line and defining lines name the one source, two blocks for one name and kind, and blocks that name two different interfaces, and no refusal ever surfaces as a traceback or as a partial document.

XC-4. The two branch states and the divergence record in one session must all name the one interface, and a branch state or a record that names a different interface is refused with an error that names both interface names, because the cross-apply and the handback are defined only over the one interface the session is about.

XC-5. Two forms of one name are the same form under exactly the normalizations named here, namely leading and trailing whitespace is stripped from every line, runs of whitespace inside a value collapse to a single space, and a value that reads as a finite number on both sides compares by numeric worth rather than by its characters, and statement order never matters, and this rule is forkdrift's equality rule restated so the two tools agree on when two forms are the same form, and a difference one of these normalizations removes is never a conflict.

XC-6. The cross-apply of two branch states classifies every name at least one state defines as clean when both states define the name in the same namespaces and every paired form is the same form under XC-5, as conflicting when both states define the name and the name is not clean, and as one-sided when exactly one state defines the name, and label lines, excludes lines, and breaking lines never change the classification of any name.

XC-7. A name the divergence record reports as diverged that neither branch state defines is reported in the cross-apply report as missing, and a missing name is never counted as clean, conflicting, or one-sided, because a branch that was never applied to the staging area is reported as missing rather than assumed integrated.

XC-8. The cross-apply report carries the interface line, the authors line naming the two author roles in the order the states were handed in, and then the clean, conflicting, one-sided, and missing lines in that order, with every list of names sorted and every one-sided name marked with the side that defines it, and running the staging twice over the same inputs produces byte identical reports.

XC-9. Every divergence in the record produces exactly one reconciliation task, and a divergence with no task is a defect, and each task states the interface and the diverged name, both author roles, the divergence kind, every quoted line from the record block with its source, the size band and score with the three factors the score came from, and the clean-apply summary for the session, and a task that cannot state all of those is refused as incomplete with the missing piece named rather than printed in partial form.

XC-10. The size of a task is the sizing rule stated above and no other rule, and the task prints the number of quoted defining lines, whether a result type is among the conflicting forms, and whether either branch state marks the diverged name as a stated breaking change, so the band a reader sees can always be recomputed by hand from what the task prints.

XC-11. The handback prints its tasks sorted first by interface name and then by the diverged name and then by divergence kind, and running the handback twice over the same inputs produces byte identical output, and a record with no divergences produces no tasks and empty output, and the tasks in one handback never include a divergence the record does not carry.

XC-12. The command line tool exits with code 0 when the staging report or the handback document is produced and with code 2 when any input is malformed or a file cannot be read, and a refusal prints one error line to the error stream and never a traceback, and the tool writes nothing to the standard output but the report or the document it was asked for.

XC-13. The tool reads only the files it is handed on the command line and never scans a directory, a fleet, or any other source looking for more divergence, and it reconciles only the pairs the record it is handed carries, and the output for a session is the same whatever other files sit beside the inputs.

XC-14. The record is the session's evidence of divergence and crosscurrent never re-derives divergence, so it does not verify the quoted lines against the branch states, and when the staging cross-apply classifies a task's diverged name as clean the task prints a note stating that the staging and the record disagree, and the task still follows the record, and this trust is a stated limit of the round.
