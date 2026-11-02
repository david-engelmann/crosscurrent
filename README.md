# crosscurrent

A reconciliation protocol for parallel agent work, where every agent's diff is continuously cross-applied against a shared staging branch and divergence is handed back to the pair that caused it while both are still live.

## What this is

Worktree tools isolate agents well and do nothing about semantic drift, so everything compiles and merges cleanly and the integration tax arrives all at once. Crosscurrent runs three mechanisms instead of a merge tool. A staging branch continuously absorbs every live agent's diff, so integration is a stream and not an event. A divergence detector works above the text level, flagging two incompatible contracts for one interface or a helper written twice under two names. And the conflict is handed back to the pair that created it as a sized reconciliation task, never saved up for a human merge day. The boundary with its sibling is one sentence. Forkdrift finds the divergence across the fleet, crosscurrent reconciles the pairs it catches.

## Status

Setup slice in progress. The build has not started. Closed rounds 0 of 100. The contract is a stub and is not yet met. Part of the impatient programming portfolio, in the Fleet Operations arc.

## Licence

MIT. See LICENSE.
