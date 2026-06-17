# 0002 Evidence Store Keeps Pointers; Artifacts Are Gitignored

Date: 2026-06-17

## Status

Accepted

## Context

`story verify` only persists pass/fail (1/0); stdout/stderr "evaporates" after
the agent reads it. The Stage-4 "Evidence Gate" requires reading the log, but
the log is not durable — contradicting the goal "have evidence, have artifacts".
This decision settles where evidence artifacts live and how the proof model
references them, which touches the verification/proof contract and data
ownership (a Hard Gate), so it needs a durable record.

## Decision

1. **Db keeps pointer + hash + digest; artifact bytes live locally and are
   gitignored.** A new `evidence` table (migration 007) stores `path`, `sha256`,
   `bytes`, `digest`, `command`, `result`, `source`. Files are copied under
   `_harness/evidence/<key>/...` and `_harness/evidence/` is added to
   `.gitignore`.
2. **Auto-capture is default-on with keep-last-per-story.** `story verify`
   writes its stdout+stderr as a `kind='log'` evidence row by default. Before
   insert it deletes the prior row+file for the same `(story_id, kind, result)`
   (keep only the newest), EXCEPT when the result transitions `fail→pass` (keep
   both, preserving the fixed-it evidence). Identical `sha256` ⇒ no new file/row,
   just refresh `created_at`. `--no-capture` is the draft-run escape hatch.
3. **`story.evidence` free-text is retained** for backward compatibility; the
   new table is the structured source that `status`/`done-check`/`recap` read.

## Alternatives Considered

1. Commit artifacts into git — rejected: bloats the repo with logs/screenshots.
2. Opt-in capture (`--capture` flag) — rejected: relies on agent discipline,
   which is the exact failure mode being fixed.
3. Keep every verify log (no dedup) — rejected: evidence store grows unbounded
   across dev loops.

## Consequences

Positive:

- A proof boolean `1` always has a fresh log backing it; the Stage-7 done-check
  can enforce evidence without friction.
- Repo stays clean; reviewers see pointers, not raw files.

Tradeoffs:

- Artifacts do not rebuild on another machine / CI (accepted with the user).
- Retention/pruning (`evidence prune --older-than`) is out of scope for v1.

## Follow-Up

- Implement migration 007 + `evidence add/list` (US-003) and auto-capture
  (US-004).
- Wire evidence presence into `done-check` (US-007).
