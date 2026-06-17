-- Harness v0 schema - migration 006
-- Resume hint for WIP continuity across sessions.
-- story.next_action is the LIVE pointer (always "the current next step");
-- trace.next_action is the IMMUTABLE record captured at trace time.

ALTER TABLE story ADD COLUMN next_action TEXT;
ALTER TABLE story ADD COLUMN next_action_at TEXT;
ALTER TABLE trace ADD COLUMN next_action TEXT;

INSERT INTO schema_version (version) VALUES (6);
