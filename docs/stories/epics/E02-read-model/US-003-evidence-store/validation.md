# Validation

## Proof Strategy

The store must produce a correct, re-checkable pointer: the recorded `sha256`
must equal an independent `sha256sum` of the artifact, the file must appear under
`_harness/evidence/`, and `_harness/evidence/` must be gitignored. `evidence
list` must round-trip what `add` wrote.

## Test Plan

| Layer       | Cases                                                              |
| ----------- | ----------------------------------------------------------------- |
| Unit        | sha256/bytes correctness; digest head+tail for text; binary digest = metadata; kind/source validation; key path scheme |
| Integration | `evidence add --kind log --path <f>` → row + file present; `evidence list --story <id> --json` round-trips |
| E2E         | n/a                                                               |
| Platform    | n/a                                                               |
| Performance | n/a                                                               |
| Logs/Audit  | recorded `sha256` equals `sha256sum <f>`                          |

## Fixtures

- A small text file (multi-line) to exercise head+tail digest.
- A tiny binary file to exercise metadata-only digest.

## Commands

```text
scripts/bin/harness-cli evidence add --kind log --path /tmp/sample.log --story US-003
scripts/bin/harness-cli evidence list --story US-003 --json
cargo test -p harness-cli
```

## Acceptance Evidence

Add results after verification.
