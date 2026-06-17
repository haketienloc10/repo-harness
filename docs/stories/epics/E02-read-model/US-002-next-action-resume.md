# US-002 Next-Action / Resume Continuity (P3)

## Status

planned

## Lane

normal

## Product Contract

A trace with outcome `partial`, `blocked`, or `failed` carries a "continue from
here" pointer so a later session does not lose WIP context. `story.next_action`
is the live pointer; `trace.next_action` is the immutable record at trace time.

## Relevant Product Docs

- `docs/product/read-model.md`
- `docs/decisions/0001-read-model-first-class-conditional.md`

## Acceptance Criteria

- Migration `006-next-action.sql` adds `story.next_action`,
  `story.next_action_at`, `trace.next_action`; `schema_version` row 6 inserted.
- `harness-cli trace --outcome blocked` without `--next-action` exits `!=0` with
  a clear message (same for `partial`, `failed`).
- `harness-cli trace --outcome partial --story US-X --next-action "..."` writes
  `trace.next_action` and `story.next_action` (+ `next_action_at`).
- `harness-cli trace --outcome completed --story US-X` clears
  `story.next_action` to NULL.
- `harness-cli story update --id <id> --next-action "..."` sets the live pointer
  and `next_action_at`.
- Unit tests cover the enforce-on-unfinished and clear-on-completed paths.

## Design Notes

- Commands: `trace --next-action`, `story update --next-action`
- Queries: surfaced later by `query status` RESUME (US-005)
- API: CLI only
- Tables: `story.next_action TEXT`, `story.next_action_at TEXT`,
  `trace.next_action TEXT`
- Domain rules: enforce `--next-action` for `partial|blocked|failed`; clear
  `story.next_action` for `completed` when the trace has `--story`.
- UI surfaces: CLI output only

## Validation

`scripts/bin/harness-cli story update --id US-002 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                              |
| ----------- | ----------------------------------------------------------- |
| Unit        | `cargo test -p harness-cli` (enforce + clear paths)         |
| Integration | CLI: blocked-without-next-action rejected; completed clears |
| E2E         | n/a                                                         |
| Platform    | n/a                                                         |
| Release     | n/a                                                         |

## Harness Delta

Adds resume continuity; `docs/TRACE_SPEC.md` tier rules updated in US-008.

## Evidence

Add after validation.
