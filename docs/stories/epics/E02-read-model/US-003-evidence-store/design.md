# Design

## Domain Model

- `EvidenceKind` value object: `log | diff | screenshot | report | file`.
- `EvidenceSource`: `agent | human | ci | reviewer` (default `agent`).
- `result`: `pass | fail | NULL` (set for verify logs).
- `digest`: for text kinds (`log`/`diff`/`report`) = first N + last N lines
  (default 20+20); for binary kinds (`screenshot`/`file`) = `<mime/size>`
  metadata only.
- `sha256` + `bytes` computed from the artifact bytes.

## Application Flow

`evidence add`:

1. Read `--path`, compute `sha256` + `bytes`.
2. Build `digest` per kind.
3. Copy artifact into
   `_harness/evidence/<story-or-trace-key>/<created_at>-<kind><ext>`.
4. Insert a row pointing at the normalized path.

`evidence list`: filtered SELECT (`--story`/`--trace`/`--kind`), text or json.

## Interface Contract

```text
harness-cli evidence add --kind <k> --path <p>
   [--story <id>] [--trace <id>] [--command "<cmd>"] [--source <s>] [--notes "<t>"]
harness-cli evidence list [--story <id>] [--trace <id>] [--kind <k>] [--json]
```

Errors: missing path, unreadable file, invalid kind/source, neither story nor
trace supplied (key needs one).

## Data Model

```sql
CREATE TABLE evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    story_id TEXT REFERENCES story(id),
    trace_id INTEGER REFERENCES trace(id),
    kind TEXT NOT NULL CHECK(kind IN ('log','diff','screenshot','report','file')),
    path TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    bytes INTEGER,
    digest TEXT,
    command TEXT,
    result TEXT CHECK(result IN ('pass','fail') OR result IS NULL),
    source TEXT NOT NULL DEFAULT 'agent'
        CHECK(source IN ('agent','human','ci','reviewer')),
    notes TEXT
);
CREATE INDEX idx_evidence_keeplast ON evidence(story_id, kind, result);
INSERT INTO schema_version (version) VALUES (7);
```

Retention: none in v1 (keep-last dedup is added with auto-capture in US-004).

## UI / Platform Impact

CLI only. New gitignored directory `_harness/evidence/`.

## Observability

The store IS the observability artifact. `query status` CẦN PROOF and
`done-check` (later stories) read these rows.

## Alternatives Considered

1. Store artifacts inline as a BLOB in sqlite — rejected: bloats the db, loses
   the "clean repo / local file" property.
2. Keep only free-text `story.evidence` — rejected: not re-checkable (the
   original problem).
