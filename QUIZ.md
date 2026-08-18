# Chatapp — Final Review Quiz

Comprehensive review of everything built (Phases 0–11). Two parts: **System Design** + **Rust**.
Answer from memory first. Then check the **Answer Key** at the bottom (no peeking).

Mix of: _recall_ (what is X), _why_ (tradeoff/reasoning), and _code-reading_ (Rust).

---

## Part 1 — System Design

### A. Auth & Sessions

1. Why store the session token in Redis (`session:<token> → user_id`) instead of a signed stateless JWT? Tradeoffs of each.
   jwt, if leaked, will allow the hacker to keep accessing the app with no way for us to revoke access.

Redis sacrifices a network call to ensure that, if needed the user, or admin can update the tokens so that access can be revoked.

2. Login hashes with argon2 and falls back to `DUMMY_HASH` when the user doesn't exist. What attack does that defend against, and how?

hacker can calculate the time with each username input, longer ones means that a user exists and we tried to compare the pw hash. So this will give away the usernames of our userbase

3. What's the `DUMMY_HASH` bug that metrics surfaced? Why does it return 500 instead of 401?
   Not sure

4. Login is rate-limited by **IP**, not user id. Why can't it be by user id? What attack does IP-only miss, and what key would fix it?

Swithcing IP address is harder, so hacker cannot loop over a bunch of usersnames trying to DDOS the login service.

IP address will miss proxy/vpn. Not sure how we can stop it though.

5. `AuthUser` accepts the token via `Authorization` header **or** `?token=` query param. Why is the query-param path needed at all?

Token will be passed, we check the token in redis and this will tell us whether the user can access what they requested.

### B. Data Model & Pagination

6. Messages use UUIDv7 ids. What property of v7 makes it better than a random UUID or an auto-increment int here? Name two things it enables.

uuid created after another uuid can be compared and the one created after will always be larger

not int based means we can't use the id to tell how many users we have had.

and id collision is highly unlikely

7. Explain keyset (cursor) pagination vs `OFFSET`. Why does keyset scale and offset doesn't?

offset doesn't scale before it would cause the DB to still read all the rows upto where we want to offset.

pagination makes it so that we only query the slice of data we care about. Db footprint is smaller

8. `?before=` scrolls back (DESC), `?after=` catches up (ASC). Why must catch-up be ASC-ordered rather than DESC?

because we need to loop through the results to display the catch up in the order they came in.

9. The read cursor is `last_read_message_id` (a UUID), not `last_read_at` (a timestamp). Why the id over the timestamp?

because the id can be used to compare with other ids. later ids are going to be in higher value compared to earlier ids

10. `mark_read` uses `WHERE ... AND ($1 > last_read_message_id)`. What property does that clause guarantee, and why does it matter?

is guarantees that the current message we are trying to mark as read is a message created after the last message we marked as read.

### C. Realtime: Fanout & WebSockets

11. Describe the full path of a message from sender to a recipient on a **different node**. Which components does it cross?

ws connected. in the ws i type in the json with the conv id, and the content.

this will then call the crete message endpoint behind the scene. We check if the user even have the right to write to this conv id

then once we have done all the checks, we publish this to redis with the appropriate prefix + conv id

other nodes sub to this will fetch the content. check if the users of the convo has a ws connected. then push this to the queue. the queue is drained with the result streamed to the ws

12. Why does _every_ message (even same-node) route through Redis pub/sub instead of the local Hub directly?

if a ws was connected to another instance that should receive this message, we don't konw where this ws is, or if there even is a ws we need to send the new message to. So we have to publish everything onto redis, so that other instances of the api can fetch it, and check if there is a relevant ws connected that need this msg streamed to them

13. What is the `Hub` and why is it per-node in-memory (not shared)? What happens to it on pod crash?

the hub is used for the current instance to track what are the users that are connected to it with a ws. Then, when conversation with user connected receives a message, we know to use the channel stored there to stream the message to their ws.

On crash, the hub is lost and the ws connections should be disconnected as well

14. Per-conversation channels (`conv:{id}`) vs one global channel — what problem does per-conv solve, and what new problem does it introduce for users in many conversations?

per conversation channel allows each instance to sub to only the conversations that it cares about. That is, the conversations that a user with a ws connected to.

A global channel will mean that we have no way of knowing which conv the lateest message s for, so every instnace will need to pull that message down and do a local check for whether it they need to stream the message to a user with a ws opened locally.

The problem is if a users have many conversations, then an instance will need to be subbed to many channels.

15. The `subs` refcount map: why count subscriptions per-socket instead of a boolean "subscribed or not"?

because if there is a convesation that belongs to two users, and if both users are connected to this same instance, then have a flag means that if one person logs off, the other person still connected may lose updates to that conversation because we would have turned it off.

16. Mid-session a user joins a new conversation. Why doesn't their existing socket receive live messages, and how does the `control` channel fix it?

because we load all the channels that the user is part of when they have connect to the insrance via ws. if a new conv was created, this list is not updated.

A control channel is a patch on this. if a new conv was created,and the user for that conv is connected to me, I need to also sub to it. so the control channel is basically used for this.

### D. Presence & Typing

17. Presence uses Redis TTL + heartbeat, **not** a connect/disconnect flag. What failure mode does the flag approach have that TTL fixes?

if the client crashes, the status might not get updated. so they will remain online falesely

18. Why is there no explicit `DEL` on disconnect for presence? What multi-device behavior does TTL-only preserve?

because there is no need. the key will be gone after some time. if user logged into two devices, ttl only makes it so that when they log off, they can still appear as logged in because the other device will refresh the key

19. Typing is described as the "opposite of a message." List 4 ways it differs (persistence, ack, retry, ordering).

persistence, it is gone after the set time,

doesn't not need ack or receipt, because the ws for the other use might have been logged of, or there may not even be a api instance that is listening to this typing event, no retry, doesn't matter if we sent or not, they can get the next one if something happens

doesn't matter, can over write if needed.

20. Who throttles typing events and who expires them? Why is that split (not the server)?

the frontend will be throttling the typing event because that is where the typing can be tracked freely without any network call.

redis expires them. split because keystorke is local with the client, so it can control how often the poke the keystroke endpoint

### E. Reliability: Backpressure, Rate Limiting, Idempotency

21. The WS channel was changed `unbounded` → `bounded(32)`. What disaster does the bound prevent? What's the cost?

a client crashes, ws still connects so messages keeps piling ontp the channel witohut it being drained

22. `local_fanout` uses `try_send` and drops on `Full`. Why is dropping acceptable here — what recovers the dropped message?

the message is already written to the db, so nothing is lost. just no immediate update

23. Fixed-window rate limit: `INCR` + `EXPIRE` only on `n==1`. Why only on the first increment? What's the "2× seam burst" weakness?

no sure what this is abot.

24. Name the three rate-limit algorithms discussed and one tradeoff of each (fixed-window, sliding-log, token-bucket).

fixed windows, the 2 seamed burst

sliding log, uses more memory as we need to track the time stamp of each message

token based. need to assign tokn so we need to validate the token? not sure about this one

25. App-level vs gateway-level rate limiting — what belongs at each layer and why (defense in depth)?

app level for tracking all internal endpoints like how many msgs a user is sending. gateway level for login in.

26. How does client-side dedup by UUIDv7 achieve idempotent delivery? Give a scenario where the same message arrives twice.

not sure

### F. Media & Storage

27. Explain the control-plane / data-plane split for media. What crosses the server, what doesn't?

no binary bytes cross the server. the server will only talk to the minio to get a link (does some check too like is the user allowed to add to this conv. then the link is given back to the client where they will submit it directly to minio)

28. Two gates: the **write gate** (attach) and **read gate** (download). Why is the read gate alone forgeable, and how does the write gate make links trustworthy?

not sure

29. A presigned URL is called a "bearer token." What does that mean for security? Name two mitigations.

not sure

30. Presigning is "offline." What does that mean, and why did it let us sign URLs for `localhost:9000` while the app connects to `minio:9000`?

because intrenaly to the app we are connectd to minio via a closed network set up by k8s which allows us to call another service with mino:9000, but this url is not available outside of the k8s internal so they will connect to localhost:9000 where the port is forwarded by us manually

31. Store compromise exposes all media even with encryption-at-rest. Why? What's the only thing that hides content from the server, and what does it cost?

not sure what this question is asking

### G. Sharding & Scaling

32. Sharding by hashed id gives N× write throughput. Read _by key_ scales too — why? Read _without_ the key (search) degrades — why?

read by key scales because the key will allow us to pin point exactly which instance of the db the data is at. if we don't have the key, we would have to scan through all instances of db to find the data.

33. What is scatter-gather and why is its latency the _slowest_ shard?

scatter gather is when you send the same query to all shards, and collect all results in one place and return the aggregated results. so natually, since we query all shards and wait for all results to comeback, the latency is the slowest shard.

34. Why do you put search in a dedicated index (Elasticsearch) instead of querying the shards? What pattern is this (write model ≠ read model)?

because we can do it faster. not sure exactly what you are looking for here

35. Name three shard-key gotchas and the fix for each (hot shards, rebalancing, cross-shard joins).

not sure

36. Object stores scale differently from databases. Contrast erasure-coded distributed clusters vs primary/replica. Which model is which?

former is object, and latter is db

### H. Observability

37. Prometheus is "pull-based." What does it scrape, how often, and what does that imply about history?

we set a /metrics endpoint where prom can pull the data. we set how often we can fetch right? so if a node goes down before a pull, we lose the metrics,unless we write it to disk, which defeats the purpose of prom

38. What are RED metrics and how does `axum-prometheus` give them "for free"?

no idea

39. `rate(counter[1m])`, `histogram_quantile(0.95, ...)`, `up` — what does each tell you?

how many hits per minute in total across all endpoint

how long does 95pct of our request take to return.

if the service is up?

40. The three pillars (metrics/logs/traces) — why are traces **sampled** but metrics not? When do traces shine?

traces can be quite heavy since, so sending it on every hit is going to take bandwidth off the app from doing things that it should spend mose time doing.

metrics are not sampled because they are just a dict of entries plus lisst and counts. they are small.

traces shine from being able to tell us which part of the system is a bottle neck in terms of performance, like if this query to db taking too long

### I. Deploy & Kubernetes

41. Cluster → Node → Pod → Container. Define each in one line.

cluster is where a set of nodes are linked via k8s.

a node is the physical machine in this cluser

a node can spawn many pods which can be thought of as its own vm inside the node

containers are spawned inside each node, where the containers can talk inside the pod as if they were local

42. Deployment vs Service vs ReplicaSet — what does each do? Which one you rarely touch?

deployment species the task that you want k8s to run for you. could be a one of job, could be a consistent state of the system you want k8s to maintain

service is a set of pods that load the containers you would want to run. each pod in the same service is given the same label, but a different id, traffics to this service will be distributed amongts these pods

replica set, one replica set is one pod with a specific set of containers running inside it

43. How does a Service find its pods? (Not by containment — by what?)

don't remember

44. Why is a Postgres Deployment with `replicas: 3` behind one Service a _data-corruption_ bug, while an app Deployment with `replicas: 3` is fine?

because db need a single source of truth... and when you write to a db, you want the write to always go to an expected instance, so your reads will also always go to the same fo fetch the data we expect it to have

45. Declarative vs imperative: why did `kubectl apply` silently scale your 5 pods back to 1?

because it reads the yaml file, which declares 1, but we temporarily set the replicas to 5 via kubectl, so it is not declarative

46. What makes a rolling update zero-downtime? What role does the readiness probe play?

it will replace the pods one by one as they are ready to be removed. the readiness probe checks if a pod is ready to be taken down for a new one to replace it

47. Why use immutable image tags (`v3`) instead of reusing `latest`/same-tag? What breaks with same-tag?

rolling update won't use the latest image because it wouldn't know that we have made changes to the app

48. `kubectl port-forward svc/chat` — why do two browser tabs land on the _same_ pod? How is that different from a real Service?

it pins a specific pod's port to local, so there is no service discovery or network routing to send it to different pods

49. A real Service load-balances "per-connection," not "per-request." What does that mean for a WebSocket, and why does it force you to use Redis fanout?

so the ws connected to each pod is balanced so that no one single pod gets all the traffics. since ws will be connected to diff pods, they are separate so they need redis as a bridge to tell it which conv has been updated and whether they need to pull those changes down for the user

50. Why does `kind` need `kind load docker-image`? (What are the 3 separate image stores?)

kind spawns a container as a cluster. it does not connect to my local's image stores and instead keeps a copy of its own images in the container. not sure what you mean by 3 imaget stores, local, local within kind, and one more?

51. Why is bucket creation / migrations done by an external Job / on-boot, not baked into the pg/minio server pods? (mechanism vs policy)

because the server dosen't have the cli tool needed to run those commands

52. How does a pod authenticate to a cloud secret store _without_ stored keys? Name the pattern (GCP + AWS names).

uses a service account, which gives access to secrets stored in the corresponding cloud provider's secret manager. those secrets can then be loaded into the pod via clould native mechanisms

53. Why is a k8s `Secret` not actually secure at rest, and what do you use in production instead?

secret is stored as base 64 so you can just convert it back to plain text. you would use a secret manager service

54. Graceful connection draining on deploy: what does k8s do by default (SIGTERM → ? → ?), and what would you add to drain WS cleanly?

not sure

---

## Part 2 — Rust

### J. Ownership & Borrowing

55. `create_message` takes `redis: &mut ConnectionManager`. Why `&mut` and not owned or `&`?

because one of the methods we use with redis require it to be mutable

56. In `ws.rs` disconnect cleanup, the hub `std::Mutex` guard must be dropped _before_ an `.await`. Why? (What trait is involved?)

await releases the ownership from the current scope. so we have to be done and release the lock for the hub

57. `AuthUser(pub Uuid)` is a newtype. What does the newtype pattern buy you here vs passing a raw `Uuid`?

the authuser is an axum object that will make endponts call it automatically?

58. What's the difference between `String` and `&str`, and why do DTOs use `String` while error returns use `&'static str`?

String is owned with the current scope. &str is borrowed from another scope and we can't free mutate it.

String is not known at compile time... so it will ocuppy space in the heap. but static str is a fixed string we define in the code, so it can go to the stack and makes it more efficient

### K. Async & Concurrency

59. `tokio::select!` in `handle_socket` races the send task and recv task. What happens to the _other_ branch when one completes?

both tasks will be closed by tokio

60. Why is the pub/sub connection `split()` into sink + stream? What borrow problem does that solve?

so that the stream can be owned by the ws at all time. and sink can we used inside hub? otherwise, we are not allowed to have the object used in multiple locations

61. `mpsc::Sender::try_send` vs `send().await` — when do you use each? Why does `local_fanout` use `try_send`?

send().await is when we are in a task?

try_send will check if the queue is full or not and only send if not full

local_fanout uses try_send because we need to know if the queue is not full so we can safely send it

62. The metrics `/metrics` route uses `move || async move { ... }` — a "double move." What are the two moves and why are both needed?

i know that first move is required because we need to pass the object into the closure. second move i guess is for moving it into a task?

63. `tokio::spawn` returns a `JoinHandle`. Why does the code `.abort()` the send task but let the recv future drop naturally?

not sure

### L. Traits, Types, Serde

64. `AuthUser` implements `FromRequestParts`. What does implementing that trait let axum do automatically?

it gives us access to fetch the the authorization tokens and other http parts from the header etc so we can use it to validate

it lets axum automatcally fetch parse those from the incoming request

65. `ServerEvent` / `ClientEvent` use `#[serde(tag="type", content="data")]`. What JSON shape does that produce? Why adjacently-tagged over untagged?

a json with typy = "server_event/client_event" as a key and the content as the data key

66. Why does adding a `Hello` variant to `ServerEvent` require no change to the match arms that don't handle it — but the client JS _does_ need a new branch?

not sure

67. `sqlx::query_as::<_, (Uuid, String)>()` returns a tuple. Why tuples here instead of a struct with `FromRow`?

because the result is a row with two columns

### M. Macros & Compile-time

68. `include_str!("../static/index.html")` — when is the file read, and what must you do to see HTML edits?

you need to rebuild?

69. `sqlx::migrate!("./migrations")` — where do the migration SQL files end up, and why did the Dockerfile need `COPY migrations`?

they end up with the app binary. and we copy them because this contains the instructions we need to give to the db to make sure that we have the right db tables

70. You use `sqlx::query()` (runtime) not `sqlx::query!()` (macro). Why did that choice mean the Docker build needs **no live database**?

not sure

---

## Answer Key — no peeking until you've tried

<details>
<summary>Click to reveal</summary>

**A. Auth & Sessions**

1. Redis session = **revocable** (delete key = instant logout), server-controlled TTL, but stateful (needs Redis lookup per request). JWT = stateless/no lookup but **can't revoke** before expiry, and key rotation is harder. Chose Redis for revocability + simplicity.
2. **User-enumeration via timing.** If a missing user returned instantly (no hash) and a real user took ~100ms (argon2), an attacker learns which usernames exist. Hashing a `DUMMY_HASH` on the miss keeps timing ~constant.
3. `DUMMY_HASH="tatftahbbktfatene"` isn't a valid argon2 PHC string → `PasswordHash::new` returns `Err` → mapped to 500. Breaks the timing-parity defense. Fix = a real argon2 hash.
4. Pre-auth, there's **no user id yet** — the caller is unauthenticated. IP-only misses a **distributed/botnet attack on one account** (many IPs, one username). Fix = also key on `rate:login:user:{username}` (available from the body).
5. **Browsers can't set headers on a WebSocket handshake.** The JS WS client passes the token in the URL; curl/websocat still use the header.

**B. Data Model & Pagination** 6. UUIDv7 is **time-ordered**. Enables: (a) exact chronological sort / watermark without a separate timestamp, (b) keyset pagination on the PK, riding the existing `(conversation_id, id DESC)` index; also client-side dedup by stable id. 7. `OFFSET n` scans+discards n rows (O(n), worse deeper). Keyset uses `WHERE id < cursor` → an **index seek** to the cursor, O(log n), stable under inserts. 8. Catch-up must return **oldest-missed first** so a `LIMIT` doesn't skip the gap between last-seen and newest. DESC+LIMIT would return the newest and drop the middle. 9. UUIDv7 = exact watermark + keyset resume, rides the message index; timestamps **collide within the same millisecond** → ambiguous cursor. 10. **Monotonic advance** — the read cursor only ever moves forward, never backward (a stale/older mark-read is ignored). Prevents out-of-order requests from regressing the receipt.

**C. Fanout & WebSockets** 11. sender → `create_message` (validate, rate-limit, insert to PG) → `PUBLISH conv:{id}` to Redis → every node's `run_subscriber` receives → `local_fanout` → the recipient's in-mem `Hub` sender → their WS sink. 12. **Uniformity + multi-node correctness.** The recipient's socket may live on a _different_ node whose in-mem Hub the sender's node can't see. Routing all delivery through Redis means every node's subscriber handles it identically. 13. `Hub = Arc<Mutex<HashMap<Uuid, Vec<Sender>>>>` — user*id → live socket senders on \_this* node. In-mem because sockets are node-local resources; on crash the OS frees it and clients reconnect (state rebuilt from PG + `?after=`). 14. Per-conv scopes delivery so nodes only get messages for conversations they host → less cross-node noise. New problem: a user in _many_ conversations = many SUBSCRIBEs; scales poorly → eventually per-user channels. 15. A user can have **multiple devices/sockets** on one node; per-socket refcount keeps SUBSCRIBE/UNSUBSCRIBE balanced (only UNSUB at 1→0). A boolean would unsubscribe while another socket still needs it. 16. The socket snapshots `conv_ids` **at connect**; a conversation created later isn't in that set, and the creator may be on another node. `create_dm` PUBLISHes `ConvCreate` to the node-level `control` channel → every node checks its local Hub for a member and `add_sub`s per local socket.

**D. Presence & Typing** 17. Flag approach: crash/kill/partition means the disconnect handler **never runs** → user stuck "online" forever (ghost). TTL self-expires (≤20s) with no cooperation from the dead node. 18. Multi-device: any device's heartbeat keeps the key alive; an explicit DEL on one device disconnect would flip the user offline while still online elsewhere. TTL-only = "offline only when _all_ heartbeats stop." 19. Never persisted (vs stored), no ack (vs delivery confirmation), no retry (vs 3-attempt publish), no ordering guarantee (vs ordered by id). Best-effort, cleared client-side. 20. **Sender** client throttles (1 frame / 3s); **receiver** client auto-expires (~5s timer). Server is a stateless dumb router — keeping timers server-side would add per-connection state for an ephemeral signal.

**E. Reliability** 21. A slow/stuck consumer with an unbounded buffer grows without limit → **node OOM**. Bound caps blast radius; cost = frames dropped when full (best-effort push). 22. Push is best-effort; the dropped frame is recovered on the **pull path** — client reconnect + `?after=<last id>` refetches from PG. Durability lives on pull, speed on push. 23. `EXPIRE` on `n==1` stamps the window once (first send); later INCRs ride the same TTL = a **fixed** window, not sliding. 2× seam burst: a client can send 30 at the end of one window + 30 at the start of the next → 60 in a short span across the boundary. 24. Fixed-window (simple, seam-burst). Sliding-log (exact, memory-heavy — stores every timestamp). Token-bucket (burst-friendly, needs atomic read-modify-write → Lua). 25. App-level = **semantic** limits (per-conversation, per-plan, per-user) where identity/context is known. Gateway-level = coarse **volumetric** (per-IP) in front of everything. Both = defense in depth. 26. Ids are stable + client tracks a `seenIds` set; a duplicate id is ignored. Scenario: a message arrives via live WS _and_ again via `?after=` catch-up after a reconnect → dedup drops the second.

**F. Media** 27. Control plane = server signs URLs + records metadata (small JSON). Data plane = the actual **bytes go client ↔ S3 directly**, never through the server. Server never proxies file content. 28. Read gate (share-a-conversation JOIN) alone is forgeable: anyone could reference any `media_id` in a message. The **write gate** (only the uploader may attach a media*id) makes the link→media association trustworthy, so the read gate can trust it. 29. Anyone holding the URL can use it — it's not user-bound at S3; identity gates \_issuance*, not _use_. Mitigations: short TTL, TLS, (and post-upload server-side verification). 30. Presigning computes a signature **locally, no network call**. So the signed URL's host can be the browser-reachable one (`localhost:9000`) even though the server would _connect_ internally at `minio:9000` — the server never actually connects for presign. 31. Encryption-at-rest is transparent to authorized access — a compromised store _is_ authorized. Only **E2EE** (client-side encrypt, keys device-only) hides content from the server; cost = no server-side processing (thumbnails, scan, search).

**G. Sharding** 32. Read by key: `hash(key)` → the one shard, single-shard lookup, scales like writes. Without the key you don't know the shard → must hit all of them. 33. Scatter-gather = query **all** shards in parallel, merge. Latency = the slowest shard because you must wait for every shard to return before merging (tail latency dominates). 34. Sharded stores are optimized for keyed access, not full-text/relevance; a dedicated index is built for scatter-gather + ranking. Pattern = **CQRS** (write model = sharded DB, read model = search index), synced async. 35. Hot shards → high-cardinality/even key. Rebalancing on `%N` change → **consistent hashing**. Cross-shard joins/txns → co-locate related data under the same shard key. 36. **Object store** = distributed **erasure-coded** cluster (data+parity sharded across peer nodes, one namespace) or managed S3. **Database** = primary/replica (one writer, read replicas). Don't apply primary/replica to object stores.

**H. Observability** 37. Scrapes each instance's `/metrics` endpoint (here every 5s) = a **live snapshot** of current counter/gauge values. No history in the app; Prometheus stores the time series. 38. **R**ate, **E**rrors, **D**uration. `axum-prometheus` adds a tower layer instrumenting every route → request count (rate), status labels (errors), duration histogram — zero per-handler code. 39. `rate(counter[1m])` = per-second rate over 1m. `histogram_quantile(0.95, ...)` = p95 latency from histogram buckets. `up` = built-in per-target liveness (1/0) → detects a down instance. 40. Metrics are cheap aggregates (keep 100%). Traces are per-request and heavy → **sampled** (head or tail; keep errors + slow ones). Traces shine across **multiple services** via trace_id propagation; low ROI on a single binary.

**I. Kubernetes** 41. Cluster = the whole k8s system. Node = a machine running pods (in kind, a container). Pod = smallest deployable unit, wraps 1+ containers, has its own IP. Container = your running image. 42. Deployment = keep N pods of a version alive + rollouts. ReplicaSet = the per-version pod-counter (rarely touched, auto-made). Service = stable DNS/IP load-balancing to pods. 43. By **label selector** — the Service's `selector` matches pods carrying those labels, evaluated live. Not containment. 44. Postgres = single-writer; multiple pods each with their own storage (or racing one) = split-brain / corruption. App pods are **stateless** (state in PG/Redis/S3) so any replica is interchangeable. 45. `apply` is declarative — the manifest (`replicas: 1`) is the source of truth and overwrote the imperative `scale 5`. Fix = put the count in the yaml (or omit it + use HPA). 46. Surge a new pod, wait for its **readiness probe** to pass, only then route traffic + terminate an old pod. Readiness gates traffic so no request hits a still-booting pod. Needs ≥2 replicas for a true gap-free roll. 47. Same tag = k8s can't tell versions apart → won't roll (no spec change), can't roll back cleanly, and nodes may cache different images under one tag. Immutable tags = unique, traceable, rollback-able; the spec change triggers the roll. 48. `port-forward` picks **one** pod at setup and pins all traffic to it (a debug tunnel, no balancing). A real Service load-balances across pods via kube-proxy. 49. A Service is L4 — it picks a pod when the **connection** opens, then that whole connection sticks. A WebSocket = one long connection = pinned to one pod for its life. Two users on two pods can't share in-mem state → Redis pub/sub bridges them. 50. kind's node runs its **own containerd**, separate from host Docker. The 3 stores: (1) host Docker (build), (2) node containerd (`kind load` copies here; pods use it), (3) cloud registry (`docker push`, for real clusters). 51. The server is a **generic engine (mechanism)**; buckets/tables are **app-specific policy** applied externally so it's reusable + reproducible (in git), not manually exec'd into a live, ephemeral container. Also `mc` isn't in the minio server image. 52. **Workload Identity** (GCP) / **IRSA** (AWS): bind the pod's k8s ServiceAccount to a cloud IAM identity; the pod gets short-lived tokens at runtime — no static keys stored. 53. A k8s Secret is only **base64-encoded** (not encrypted) in etcd — anyone with cluster/etcd read sees it. Production = external secret store (Secret Manager / Vault) + workload identity, or passwordless IAM DB auth. 54. Default: SIGTERM → wait `terminationGracePeriodSeconds` (30s) → SIGKILL. To drain WS: handle SIGTERM (stop new conns, close existing cleanly), raise the grace period, rely on client reconnect (already built).

**J. Ownership & Borrowing** 55. It mutates connection state (issues commands) → needs `&mut`. Not owned (caller reuses it across the WS loop), not `&` (Redis calls take `&mut self`). 56. A `std::sync::MutexGuard` is **not `Send`**; holding it across `.await` would require the future to be `Send` to move across threads → won't compile. Scope the guard so it drops before the await. 57. Type safety — `AuthUser` can only come from the auth extractor, so a handler that takes it is _guaranteed_ authenticated. A raw `Uuid` could be any id from anywhere; the newtype encodes "this is an authenticated user." 58. `String` = owned, heap, growable; `&str` = borrowed view. DTOs own their data (serialized/deserialized, must outlive the request) → `String`. Error messages are compile-time constants → `&'static str` (no allocation).

**K. Async & Concurrency** 59. `select!` returns when the first branch completes; the other branch's future is **dropped** (cancelled). The code explicitly `abort()`s the _spawned_ send task since it's a separate task, not just a dropped future. 60. `on_message()` borrows `&mut pubsub` for the whole receive loop, which would block calling `subscribe()`. `split()` gives independent owned halves (sink for subscribe-control, stream for receiving) usable concurrently. 61. `send().await` waits for capacity (backpressure/blocking); `try_send` fails immediately if full. `local_fanout` uses `try_send` to stay **synchronous** (no await under a std Mutex) and to **shed** on a slow consumer instead of blocking fanout. 62. Outer `move` = the closure captures `metric_handle` by value (axum stores the closure). Inner `async move` = the returned future also owns it (polled later, after the closure returns). Both must own it because closure and future outlive each other's frames. 63. The send task is a real spawned task (own thread of execution) → must be explicitly `abort()`ed to stop it. The recv future is local to `select!` → dropping it (when select! ends) cancels it automatically.

**L. Traits, Types, Serde** 64. axum auto-runs the extractor before the handler — parses the token, checks Redis, and either injects `AuthUser(uuid)` or short-circuits with the `Rejection` (401). Handlers just take `AuthUser` as a param. 65. `{"type":"new_message","data":{...}}` — the variant name in `type`, payload in `data`. Adjacently-tagged is self-describing + robust vs untagged (which guesses by shape and can mis-parse ambiguous variants). 66. Rust `match` on the enum still compiles (arms unchanged) as long as they don't need the new variant — but a non-exhaustive match would force handling it; here unhandled server events fall through. The JS has no compiler — it must add an `if (ev.type === 'hello')` branch or the event is ignored. 67. Tuples are quick for ad-hoc row shapes without defining a `FromRow` struct; fine for small internal queries. A `#[derive(FromRow)]` struct is better when the shape is reused or has many columns.

**M. Macros & Compile-time** 68. Read at **compile time** — baked into the binary. To see edits you must **rebuild** (`cargo build` / `docker build`), not just restart. 69. `migrate!` embeds the SQL files into the binary **at compile time**, so they must exist in the build context → the Dockerfile needs `COPY migrations` before `cargo build`. At runtime `.run()` applies unapplied ones. 70. `query!` validates SQL against a live DB at compile time (needs a database or `.sqlx` cache). `query()` is checked at **runtime** → the build only needs the source, no database connection.

</details>

---

_When you've worked through these, we can go over any you want to dig into — then call the project done._
