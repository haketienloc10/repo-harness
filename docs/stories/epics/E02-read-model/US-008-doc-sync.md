# US-008 Documentation Sync for Read-Model & Evidence

## Status

planned

## Lane

normal

## Product Contract

The harness operating docs describe the new read-model, evidence, next-action,
and done-check capabilities so agents discover and use them through the normal
Stage-0..7 workflow.

## Relevant Product Docs

- `docs/product/read-model.md`

## Acceptance Criteria

- `_harness/00-AGENTS.md §3`: `query status` shares the skip predicate with the
  Execution Tracker; the Read-Model layer is named.
- `_harness/01-WORKFLOW.md`: Stage 0 gains step `0b` (conditional state digest);
  Stage 4 Evidence Gate references auto-capture; Stage 5 references next_action
  enforcement; Stage 7 mandates `done-check`.
- `_harness/03-CLI_REFERENCE.md`: syntax for `query status`/`query recap`/
  `evidence add|list`/`done-check`/`trace --next-action`/`story update
  --next-action`.
- `docs/CLI_REFERENCE.md`: deep semantics + examples per command.
- `docs/TRACE_SPEC.md`: next_action enforcement by tier; evidence id in notes.
- `docs/HARNESS_COMPONENTS.md`: Observability/Verification status + file
  inventory updated.
- `.gitignore`: `_harness/evidence/` added.
- `docs/proposals/2026-06-17-read-model-evidence-upgrade.md` frozen (status note
  pointing to the sliced stories).

## Design Notes

- Commands: docs only
- Tables: none
- Domain rules: keep `01-WORKFLOW.md` token-budget note and `CONTEXT_RULES.md`
  in sync.
- UI surfaces: docs.

## Validation

`scripts/bin/harness-cli story update --id US-008 --unit 0 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                                          |
| ----------- | ------------------------------------------------------- |
| Unit        | n/a (docs)                                              |
| Integration | `harness-cli knowledge check` passes; help text matches |
| E2E         | n/a                                                     |
| Platform    | n/a                                                     |
| Release     | n/a                                                     |

## Harness Delta

This story IS the harness delta for the whole initiative.

## Evidence

Add after validation.
