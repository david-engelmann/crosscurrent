# crosscurrent

Receives one forkdrift report and works the divergence, staging records, applying and undoing patches in its own workspace, and building a task for each divergence.

Crosscurrent is a command line tool that reconciles recorded divergence. It reads two recorded branch states and one or more divergence records of the form forkdrift prints, and it cross-applies the two states in its own staging area to sort every name the states carry into the names that agree and the names that conflict. It prints either the staging report for one record or one sized reconciliation task for each divergence in a bundle of records. Every input is a recorded file and every run finishes from the files it is handed, and crosscurrent reconciles only the pairs it is handed. Finding divergence across the fleet is forkdrift's work and never moves here. That reconciliation work now lives in forkdrift as its reconcile subcommand.

[![CI](https://github.com/david-engelmann/crosscurrent/actions/workflows/ci.yml/badge.svg)](https://github.com/david-engelmann/crosscurrent/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

```console
$ ./target/debug/crosscurrent handback examples/branch-a.state examples/branch-b.state examples/divergences.txt
task shard-store store
authors engineer-a engineer-b
kind result-type
  left (examples/branch-a.state) operation store = ack
  right (examples/branch-b.state) operation store = receipt
size large score 7
factors definitions 2, result-type yes, breaking yes
staging clean 2, conflicting 1, one-sided 1, missing 0
```

## Install

Crosscurrent is a Rust crate that builds from source with cargo.

```sh
git clone https://github.com/david-engelmann/crosscurrent.git
cd crosscurrent
cargo build
```

The binary lands at target/debug/crosscurrent, and the test suite runs with `cargo test`.

## Quickstart

From the repository root, with the committed examples.

```sh
./target/debug/crosscurrent handback examples/branch-a.state examples/branch-b.state examples/divergences.txt
```

The command prints the handback at the top of this page, one task for the one divergence the record carries, with the size of the work and the staging counts of the pair behind it.

## Usage

### stage

`crosscurrent stage <left-state> <right-state> <record>` reads two recorded branch states and one divergence record and prints the cross-apply report. The report names the interface and its two authors and sorts the names of the interface into the names that agree on both sides, the names that conflict, the names only one side states, and the names a divergence expects that neither side states.

```console
$ ./target/debug/crosscurrent stage examples/branch-a.state examples/branch-b.state examples/divergences.txt
interface shard-store
authors engineer-a engineer-b
clean: replicas, timeout
conflicting: store
one-sided: cache-ttl (left only)
missing: none
```

### handback

`crosscurrent handback <left-state> <right-state> <record> [<record> ...]` prints one sized reconciliation task for each divergence over a bundle of records, as the block at the top of this page shows for a bundle of one. A divergence two records of the bundle carry is one task, identified by the set of its quoted rows and never by their order, and every task prints its quoted rows in the printed form order. Each task states the interface and the name that diverged and prints the rows the record quotes for it, both conflicting forms where the two sides state the name in the same form, and for a presence divergence the one-sided statement beside the other side's interface line, and it states the size of the work with the factors behind it and the staging counts of the pair.

Every command reads only the files it is handed. A branch state or a record that is malformed or cannot be read refuses with one error line and exit status 2, and a malformed invocation exits with status 2. A line that cannot be printed because its stream is already gone answers with status 120 instead, as XC-29 states.

## Status

The tool is complete at the surface its contract describes, with no stubbed or held back part, and the full CI gate runs on every change. Closed rounds 8 of 100. This repo is folded into forkdrift. Its session work stands as the reconcile subcommand in forkdrift, built in forkdrift PR 26 and closed in forkdrift PR 28. This repo receives no further rounds. The first committed example record names the branch states themselves as its sources, and no run of forkdrift over committed files regenerates it, because the author and breaking lines are grammar this repo alone defines. The presence and bundle example records name the committed tapes as their sources, and the presence record carries a timeout divergence between the tapes that the presence branch states do not carry.

## License

MIT. See [LICENSE](LICENSE).
