# US-006 Recap Rollup — `query recap` (P5)

## Status

planned

## Lane

normal

## Product Contract

`harness-cli query recap` prints a deterministic templated rollup over traces
(by story / epic prefix / since-date) so an agent digests "what was done" in a
few lines instead of reading dozens of trace rows. No semantic summary — pure
count/group rollup.

## Relevant Product Docs

- `docs/product/read-model.md`

## Acceptance Criteria

- `query recap --story <id>` outcome counts match `query traces` filtered by
  hand.
- `query recap --epic E02` aggregates every story whose id matches the prefix.
- `--since <YYYY-MM-DD>` bounds the window.
- Output rolls up: outcome counts, top files-touched by frequency, friction
  grouped by the 11 Components, decisions touched, intervention counts.
- `--json` is deterministic (same db → same output).
- Unit tests cover story rollup counts and json determinism.

## Design Notes

- Commands: `query recap [--story] [--epic] [--since] [--json]`
- Queries: aggregates `trace` (+ `intervention`); pure read.
- Tables: none new.
- Domain rules: deterministic; counting/grouping only, no LLM.
- UI surfaces: text + json.

## Validation

`scripts/bin/harness-cli story update --id US-006 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                       |
| ----------- | ---------------------------------------------------- |
| Unit        | `cargo test -p harness-cli` (counts + json)          |
| Integration | CLI: recap counts equal manual trace filter          |
| E2E         | n/a                                                  |
| Platform    | n/a                                                  |
| Release     | n/a                                                  |

## Harness Delta

Stage 0 / Stage 6 reference recap in US-008.

## Evidence

Add after validation.
