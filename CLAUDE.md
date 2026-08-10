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
- **Always cite the line number** when giving updated/changed code — tell user exactly which line(s) the edit lands on (e.g. "line 42" or "lines 59-72"), so they can locate it fast. Read the file if unsure of current line numbers.
- Anticipate beginner Rust questions (ownership, async, lifetimes, traits) — flag tricky bits inline.
- After user reports done → update **Progress** section below. Then wait for next ask.
- New session → AI reads Progress section first, then reads **all source files** (`server/src/*.rs`, `shared/src/*.rs`, `client/src/*.rs`), `Cargo.toml`s, `docker-compose.yml`, `migrations/*.sql`, `justfile`. Builds full mental model before answering. No "let me check" mid-conversation.

## End-of-project TODO

- **Quiz user on EVERYTHING** at project end: system-design concepts (fanout, backpressure, idempotency, pull/push, contract-first, keyset pagination, sessions, etc.) + Rust (ownership, borrow, async, traits, serde, tokio). Comprehensive review before calling it done. Do not skip.

## Progress

**Current phase:** Phase 4/6 reliability hardening → next likely Phase 5 (presence/typing)
**Last completed step:** Phase 4 reliability gap closed — mid-session conv-join re-subscribe via `control` channel (2026-08-09, tested working on :3001 single-node). Phases 0–4 done, Phase 6 core done, Phase 10 test harness done early. WS subscriber reconnect-on-drop also already done.
**Next step:** open options — (a) Phase 5 presence/typing (new concept: ephemeral Redis TTL keys + heartbeat — biggest untouched surface); (b) live read receipts (PUBLISH read event on mark_read → ✓✓); (c) unread count badge + expose `last_read_message_id` in `GET /conversations`; (d) backpressure: bounded WS channel + slow-consumer disconnect (phase-3 deferred risk).
**Files in flight:** `Cargo.toml` (single pkg, no workspace), `src/*.rs`, `static/index.html` (test client), `docker-compose.yml`, `.env`, `Justfile`
**Repo structure:** single crate. Root = `server` binary (`src/*.rs`). Wire types live in `src/contract.rs` (`mod contract`) — all API DTOs + WS protocol, the single source of truth for the API shape. `client` crate + `contract`/`shared` crates dropped. Frontend = external consumer coding against `contract` module (contract-first; Rust frontend deferred to phase 10, likely non-Rust).
**Open decisions:**
- ~~Frontend framework (leptos vs dioxus vs yew)~~ — RESOLVED 2026-08-03: Rust WASM frontend cut. Phase 10 = shabby static `index.html` + vanilla JS as a manual test harness only. Backend stays contract-first, consumed as an external client would.
- Queue (NATS vs Kafka) — defer to phase 4

**Log:**
- 2026-08-09 — Phase 4 reliability: mid-session conv-join re-subscribe. Bug: `handle_socket` snapshots user's `conv_ids` once at connect (ws.rs) → conv created *after* WS open never gets `SUBSCRIBE conv:{id}` on that node → peer gets no live msg until reconnect. Also multi-node: `create_dm` runs on the HTTP node, but the peer's socket may live on a *different* node whose in-mem hub the creator can't see. Fix = node-level `control` pub/sub channel: `create_dm` PUBLISHes `ConvCreate{conv_id, members}` to `control` after commit (conversations.rs); every node subscribes `control` once at startup + re-subs on reconnect (fanout.rs `run_subscriber`); on receipt, node checks its local `hub` for any member, and `add_sub(conv_id)` once **per local socket** (`local_count` = Σ senders across hosted members — per-socket unit keeps refcount balanced vs multi-device). `run_subscriber` now dispatches by `msg.get_channel_name()`: `control` → `ConvCreate`, `conv:*` → `Broadcast`→`local_fanout`. Refcount cleanup (hole): subscriber adds but the socket's frozen `conv_ids` snapshot can't remove it → disconnect now **re-queries** current membership from DB and `remove_sub`s that set (ws.rs) instead of the snapshot → balances the mid-session add. Rust trap dodged: hub `std::Mutex` guard scoped + dropped *before* the `async add_sub().await` (guard not Send across await). Known gap (latent, no feature yet): leave-conversation would leak an add (re-query won't see the left conv). New type `ConvCreate` (not a flag on `Broadcast` — channel name is the discriminator). Tested :3001: bob WS open → new account creates DM w/ bob → sends → bob receives live, no reconnect.
- 2026-08-06 — Phase 6 core done: read cursor + catch-up. Migration `add_read_cursor`: dropped `last_read_at` (timestamp), added `last_read_message_id UUID` on `conversation_members`. Why msg-id over timestamp: UUIDv7 = time-ordered → exact watermark + keyset resume, rides existing `messages(conversation_id, id DESC)` idx; ts collides in same ms. `POST /conversations/{id}/read` (`mark_read` in messages.rs) — monotonic advance (`WHERE ... AND (last_read_message_id IS NULL OR $1 > last_read_message_id)`); 0 rows → membership check to distinguish 403 from stale-cursor; returns 204. `list_messages` extended: `?after=<id>` → `id > $ ORDER BY id ASC` (catch-up, oldest-missed first, so LIMIT doesn't skip gap); `?before=` unchanged (scroll-back DESC); match on `(q.after, q.before)`, after wins if both. Concept: push (live WS) + pull (`?after=` catch-up) hybrid; delivery ≠ read (don't auto-mark on catch-up; client reports read via explicit endpoint). Real clients keep local SQLite + sync forward via `?after=<local max>`; read cursor server-stored for multi-device receipts. NOTE route in main.rs is `post` (fixed from an initial `get` — GET+Json body breaks). Test data reset: cleaned bob↔carol history + deleted bob's other 2 DMs (2026-08-06).
- 2026-08-06 — Phase 3 Step (a) done: `ClientEvent::SendMessage {conversation_id, body}` inbound over WS. Refactored `send_message` (axum handler) → thin wrapper over new `create_message(db, redis: ConnectionManager, sender_id, conv_id, body)` core (validate→insert→PUBLISH); WS recv-task calls core directly. Added `ServerEvent::Error(String)` (sent to socket on create fail; JSON-parse fail swallowed by `Err(_)=>{}`). `create_message` takes redis by value (owned, `mut`) — one clone at call site (WS loop clones per iter since fn consumes it). Sender's own echo arrives via Redis fanout round-trip, symmetric w/ HTTP.
- 2026-08-06 — Phase 10 test client done early: `static/index.html` (plain HTML+vanilla JS, dark-mode via `prefers-color-scheme`), served by axum `/` route via `include_str!` (no ServeDir dep, bakes at compile time → rebuild to see HTML changes). User picker (bob/carol), login→auto WS connect + load convs, live receive log, send over WS, mark-read button (tracks newest seen id). WS auth: browsers can't set headers on WS handshake → `AuthUser` now also accepts `?token=` query param (header still works for curl/websocat). Dev UIs in docker-compose: Adminer (pg, :8082, native dark, `ADMINER_DEFAULT_SERVER=postgres`), RedisInsight (redis, :8081, arm64-native). Connect via service names `postgres`/`redis` (not localhost — that's the UI container).
- 2026-08-03 — Phase 3 Step 6 done: per-conversation channels + dynamic sub/unsub. `pubsub.split()` → `(PubSubSink, PubSubStream)`: sink (Clone) in `AppState`, stream drained by `run_subscriber`. Channel = `conv:{id}`. On WS connect: query user's conv_ids, `add_sub` each (node-level refcount map `subs: Arc<Mutex<HashMap<conv,usize>>>`; SUBSCRIBE only on 0→1). On disconnect: `remove_sub` (UNSUBSCRIBE on 1→0). `send_message` PUBLISHes to `conv:{conv_id}` via ConnectionManager (`state.redis`), not the sink — sink is subscribe-control only. Verified 2-instance: `PUBSUB CHANNELS` shows `conv:*` while connected, vanish on disconnect; cross-node delivery works. Why split(): `on_message()` borrows `&mut pubsub` for the loop, blocking `subscribe()`; split gives independent owned halves. Known gaps: subs computed at connect (mid-session conv-join needs reconnect); no subscriber reconnect on drop; per-conv scales poorly for users-in-many-convs (→ per-user channels).
- 2026-08-02 — Phase 3 Step 5 done: Redis pub/sub cross-node fanout. `src/fanout.rs`: global channel `"message"`, `Broadcast {members, payload}`, `run_subscriber` (dedicated `get_async_pubsub` conn, `on_message` loop → `local_fanout` to local Hub), spawned in main w/ `redis_client.clone()` + `hub.clone()`. `send_message` now PUBLISHes `Broadcast` instead of touching Hub directly — all delivery (even same-node) routes through Redis → every node's subscriber → local sockets. `PORT` env-configurable (Justfile `run port="3000"`). Verified 2-instance: carol POST→A, bob WS on B receives. Bugs squashed: `PULISH` typo (swallowed by `unwrap_or`), websocat variadic `-H` eats URL (put URL first). Known gaps: no subscriber reconnect on drop; global channel = every node sees every msg (next: per-conversation channels).
- 2026-08-02 — Collapsed `contract` crate → `src/contract.rs` module (`mod contract`). Single crate now, no workspace. Reason: only one consumer (server), so a separate crate was ceremony w/o payoff (YAGNI); the dep-firewall benefit didn't justify a whole crate for one binary. Imports → `crate::contract::*`. Note: `ClientEvent` now warns unused (module-level dead-code lint sees it; a lib crate's `pub` items didn't) — expected until inbound WS parse wired. Build green.
- 2026-08-02 — Repo restructure: dropped `client` crate; renamed `shared`→`contract`; promoted `server` to root package (`src/*.rs` at root, no `server/` subdir); consolidated ALL wire DTOs into `contract` (entities + WS protocol + every request/response DTO: `Message`, `ServerEvent`/`ClientEvent`, `SignupReq/Resp`, `LoginReq/Resp`, `MeResp`, `CreateConvReq`/`ConvResp`, `SendMsgReq`, `ListMsgQuery`, `ListMsgResp`). `MsgResp` deleted (was dup of `Message`). Rationale: contract-first — one crate = single source of truth for API shape, dep-free so it can't leak `sqlx`/`axum`; more realistic than Rust-fullstack shared-types (industry norm = backend + external frontend via contract). sqlx row tuples stay in `src/`. `cargo build` green.
- 2026-08-02 — Phase 3 Step 3 done: typed WS protocol. `shared/lib.rs` = adjacently-tagged enums `ServerEvent`/`ClientEvent` (`tag="type", content="data", snake_case`). `serde` dep in shared fixed (was `verson`/`derived` typos). Collapsed `MsgResp`→`ServerEvent` in `send_message`: both HTTP 201 body + WS fanout now serialize `ServerEvent::NewMessage` (one type, no clone). `MsgResp` retained only for `list_messages`. Wire = `{"type":"new_message","data":{...}}`. `ClientEvent::Ping` defined but inbound parsing not wired yet (recv-task still only watches Close).
- 2026-07-27 — Phase 3 Step 1+2 done: `/ws` endpoint w/ `AuthUser` handshake gate + in-memory fanout. `Hub = Arc<Mutex<HashMap<Uuid, Vec<UnboundedSender<String>>>>>` in `AppState` (user_id → per-device senders). `handle_socket`: `socket.split()` → sink/stream, register `tx` clone in Hub, spawn write-task (rx→sink), recv-task watches Close, `select!` (abort spawned send_task on recv end; local recv future auto-dropped), cleanup `retain(same_channel)` + remove-if-empty. `send_message` fanout: query members → lookup Hub → `tx.send(serde_json payload)` (includes sender, ignores dead senders). Single-node only (in-mem map) — Redis pub/sub deferred. Unbounded channel = slow-consumer risk, deferred (bounded+disconnect later).
- 2026-07-20 — Phase 0 Step 5 done: redis 0.27 ConnectionManager w/ `ConnectionManagerConfig::set_connection_timeout(2s) + set_response_timeout(2s)`, `/ready` pings both pg + redis (`redis::cmd("PING").query_async::<String>` == "PONG"), 200/503 matrix verified. **Phase 0 complete.**
- 2026-07-23 — Phase 2 Step 2 done: `POST /conversations` — DM find-or-create in tx (self-join on `conversation_members` to find existing DM; INSERT conv + 2 members if not). 201 create / 200 reuse / 400 self-dm / 400 dm needs 1 peer / 400 peer not found. Group stub 501.
- 2026-07-25 — Phase 2 Step 5 done: `GET /conversations` — one query w/ self-join (`cm` filters my convs, `cm2` gathers all members), `array_agg(cm2.user_id)` → `Vec<Uuid>` auto-decoded by sqlx, `ORDER BY c.id DESC LIMIT 100`. Route chained `.post().get()`. member_ids includes self by design.
- 2026-07-24 — Phase 2 Step 4 done: `GET /conversations/{id}/messages?before=&limit=` — keyset pagination (id DESC), membership 403 gate, `Query<T>` extractor for query params, `next_cursor` = last row id when page full, null when tail. Route chained `.post().get()` on same path.
- 2026-07-23 — Phase 2 Step 3 done: `POST /conversations/{id}/messages` in new `messages.rs` module — membership check via `SELECT 1 FROM conversation_members`, 403 if not member, 400 on empty/too-long body (>4096), UUIDv7 msg id. Route uses axum 0.8 `{id}` path syntax.
- 2026-07-22 — Phase 2 Step 1 done: migration `create_conversations_and_messages` — `conversations(id, kind CHECK IN ('dm','group'), name, created_at)`, `conversation_members(conversation_id, user_id, joined_at, last_read_at, PK composite)`, `messages(id UUIDv7, conversation_id, sender_id, body, created_at)`. FKs: conv→members/messages CASCADE, user→members/messages RESTRICT. Indexes: `conversation_members(user_id)`, `messages(conversation_id, id DESC)`.
- 2026-07-22 — Phase 1 Step 4 done: `server/src/auth.rs` — `AuthUser(Uuid)` newtype + `FromRequestParts<AppState>` impl (parse `Authorization: Bearer <token>` → redis `GET session:<token>` → parse uuid). Protected `GET /me` returns `{id, username}` from users table. Tested 200/401/401/401. **Phase 1 functionally complete.**
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
