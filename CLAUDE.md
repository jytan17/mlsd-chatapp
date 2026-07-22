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
- **Code chunks = section-level w/ left-margin diff markers.** Show each logical section (imports, struct, full updated fn) as a self-contained block. Prefix: `+ ` added, `- ` removed, `~ ` modified in place, `  ` (two spaces) unchanged. Uniform across block. User scans left edge to spot changes. Strip prefixes before pasting. Never full-file replacements.
- Anticipate beginner Rust questions (ownership, async, lifetimes, traits) — flag tricky bits inline.
- After user reports done → update **Progress** section below. Then wait for next ask.
- New session → AI reads Progress section first, then reads **all source files** (`server/src/*.rs`, `shared/src/*.rs`, `client/src/*.rs`), `Cargo.toml`s, `docker-compose.yml`, `migrations/*.sql`, `justfile`. Builds full mental model before answering. No "let me check" mid-conversation.

## Progress

**Current phase:** Phase 1 — auth
**Last completed step:** Phase 1 Step 3 — `POST /login` w/ password verify + redis session (2026-07-21).
**Next step:** Phase 1 Step 4 — auth extractor + protected `GET /me`.
**Files in flight:** `Cargo.toml`, `{shared,server,client}/Cargo.toml`, `{shared,client}/src/lib.rs`, `server/src/main.rs`, `docker-compose.yml`, `.env`, `justfile`
**Open decisions:**
- Frontend framework (leptos vs dioxus vs yew) — defer to phase 10
- Queue (NATS vs Kafka) — defer to phase 4

**Log:**
- 2026-07-20 — Phase 0 Step 5 done: redis 0.27 ConnectionManager w/ `ConnectionManagerConfig::set_connection_timeout(2s) + set_response_timeout(2s)`, `/ready` pings both pg + redis (`redis::cmd("PING").query_async::<String>` == "PONG"), 200/503 matrix verified. **Phase 0 complete.**
- 2026-07-21 — Phase 1 Step 3 done: `POST /login` in `login.rs`, argon2 `verify_password` w/ `DUMMY_HASH` fallback on user-miss (timing parity), 32-byte OsRng token → base64url, redis `SETEX session:<token> 2592000 <user_id>`. Deps: base64 0.22 (rand skipped — reused argon2's OsRng).
- 2026-07-21 — Phase 1 Step 2 done: `POST /signup` in new `signup.rs` module, argon2id via `Argon2::default()` + random `SaltString`, UUIDv7 ids, validation (username 3..32, password ≥8), 201/409 on unique violation/400. Deps added: argon2 0.5, uuid 1 w/ v7+serde, serde/serde_json, sqlx feature `uuid`.
- 2026-07-20 — Phase 1 Step 1 done: `create_users` migration — UUID pk, CITEXT username UNIQUE, password_hash TEXT NOT NULL, created_at TIMESTAMPTZ default now(), idx on created_at DESC, citext extension. Reversible (up/down). Added `migrate` + `migrate-revert` to justfile.
- 2026-07-16 — Phase 0 Step 4 done: sqlx 0.8 PgPool w/ `PgPoolOptions::acquire_timeout(2s)`, `/ready` runs `SELECT 1` → 200/503, dotenvy loads `.env` (`DATABASE_URL`), justfile w/ run/db-up/db-down. Pool auto-recovers after pg pause/unpause. Fast-fail readiness (no 30s default hang).
- 2026-07-15 — Phase 0 Step 3 done: axum 0.8 server on 0.0.0.0:3000, `/health` returns "ok" (deps: axum 0.8, tokio 1 full).
- 2026-07-14 — Phase 0 Step 2 done: docker-compose w/ Postgres 16 + Redis 7, both healthchecks passing (named volumes pgdata/redisdata).
- 2026-07-14 — Phase 0 Step 1 done: Cargo workspace (shared/server/client) builds green w/ resolver = "3", edition = "2024".
- 2026-07-14 — **full reset.** Nuked src (server/client/shared), migrations, Cargo files, docker-compose, justfile, .env, target, docker volumes (pg+redis). Kept README.md + CLAUDE.md roadmap. Restart from Phase 0 Step 1. Prior HEAD: f41ac76.
- 2026-05-22 — drafted CLAUDE.md + README.md roadmap (12 phases, 140–225 hrs total)
- 2026-05-22 — added stretch goals S1–S7 to README (multi-region, E2EE, WebRTC, search, federation, abuse, compliance)
- 2026-05-22 — Phase 0 Step 1 done: Cargo workspace (shared/server/client) builds green w/ resolver = "3"
- 2026-05-22 — Phase 0 Step 2 done: docker-compose w/ Postgres 16 + Redis 7, both healthchecks passing
- 2026-05-22 — Phase 0 Step 3 done: axum 0.8 server, `/health` returns "ok" (deps: axum 0.8.9, tokio 1.52)
- 2026-05-23 — Phase 0 Step 4 done: sqlx 0.9 PgPool, `/ready` queries `SELECT 1`, returns 200/503, dotenvy + justfile. Pool auto-reconnects after pg restart.
- 2026-05-23 — Phase 0 Step 5 done: redis 1.2 ConnectionManager (features: aio, tokio-comp, connection-manager), `/ready` pings pg + redis, verified 200→503→200 on redis stop/start. **Phase 0 complete.**
- 2026-05-23 — Phase 1 Step 1 done: sqlx-cli installed, `create_users` migration (UUID PK, CITEXT username UNIQUE, password_hash, created_at, idx on created_at DESC, citext extension), reversibility verified. pgweb service added to docker-compose (port 8081, depends_on postgres healthy).
- 2026-05-23 — Phase 1 Step 2 done: `POST /signup` (new module `server/src/signup.rs`); argon2id hashing via `Argon2::default()` + `SaltString::generate(&mut OsRng)`, UUIDv7 ids, validation (username 3..32, password ≥8), 201/409 (citext unique violation)/400. Deps added: argon2 0.5, uuid 1.x w/ v7+serde, serde/serde_json, sqlx feature `uuid`, rand_core (unused now, leftover from import experiment).
- 2026-05-24 — Phase 1 Step 3 done: `POST /login` (new module `server/src/login.rs`); argon2 `verify_password` w/ DUMMY_HASH fallback on user-miss (constant-time vs enumeration), 32-byte random → base64url token, redis `SETEX session:<token> <user_id> 2592000`. Deps: base64 0.22, rand 0.10 (pinned w/o specific minor). Tested 200/401/timing-parity.
- 2026-05-25 — Phase 1 Step 4 done: `server/src/auth.rs` — `AuthUser(Uuid)` newtype + `FromRequestParts<AppState>` impl (parse `Authorization: Bearer ...` header → redis `GET session:<token>` → parse uuid). Protected `GET /me` returns `{id, username}`. Tested: 200 happy, 401 on missing/garbage/wrong-scheme. **Phase 1 functionally complete.**
- 2026-05-25 — Phase 2 Step 1 done: migration `create_conversations_and_messages` — chose unified model (Option A): `conversations(id, kind CHECK IN ('dm','group'), name, created_at)`, `conversation_members(conversation_id, user_id, joined_at, last_read_at, PK composite)`, `messages(id UUIDv7, conversation_id, sender_id, body, created_at)`. FKs ON DELETE CASCADE for conv→members/messages, RESTRICT for sender_id→users (preserve chat history). Indexes: `conversation_members(user_id)`, `messages(conversation_id, id DESC)`.
- 2026-05-31 — Phase 2 Step 2 done: `POST /conversations` (new module `server/src/conversations.rs`) — generic endpoint (Option A) w/ `{kind, member_ids, name?}`. DM branch: find-or-create in tx (BEGIN → SELECT existing DM by 2-way member JOIN → if found return 200, else INSERT conv + 2 members in one VALUES → COMMIT). 201 on create, 200 on reuse, 400 on dm-self / dm-needs-1-peer / FK violation (peer not found). Group stubbed 501. Known limitation: race window can dup-create on concurrent first hits — accept for MVP, fix later w/ unique pair index or advisory lock.
- 2026-05-31 — Phase 2 Step 3 done: `GET /conversations` lists my convs. Single query: JOIN `conversation_members` twice (cm filters to my conv ids, cm2 collects all members), `array_agg(cm2.user_id)`, GROUP BY c.id, ORDER BY id DESC LIMIT 100. `Vec<Uuid>` auto-decoded from pg `UUID[]` by sqlx. `member_ids` includes self by design. Route wired w/ `.post(create_conversation).get(list_conversations)` chain. Tested as bob → returns DM w/ both uuids.
