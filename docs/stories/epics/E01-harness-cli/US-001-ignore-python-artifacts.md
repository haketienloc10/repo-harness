# US-001 Ignore Python Artifacts in Knowledge Index

## Status

implemented

## Lane

normal

## Product Contract

`harness-cli knowledge scaffold` and `harness-cli knowledge check` ignore common
generated Python artifacts while still reporting real repository structure drift.

## Relevant Product Docs

- `docs/product/README.md`
- `harness-cli-backlog-1-ignore-python-artifacts.md`

## Acceptance Criteria

- With `data/__pycache__/` present, `harness-cli knowledge check` exits `0` if
  the index is otherwise current.
- With `.pytest_cache/` present, `harness-cli knowledge check` exits `0` if the
  index is otherwise current.
- `harness-cli knowledge scaffold` does not add generated Python artifact entries
  such as `__pycache__` or `.pytest_cache` to `docs/KNOWLEDGE_INDEX.md`.
- Real, non-ignored top-level entries and second-level key subdirectories are
  still detected as stale when missing from `docs/KNOWLEDGE_INDEX.md`.
- Tests cover ignored generated artifacts and a real missing directory.

## Design Notes

- Commands: `harness-cli knowledge scaffold`, `harness-cli knowledge check`
- Queries: none
- API: no external API changes
- Tables: none
- Domain rules: ignore common Python cache/build/coverage artifacts before
  collecting top-level entries and second-level key subdirectories.
- UI surfaces: CLI output only

## Validation

When updating durable proof status, use numeric booleans:
`scripts/bin/harness-cli story update --id US-001 --unit 1 --integration 1 --e2e 0 --platform 0`.

| Layer       | Expected proof                |
| ----------- | ----------------------------- |
| Unit        | `cargo test -p harness-cli`   |
| Integration | `scripts/bin/harness-cli ...` |
| E2E         | n/a                           |
| Platform    | n/a                           |
| Release     | n/a                           |

## Harness Delta

This story implements backlog item `1`.

## Evidence

- `scripts/bin/harness-cli story verify US-001` — pass (`cargo test -p
  harness-cli`, 43 tests).
- Temp-repo CLI acceptance: `knowledge scaffold` omitted `__pycache__` and
  `.pytest_cache`, `knowledge check` passed with those artifacts present, and a
  real `real_missing/` directory was still reported stale.
