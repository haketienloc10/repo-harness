# 0001 Read-Model is a First-Class-Conditional Orientation Layer

Date: 2026-06-17

## Status

Accepted

## Context

The harness has three reading planes (`KNOWLEDGE_INDEX.md`, `query stats`, and a
missing Read-Model). Adding orientation reads (`query status`, `query recap`)
raises two policy questions that change the workflow itself:

1. Should orientation be a new top-level command family, or live under `query`?
2. When must an agent run it — always (taxing cheap Q&A turns) or conditionally?

This touches the Stage-0 (Orient) contract and the command taxonomy, so it
requires a durable decision rather than an ad-hoc choice.

## Decision

1. **Taxonomy:** Pure read views go under `query`: `query status`, `query
   recap`. The `done-check` gate stays top-level because it carries exit-code /
   gate semantics (sibling of `story verify`/`verify-all`).
2. **First-class-conditional:** `query status` binds to the existing skip rule
   in `00-AGENTS.md §3` with the SAME predicate as the Execution Tracker. A turn
   that runs the 7-stage workflow (will write durable state) OR a question about
   _project state_ runs `query status` at Stage 0; a one-step content Q&A turn
   skips it. Ambiguous-but-touches-workflow ⇒ run (cost asymmetry: running
   spuriously costs a few hundred tokens; forgetting causes wrong-story
   selection / duplication). No new classification axis is created.
3. **Read-Model is a derived VIEW, not a store:** `query status`/`query recap`
   add no schema; every displayed byte traces back to existing tables.

## Alternatives Considered

1. `status` as a top-level verb — rejected: it is a pure view and belongs with
   `query stats`/`matrix`/`backlog`.
2. Always-run orientation — rejected: taxes one-step Q&A turns against the token
   budget for no benefit.
3. A new "needs-orientation" classification flag — rejected: duplicates the
   existing workflow/skip predicate.

## Consequences

Positive:

- One snapshot replaces manually stitching `matrix` + `backlog` + `traces`.
- Stage 0 gains a cheap, deterministic state digest without a new source of
  truth.

Tradeoffs:

- Stage 0 of `01-WORKFLOW.md` and `00-AGENTS.md §3` must be updated to reference
  the conditional read.

## Follow-Up

- Implement `query status` (US-005) and `query recap` (US-006).
- Update `00-AGENTS.md §3` and `01-WORKFLOW.md` Stage 0 (US-008).
