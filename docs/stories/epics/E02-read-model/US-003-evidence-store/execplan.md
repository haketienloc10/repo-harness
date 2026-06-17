# Exec Plan

## Goal

Ship the durable evidence store: migration 007, the `evidence` domain + repo
methods, and `evidence add` / `evidence list` CLI, with `_harness/evidence/`
gitignored.

## Scope

In scope:

- `_harness/schema/007-evidence.sql` (table + keep-last index + schema_version 7).
- `evidence add --kind --path [--story --trace --command --source --notes]`:
  hash, digest, copy-to-store, insert pointer.
- `evidence list [--story --trace --kind --json]`.
- `.gitignore` += `_harness/evidence/`.

Out of scope:

- Auto-capture on `story verify` (US-004).
- `done-check` evidence assertion (US-007).

## Risk Classification

Risk flags:

- Data model (new table).
- Audit/security (artifact paths, hashing).
- Weak proof (this is the proof-durability fix itself).

Hard gates:

- Data model / data ownership → covered by decision 0002.

## Work Phases

1. Discovery — confirm migration loader reads new sql by filename order (done).
2. Design — table columns, digest rules, store path scheme.
3. Validation planning — sha256 correctness, file placement, list filters.
4. Implementation — schema, domain (`EvidenceKind`, digest), infra (add/list),
   interface (CLI).
5. Verification — `cargo test` + CLI acceptance against a temp file.
6. Harness update — `.gitignore`, CLI reference (US-008).

## Stop Conditions

Pause for human confirmation if:

- The artifact would need to be committed to git (contradicts decision 0002).
- A binary `--path` requires reading bytes into memory beyond hashing.
- The store path scheme would collide across stories/traces.
