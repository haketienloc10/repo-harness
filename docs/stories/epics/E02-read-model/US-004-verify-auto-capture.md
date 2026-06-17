# US-004 Auto-Capture Evidence on `story verify` (P2)

## Status

planned

## Lane

normal

## Product Contract

`harness-cli story verify <id>` captures its stdout+stderr into the evidence
store by default, so a proof boolean is always backed by a fresh durable log.
Capture is clean-by-mechanism (dedup), not a burden on agent discipline.

## Relevant Product Docs

- `docs/product/read-model.md`
- `docs/decisions/0002-evidence-gitignore-pointer-store.md`

## Acceptance Criteria

- `story verify <id>` writes a `kind='log'` evidence row with `command`,
  `result` (`pass|fail`), `source='agent'`, linked `story_id`, and prints the
  evidence id.
- Dedup keep-last-per-`(story_id,'log',result)`: the prior row + its file are
  removed before inserting the new one.
- Exception: when result transitions `fail→pass`, both rows are kept.
- Content dedup: identical `sha256` ⇒ no new file/row, only `created_at`
  refreshed.
- `--no-capture` skips evidence writing for draft runs.
- A failing verify still produces an evidence `log` containing the error output.
- Unit tests cover capture-on-pass, keep-last replacement, fail→pass retention,
  and `--no-capture`.

## Design Notes

- Commands: `story verify [--no-capture]`
- Queries: read by `done-check` (US-007) and `query status` CĐ PROOF (US-005)
- Tables: writes `evidence` (US-003)
- Domain rules: see `docs/decisions/0002-...`; capture runs after the verify
  result is known.
- UI surfaces: CLI prints the captured evidence id.

## Validation

`_harness/bin/harness-cli story update --id US-004 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                          |
| ----------- | ------------------------------------------------------- |
| Unit        | `cargo test -p harness-cli` (capture/dedup/retention)   |
| Integration | CLI: verify twice → one row; fail then pass → two rows  |
| E2E         | n/a                                                     |
| Platform    | n/a                                                     |
| Release     | n/a                                                     |

## Harness Delta

Stage 4 Evidence Gate updated in US-008.

## Evidence

Add after validation.
