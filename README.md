# mlsd-chatapp

Chatapp. Rust full-stack. Learn system design by build.

## Why

Interview prep. Hands-on. Each phase = design concept + Rust skill.

## End state

- 1-to-1 + group chat
- Realtime via WebSocket
- Presence, typing, read receipts
- Offline msg delivery
- Media upload
- Multi-node scale, horizontal
- Observability (metrics, traces, logs)

## Stack

| Layer | Pick |
|-------|------|
| Backend | Rust, tokio, axum |
| DB | Postgres |
| Cache/presence | Redis |
| Realtime | WebSocket |
| Queue | NATS/Kafka (phase 4+) |
| Frontend | Rust (leptos/dioxus TBD) |
| Infra | Docker → k8s |
| Obs | OpenTelemetry, Prometheus, Grafana |

## Roadmap

Hour estimates = solo, learning pace (docs + debug). Rust-new adds ~30%.

| Phase | Hours |
|-------|------:|
| 0 Bootstrap | 4–6 |
| 1 Auth | 10–15 |
| 2 REST msg | 10–15 |
| 3 WS single-node | 15–25 |
| 4 Multi-node fanout | 15–20 |
| 5 Presence/typing | 6–10 |
| 6 Offline/inbox | 12–18 |
| 7 Media | 10–15 |
| 8 Scale/reliability | 15–25 |
| 9 Observability | 10–15 |
| 10 Frontend WASM | 25–40 |
| 11 k8s deploy | 10–20 |
| **Total** | **140–225** |

Cadence: 5 hr/wk → 7–10 mo · 10 hr/wk → 3.5–5 mo · 20 hr/wk → 2–3 mo.
Backend-only path (skip 10+11): ~100–165 hrs.

### Phase 0 — bootstrap _(4–6 hrs)_
- Cargo workspace: `server`, `client`, `shared`
- Docker compose: Postgres, Redis
- Health endpoint
- **Design lesson:** monorepo vs polyrepo. Shared types.
- **Rust:** workspace, features, error types (`thiserror`).

### Phase 1 — auth + users _(10–15 hrs)_
- Signup/login. Argon2id hash. Opaque session token in Redis (TTL).
- Auth extractor: `Authorization: Bearer <token>` → redis lookup → `user_id`.
- Postgres schema: `users` (pg). Sessions live in redis, not pg.
- **Design:** stateful vs stateless auth (opaque session vs JWT) — chose stateful for revocability + per-user session control. Constant-time defense vs account enumeration. CITEXT for case-insensitive username uniqueness.
- **Rust:** axum extractors (`FromRequestParts`), middleware, sqlx, argon2 PHC format.

### Phase 2 — REST messaging (no realtime) _(10–15 hrs)_
- Send msg → DB. Poll inbox.
- Conversation model: DM + group.
- **Design:** message schema, idempotency keys, pagination (cursor vs offset).
- **Rust:** sqlx migrations, transaction patterns.

### Phase 3 — WebSocket realtime _(15–25 hrs)_
- WS endpoint. Single-node fanout via in-mem channels.
- Heartbeat, reconnect.
- **Design:** push vs pull, long-poll vs WS vs SSE. Backpressure. Connection lifecycle.
- **Rust:** tokio `broadcast`/`mpsc`, `tokio-tungstenite`, select loops.

### Phase 4 — multi-node fanout _(15–20 hrs)_
- 2+ server nodes. Sticky? No — route any node.
- Redis pub/sub (or NATS) for cross-node delivery.
- **Design:** pub/sub vs queue. Delivery semantics (at-most/at-least/exactly once). Why exactly-once myth.
- **Rust:** shared client pools, graceful shutdown.

### Phase 5 — presence + typing _(6–10 hrs)_
- Redis presence (TTL keys, heartbeats).
- Typing = ephemeral pub/sub event.
- **Design:** soft state, TTL expiry, hot-key problem on celebs.
- **Rust:** background tasks, `tokio::time`.

### Phase 6 — offline + delivery _(12–18 hrs)_
- Per-user inbox. Unread cursor.
- Push notify hook (stub).
- **Design:** inbox fanout (write-fanout vs read-fanout), Discord/Slack patterns. Storage cost tradeoff.

### Phase 7 — media _(10–15 hrs)_
- S3-compatible (minio local). Presigned upload.
- Thumbnail worker (separate process).
- **Design:** direct-to-blob upload, signed URLs, CDN edge, virus scan async.
- **Rust:** background worker, job queue.

### Phase 8 — scale + reliability _(15–25 hrs)_
- Read replicas. Connection pool tuning.
- Rate limit (token bucket, Redis).
- Circuit breaker on deps.
- **Design:** N+1, hot partition, sharding key choice (user_id vs conv_id). CAP in practice.

### Phase 9 — observability _(10–15 hrs)_
- OTel traces across services. Prom metrics. Structured logs.
- Grafana dashboards. Alert rules.
- **Design:** RED/USE method. SLO/SLI/error budget.

### Phase 10 — frontend _(25–40 hrs)_
- Rust WASM client. WS client. Local cache (IndexedDB).
- **Design:** optimistic UI, offline-first, conflict resolution.

### Phase 11 — deploy _(10–20 hrs)_
- k8s manifests. Helm. Rolling deploy.
- **Design:** zero-downtime deploy, draining WS connections, blue/green vs canary.

## Stretch goals (post-phase-11)

Tackle after core 12 phases. Each = own mini-project. Interview-relevant for senior rounds.

### S1 — Multi-region active-active _(30–50 hrs)_
- 2+ regions, geo-routed clients, regional shards, cross-region replication.
- **Design:** CRDT vs last-write-wins, region failover, split-brain, latency budget.

### S2 — End-to-end encryption _(25–40 hrs)_
- Signal protocol (X3DH + Double Ratchet). Device keys, multi-device sync.
- **Design:** key server trust model, forward secrecy, backup/recovery tradeoffs.

### S3 — Voice/video calls (WebRTC) _(40–60 hrs)_
- SFU (e.g. mediasoup/livekit). TURN server. Signaling over existing WS.
- **Design:** P2P vs SFU vs MCU, NAT traversal, bandwidth adaptation.

### S4 — Search at scale _(15–25 hrs)_
- Elasticsearch or Meilisearch. CDC pipeline from Postgres.
- **Design:** index lag, relevance tuning, per-user ACL on search.

### S5 — Federation (Matrix-style) _(30–50 hrs)_
- Cross-server protocol. Server-to-server auth, event graph.
- **Design:** eventual consistency across untrusted servers, DAG sync.

### S6 — Abuse / spam / safety _(15–25 hrs)_
- Rate-limit by behavior, ML moderation hook, report flow, shadow ban.
- **Design:** false-positive cost, appeal loop, evasion patterns.

### S7 — Compliance _(10–15 hrs)_
- GDPR delete-cascade, data retention, audit log, export.
- **Design:** soft vs hard delete, encryption-at-rest key per user.

**Stretch total: ~165–265 hrs.** Pick by interview target.

## System design concepts covered

- Stateless services, horizontal scale
- Pub/sub vs queue vs stream
- Fanout patterns (push vs pull inbox)
- Caching layers, TTL, invalidation
- Sharding + partition key choice
- Idempotency, delivery semantics
- Backpressure, rate limit, circuit break
- Presence (soft state)
- WS connection lifecycle at scale
- CDC, outbox pattern (phase 4+ if needed)
- Observability triad
- Zero-downtime deploy w/ stateful conns

## Rust skills covered

- Workspace + module layout
- `tokio` runtime, async patterns
- `axum` handlers, extractors, middleware
- `sqlx` queries, migrations, transactions
- WS w/ `tokio-tungstenite`
- Channels: `mpsc`, `broadcast`, `watch`
- Error handling: `thiserror`, `anyhow`
- Background workers, graceful shutdown
- WASM frontend
- Testing: unit, integration w/ testcontainers

## Run

TBD per phase. Compose up first.

## Convention

- Each phase = own branch + PR.
- PR body: design lesson + tradeoffs taken.
- Commit msg: caveman style.
