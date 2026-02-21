# Chat Two-Agent Split Plan

Last updated: 2026-02-21

This plan defines how two agents execute P0 chat work in parallel with minimal conflicts.
Canonical task definitions stay in `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md`.

## Mandatory Worktree Isolation

Two agents must use separate git worktrees and separate branches.
Do not run both agents from the same checkout.

Example setup from repo root:

```bash
git worktree add ../poziomki-rs-agent-a -b chat-p0-backend
git worktree add ../poziomki-rs-agent-b -b chat-p0-mobile
```

Recommended mapping:
- Agent A works only in `../poziomki-rs-agent-a`
- Agent B works only in `../poziomki-rs-agent-b`

Worktree rules:
- Each agent commits only from its own worktree.
- Rebase/merge main independently in each worktree.
- Never copy unstaged files between worktrees.

## Ownership

| Agent | Scope | Must not edit |
|---|---|---|
| Agent A (Backend Authority) | `backend/src/controllers/migration_api/*` and backend tests | `mobile/*` |
| Agent B (Mobile Integration + UX) | `mobile/shared/*`, `mobile/composeApp/*` and mobile tests | `backend/src/controllers/migration_api/*` |

## P0 Work Split

| Checklist ID | Owner | Notes |
|---|---|---|
| `P0-SEC-01` | Agent A | Return `Cache-Control: no-store` on `/api/v1/matrix/session`. |
| `P0-EVT-01` | Agent A | Backend event room resolver/creator, auth enforced. |
| `P0-EVT-02` | Agent B | Replace mobile-local event room authority with backend endpoint. |
| `P0-EVT-03` | Agent B | Enforce attendee-only entry at event detail and event chat route level. |
| `P0-EVT-04` | Shared (A primary, B integration) | A defines backend behavior; B wires UI/client behavior and errors. |
| `P0-DM-01` | Agent A | Backend canonical DM resolver/creator (one room per pair). |
| `P0-DM-02` | Agent B | Route “Wiadomość” flow through backend canonical DM mapping. |
| `P0-UX-01` | Agent B | Correct room classification in `Wiadomości -> Wydarzenia`. |

## API Contract First Rule

Before Agent B finalizes mobile integration, Agent A publishes endpoint contracts:

1. Event room mapping endpoint:
- route, auth rules, request params, success payload, error payload

2. DM canonical endpoint:
- route, input (`userId` pair target), success payload, error payload

3. Membership sync behavior:
- join/leave side effects and error cases

Until contract is frozen, Agent B uses temporary stubs/adapters.

## PR and Merge Order

1. PR-A1 (Agent A): `P0-SEC-01` + endpoint skeletons for `P0-EVT-01` and `P0-DM-01`
2. PR-B1 (Agent B): UI gating + room classification changes that do not require final backend payloads
3. PR-A2 (Agent A): finalize event + DM canonical logic and tests
4. PR-B2 (Agent B): switch from stubs to final backend API, complete `P0-EVT-02` and `P0-DM-02`
5. PR-AB3 (joint): complete `P0-EVT-04`, run full verification matrix

## Doc Pack Per Agent

Both agents read first:
1. `docs/chat/README.md`
2. `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md`
3. `docs/chat/AGENT_SPLIT_PLAN.md`

Agent A (Backend) additionally reads:
1. `docs/chat/MATRIX_STATUS.md`
2. `docs/chat/MATRIX_REFERENCE.md` (sections relevant to rooms/invite/join/session handling)
3. `matrix_consensus.md` (historical context only)

Agent B (Mobile) additionally reads:
1. `docs/chat/MATRIX_STATUS.md`
2. `docs/chat/MATRIX_REFERENCE.md` (sections relevant to DM flow, room list classification, invite/join UX)
3. `matrix_consensus.md` (historical context only)

## Conflict Avoidance Rules

- No cross-layer edits unless explicitly agreed in PR description.
- If a shared model must change (DTO/API schema), Agent A updates contract first.
- Agent B consumes contract changes in a dedicated follow-up commit.
- Keep docs edits in one dedicated docs PR after P0 merge, except contract snippets needed for implementation.

## Verification Responsibility

| Check | Owner |
|---|---|
| Session cache header (`no-store`) | Agent A |
| Event canonical room consistency | Agent A |
| DM canonical room consistency | Agent A |
| Non-attendee access blocking | Agent B |
| Dual-surface event chat consistency | Agent B |
| Join/leave membership sync end-to-end | Shared |
| Race-condition sanity (parallel opens) | Shared |

Use the verification matrix in `docs/chat/CHAT_IMPLEMENTATION_CHECKLIST.md`.
