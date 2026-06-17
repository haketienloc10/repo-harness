# Overview

## Current Behavior

`story verify` persists only `last_verified_result` (pass/fail). The verify log
(stdout/stderr) is printed once and lost. `story.evidence` is free-text with no
hash, path, or re-checkable pointer. The Stage-4 Evidence Gate asks the agent to
read a log that does not survive the turn.

## Target Behavior

A durable `evidence` table stores structured pointers to local artifacts (logs,
diffs, screenshots, reports): `path`, `sha256`, `bytes`, `digest`, `command`,
`result`, `source`, `notes`, linked to a story and/or trace. Artifact bytes are
copied under `_harness/evidence/` (gitignored); the db keeps only the pointer +
hash + digest. `harness-cli evidence add` ingests an artifact; `harness-cli
evidence list` queries pointers.

## Affected Users

- Coding agents (Stage 4/5 evidence capture and Stage 0 read-back).
- Reviewers (see pointer + digest without raw files in git).

## Affected Product Docs

- `docs/product/read-model.md`
- `docs/decisions/0002-evidence-gitignore-pointer-store.md`

## Non-Goals

- Retention/pruning (`evidence prune --older-than`) — v2.
- Auto-capture wiring into `story verify` — that is US-004 (this story ships the
  table + `add`/`list` only).
- Cross-machine / CI artifact rebuild (explicitly traded away).
