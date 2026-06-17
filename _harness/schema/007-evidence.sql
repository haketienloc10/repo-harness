-- Harness v0 schema - migration 007
-- Durable pointer to local evidence artifacts (gitignored on disk).
-- The db keeps pointer + hash + digest; artifact bytes live under
-- _harness/evidence/ (see decision 0002).
CREATE TABLE evidence (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    story_id    TEXT REFERENCES story(id),
    trace_id    INTEGER REFERENCES trace(id),
    kind        TEXT NOT NULL
                CHECK(kind IN ('log','diff','screenshot','report','file')),
    path        TEXT NOT NULL,        -- local path (gitignored)
    sha256      TEXT NOT NULL,        -- content hash of the artifact
    bytes       INTEGER,
    digest      TEXT,                 -- short excerpt (head/tail) for quick reads
    command     TEXT,                 -- command that produced the artifact, if any
    result      TEXT                  -- for kind='log' from verify: pass/fail
                CHECK(result IN ('pass','fail') OR result IS NULL),
    source      TEXT NOT NULL DEFAULT 'agent'
                CHECK(source IN ('agent','human','ci','reviewer')),
    notes       TEXT
);
-- Dedup key for auto-capture "keep-last-per-story": each (story, kind, result)
-- keeps EXACTLY the newest row (the old row+file is removed before insert).
CREATE INDEX idx_evidence_keeplast ON evidence(story_id, kind, result);
INSERT INTO schema_version (version) VALUES (7);
