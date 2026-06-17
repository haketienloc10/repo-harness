# US-005 Read-Model / Session Brief — `query status` (P1)

## Status

planned

## Lane

normal

## Product Contract

`harness-cli query status` prints one ranked snapshot of current project state,
ordered by action priority, so an agent orients without stitching `matrix` +
`backlog` + `traces` by hand. Pure derived view — no schema.

## Relevant Product Docs

- `docs/product/read-model.md`
- `docs/decisions/0001-read-model-first-class-conditional.md`

## Acceptance Criteria

- `harness-cli query status` on an empty db prints empty sections and exits `0`
  (no crash).
- Sections in order: ĐANG LÀM (in_progress), CẦN PROOF
  (implemented but not passed / proof col 0), RESUME (partial/blocked traces +
  next_action), BACKLOG MỞ (open, high-risk first), INTERVENTION (recent),
  HOẠT ĐỘNG GẦN (recent traces) + a drift header line.
- Each section honors `--limit <n>` (default 5) and prints `(+N nữa)` when more
  rows exist ("no silent caps"). `--full` removes the cap.
- `--lane tiny|normal|high-risk` filters story-derived sections.
- `--json` returns an object with keys
  `active`/`needs_proof`/`resume`/`backlog`/`interventions`/`recent`; each
  element carries its source id.
- Unit tests cover empty db, limit+overflow marker, and json keys.

## Design Notes

- Commands: `query status [--json] [--lane] [--limit <n>] [--full]`
- Queries: see `docs/product/read-model.md` section→table mapping.
- API: CLI only
- Tables: reads `story`, `trace`, `backlog`, `intervention` + `next_action`
  (US-002); NO new tables.
- Domain rules: deterministic ordering by `created_at`/risk; drift header reuses
  audit entropy logic (one line, no full audit rerun).
- UI surfaces: text + json.

## Validation

`_harness/bin/harness-cli story update --id US-005 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                            |
| ----------- | --------------------------------------------------------- |
| Unit        | `cargo test -p harness-cli` (empty/limit/json)            |
| Integration | CLI: `query status --json` has all six section keys       |
| E2E         | n/a                                                       |
| Platform    | n/a                                                       |
| Release     | n/a                                                       |

## Harness Delta

Stage 0 (0b state digest) added in US-008.

## Evidence

Add after validation.
