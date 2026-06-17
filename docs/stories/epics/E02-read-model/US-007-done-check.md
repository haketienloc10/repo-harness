# US-007 Done-Check Gate — `harness-cli done-check` (P4)

## Status

planned

## Lane

normal

## Product Contract

`harness-cli done-check` packages the Stage-7 "done" conditions into one
lane-aware gate with an exit code, so an agent cannot self-assert completion.
Aggregator only (read + exit code) — no new store.

## Relevant Product Docs

- `docs/product/read-model.md`
- `docs/decisions/0002-evidence-gitignore-pointer-store.md`

## Acceptance Criteria

- `done-check --story <id>` exits `0` when all lane checks pass, `1` when any
  fails; prints a `✔/✘` checklist with a reason per fail.
- Lane-aware checks (per proposal §5.2):
  - all lanes: ≥1 trace links the story/intake.
  - normal+high-risk: `story.status=='implemented'`; `verify_command` set and
    `last_verified_result=='pass'`; an evidence `log` pass row exists (P2);
    declared proof columns all `1`; `story.next_action` cleared.
  - high-risk only: 4 story-packet anchors exist
    (overview/execplan/design/validation).
- `--json` returns `{passed: bool, checks: [...]}`.
- Story `tiny` needing only a trace exits `0`.
- Unit tests cover normal-missing-proof (exit 1) and tiny-trace-only (exit 0).

## Design Notes

- Commands: `done-check [--story <id>] [--intake <id>] [--json]`
- Queries: reads `story`, `trace`, `evidence`, plus filesystem for high-risk
  anchors.
- Tables: none new.
- Domain rules: lane drives which checks apply; exit code is the gate.
- UI surfaces: text checklist + json.

## Validation

`_harness/bin/harness-cli story update --id US-007 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                        |
| ----------- | ----------------------------------------------------- |
| Unit        | `cargo test -p harness-cli` (lane gates)              |
| Integration | CLI: normal story missing proof exits 1               |
| E2E         | n/a                                                   |
| Platform    | n/a                                                   |
| Release     | n/a                                                   |

## Harness Delta

Stage 7 rewritten to mandate done-check in US-008.

## Evidence

Add after validation.
