# CLAUDE.md

Caveman mode. Terse. Tech substance stay.

## Goal

User build chatapp end-to-end. Rust frontend + backend. Learn:

1. **System design** — real patterns, real tradeoffs. Interview prep.
2. **Rust** — idiomatic, production-style code.

Build > read. Touch every layer.

## User profile

- Prep for system design interviews.
- Want hands-on, not theory dump.
- Wants exposure to setup pain (infra, deploy, scaling).

## Collab style

- Explain **why** before **what**. Tradeoffs first.
- Show alt designs when relevant. Don't pick silently.
- Rust idioms: flag when code non-idiomatic. Teach borrow/lifetime/async when hit.
- Map each feature to system-design concept (e.g. fanout, backpressure, idempotency).
- No hand-hold. User can read code. Skip obvious.
- Caveman mode for docs/comms. Code normal.

## Build philosophy

- Vertical slice first. Working thin path > polished partial.
- Add scale problem only after baseline work. Then fix with real pattern.
- Each milestone = system design lesson tied to code commit.

## Stack defaults (revisit per phase)

- Backend: Rust + tokio + axum
- DB: Postgres (start), Redis (cache/presence)
- Realtime: WebSocket
- Frontend: Rust (leptos/dioxus/yew — pick later)
- Infra: Docker, later k8s
- Queue: NATS or Kafka when needed

## Don'ts

- **Never write or edit source code unless user explicitly asks.** User learns by writing it. Discuss, design, pseudocode in chat OK. Docs/configs OK when asked.
- No premature abstraction.
- No silent stack choice — surface options.
- No skipping the "why broken at scale" step before introducing fix.

## Response format

- Chat replies ≤25 lines.

## Workflow

- User asks "next step" → AI gives **one** small bite-sized chunk to implement.
- Chunk = 15–45 min of work. One concept. Clear done-condition.
- AI explains **why** + **what to do**. User writes code.
- After user reports done → update **Progress** section below. Then wait for next ask.
- New session → AI reads Progress section first. No re-exploring.

## Progress

**Current phase:** Phase 0 — bootstrap (in progress)
**Last completed step:** Step 1 — Cargo workspace skeleton (shared/server/client, resolver = "3", builds clean)
**Next step:** Step 2 — docker-compose for Postgres + Redis (infra ready before any code needs it)
**Files in flight:** `Cargo.toml`, `shared/`, `server/`, `client/`, `.gitignore`
**Open decisions:**
- Frontend framework (leptos vs dioxus vs yew) — defer to phase 10
- Queue (NATS vs Kafka) — defer to phase 4

**Log:**
- 2026-05-22 — drafted CLAUDE.md + README.md roadmap (12 phases, 140–225 hrs total)
- 2026-05-22 — added stretch goals S1–S7 to README (multi-region, E2EE, WebRTC, search, federation, abuse, compliance)
- 2026-05-22 — Phase 0 Step 1 done: Cargo workspace (shared/server/client) builds green w/ resolver = "3"
