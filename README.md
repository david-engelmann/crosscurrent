# crosscurrent

A reconciliation protocol for parallel agent work, where every agent's diff is continuously cross-applied against a shared staging branch and divergence is handed back to the pair that caused it while both are still live.

## What this is

Worktree tools isolate agents well and do nothing about semantic drift, so everything compiles and merges cleanly and the integration tax arrives all at once. Crosscurrent runs two mechanisms instead of a merge tool. A staging branch continuously absorbs every live agent's diff, so integration is a stream and not an event. And each divergence that detection hands it is returned to the pair that created it as a sized reconciliation task, never saved up for a human merge day. Detection itself sits with its sibling. Forkdrift's detector works above the text level, flagging two incompatible contracts for one interface or a helper written twice under two names. The boundary with its sibling is one sentence. Forkdrift finds the divergence across the fleet, crosscurrent reconciles the pairs it catches.

## Status

Round 2 built and in review. The staging model and the handback over recorded branch states are in place under contract obligations XC-1 to XC-17, reading the divergence records forkdrift emits and returning one sized reconciliation task per divergence to the two authors. Closed rounds 1 of 100. Part of the impatient programming portfolio, in the Fleet Operations arc.

## Licence

MIT. See LICENSE.
