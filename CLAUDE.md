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

- ~~Never write or edit source code unless user explicitly asks.~~ **Superseded:** user is new to Rust, wants code provided as chunks for reading/learning. AI gives code in chat (or writes files when asked). User does not write Rust from scratch.
- No premature abstraction.
- No silent stack choice — surface options.
- No skipping the "why broken at scale" step before introducing fix.

## Response format

- Chat replies ≤25 lines.

## Workflow

- User asks "next step" → AI gives **one** small bite-sized chunk to implement.
- Chunk = 15–45 min of work. One concept. Clear done-condition.
- **User is new to Rust.** AI provides the actual code each step — user copies, reads, asks questions. Do NOT tell user to "write it yourself". The learning loop is: read code → ask why → understand → next chunk.
- For each code drop: explain **why** (concept, tradeoffs), then **code**, then **what to watch / try**.
- **Code chunks = section-level w/ left-margin `*` markers.** Show each logical section (imports, struct, full updated fn) as a self-contained block. Prefix new/changed lines with `* `, unchanged lines with `  ` (two spaces) — uniform across the block. User scans left edge to spot new lines. Strip prefixes before pasting. Never full-file replacements.
- Anticipate beginner Rust questions (ownership, async, lifetimes, traits) — flag tricky bits inline.
- After user reports done → update **Progress** section below. Then wait for next ask.
- New session → AI reads Progress section first. No re-exploring.

## Progress

**Current phase:** Phase 1 — identity & auth
**Last completed step:** Phase 1 Step 1 — `sqlx-cli` installed, `create_users` migration applied (UUID PK, CITEXT username unique, password_hash, created_at); pgweb added to docker-compose on :8081
**Next step:** Phase 1 Step 2 — `POST /signup` handler: argon2 password hash + UUIDv7 gen + insert row, return 201 + user id
**Files in flight:** `Cargo.toml`, `shared/`, `server/`, `client/`, `.gitignore`, `docker-compose.yml`, `.env`, `migrations/`, `justfile`
**Open decisions:**
- Frontend framework (leptos vs dioxus vs yew) — defer to phase 10
- Queue (NATS vs Kafka) — defer to phase 4

**Log:**
- 2026-05-22 — drafted CLAUDE.md + README.md roadmap (12 phases, 140–225 hrs total)
- 2026-05-22 — added stretch goals S1–S7 to README (multi-region, E2EE, WebRTC, search, federation, abuse, compliance)
- 2026-05-22 — Phase 0 Step 1 done: Cargo workspace (shared/server/client) builds green w/ resolver = "3"
- 2026-05-22 — Phase 0 Step 2 done: docker-compose w/ Postgres 16 + Redis 7, both healthchecks passing
- 2026-05-22 — Phase 0 Step 3 done: axum 0.8 server, `/health` returns "ok" (deps: axum 0.8.9, tokio 1.52)
- 2026-05-23 — Phase 0 Step 4 done: sqlx 0.9 PgPool, `/ready` queries `SELECT 1`, returns 200/503, dotenvy + justfile. Pool auto-reconnects after pg restart.
- 2026-05-23 — Phase 0 Step 5 done: redis 1.2 ConnectionManager (features: aio, tokio-comp, connection-manager), `/ready` pings pg + redis, verified 200→503→200 on redis stop/start. **Phase 0 complete.**
- 2026-05-23 — Phase 1 Step 1 done: sqlx-cli installed, `create_users` migration (UUID PK, CITEXT username UNIQUE, password_hash, created_at, idx on created_at DESC, citext extension), reversibility verified. pgweb service added to docker-compose (port 8081, depends_on postgres healthy).
