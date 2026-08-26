# Requiem

Requiem is a deterministic, server-hosted social deduction platform built around a strict accepted-input log and replay-first architecture.  
Core philosophy: simulation is authoritative and reproducible, persistence is write-ahead and append-oriented, and clients are state-folders over ordered output rather than local authorities.

The game implemented on this stack is a long running (multiple real days) Death Note-inspired social deduction game with hidden roles, asymmetric role kits, notebooks (real/fake), investigations, bugs/wiretaps, kidnappings, prosecutions, polls, world events, and layered communication spaces (world channels, lounges, groupchats, custody/trial channels).

## Engine (lawliet)

`lawliet` is the deterministic Rust simulation core.

Key primitives:
- **Engine** owns `World`, dynamic `Config`, current virtual `Time`, and a min-heap based `Jobs` scheduler.
- **Action execution is two-pass**: validate (non-mutating dry run) then execute (mutating), with recursive sub-actions sharing one command buffer. Actions may reject at any level, and those
rejections must propagate to the top level without causing state corruption.
- **Temporal causality** is enforced by draining due jobs before the requested action at a timestamp.
- **Failure model** is explicit: inconsistent validate/execute behavior is treated as a crash-worthy invariant violation; recovery is by replay.

State model highlights:
- Players and organizations are both actors with state/modifier bitfields, links, ownership caches, and role-configured grants.
- Role config maps roles to default abilities, passives, notebooks, actor links, and world-channel profiles.
- Complex subsystems (polls, prosecution state machine, charge pools, notebook ownership/borrowing, incarceration/kidnapping) are modeled as first-class typed objects and actions.
- Actions and abilities are executed via static dispatch to avoid the pointer chasing costs of dynamic dispatch.

Information asymmetry model:
- Engine outputs are addressed to **recipients** (admin, player, viewport, log), not broadcast globally.
- **Viewport-owned history** means late access can receive backlog in order, while loss of access only stops future delivery.
- Client-visible state is monotonic; deletions are represented by archival/lifecycle commands, not state retraction.

## Server (yagami + yagami-runtime)

`yagami` is the authoritative multiplayer host; `yagami-runtime` is the child simulation process per game.

Architecture and concurrency:
- One game coordinator task owns a game’s accepted stream, runtime child pipes, history cache, clock, and connection fanout.
- Server state tracks live games, key handles, tickets, and connections; cancellation is token-tree based (game -> key -> connection).
- HTTP/WS edge is separated from game execution: handlers authenticate/attach, game task serializes game mutations.

Connection handling model:
- **Two-step join**: client exchanges durable key for a short-lived single-use ticket, then upgrades websocket with that ticket (key never rides ws URL).
- **Claim-at-upgrade semantics**: ticket claim is made during upgrade, making HTTP `101` authoritative (after `101`, failures are transport failures, not auth ambiguity).
- **Single-connection claim per ticket**: claim state is tracked in live connections; replayed/duplicate ticket attempts are rejected as invalid.
- **Guarded cleanup**: ticket claims are released by a drop guard, so failed upgrades and normal disconnects both reliably clear ticket/connection state.
- **Per-connection outbox + game inbox**: websocket task shuttles inbound frames to the game task; game task pushes filtered batches to each connection outbox.
- **Heartbeat/liveness**: inbound read timeout and outbound ping loop detect dead peers; protocol violations (e.g., binary frames/bad JSON) close the socket.
- **Backpressure policy**: if a connection outbox is full, that connection is cancelled and marked dropped rather than stalling global delivery.
- **Batch framing**: server emits `Initialize` (full reset/catch-up) and `Live` batches; oversized batches are chunked into `Continuation` frames preserving order.
- **Reply correlation model**: only the initiating connection receives the response pair, and it is carried on the terminal `Live` batch.
- **Sync/resync behavior**: new connections, privilege changes, rewind, and some authority transitions trigger full `Initialize` replay from log start.
- **Delivery watermarks**: each connection tracks per-viewport delivery position + membership, enabling exact late-entry backfill without duplicate replay.
- **Privilege-first sync**: every initialize replay begins with a connection-scoped privileges output so UI can gate controls before replayed data is applied.

Reliability and crash handling:
- **Write before ack**: accepted inputs are committed (idempotent by `(game_id, seq)`) before client acknowledgement.
- **Replay is the source of truth**: engine/sim state is rebuilt by re-feeding accepted inputs; history is a rebuildable cache.
- **Runtime pipe safety**: failed read/write/timeouts kill the child to prevent response-stream misalignment.
- **Crash recording**: crashing accepted sequences are persisted as inert repro artifacts.
- **Boot retry with backoff**: failed spawn/replay attempts retry up to a fixed bound; unrecoverable games are torn down.

Time and timeline control:
- Game time is sandboxed with a virtual clock anchored to wall time.
- Forward jumps are settled with null progression; backward jumps truncate accepted history and rebuild from target time.
- Key/connection handles are reconciled against rebuilt key state after replay, so revoked/rolled-back authority is not left live.

Hard problems solved in server/runtime:
- Keeping auth, deterministic replay, and live sockets consistent by separating **sim state** (replayable) from **live handles** (reconciled).
- Making time travel safe by treating rewind as authoritative log truncation + full rebuild instead of in-place mutation.
- Preserving deterministic behavior while still handling real-world failures (timeouts, child crashes, partial pipe exchanges).
- Making most race conditions structurally impossible rather than patching them over as they occur.
- Creating a delivery model capable of both live streaming + late join catch up.

## Client (amane)

`amane` is a Svelte client split into reusable core (`amane-core`) and host (`amane-webhost`).

Architecture:
- Transport contract is batch-oriented (commands + reply correlation in one ordered stream), avoiding dual-stream race conditions.
- Session owns connection lifecycle and reply waiters; game state is a pure command fold with per-view routing.
- Per-view backfill is indexed by viewport history and delivery watermarks, matching server delivery semantics at client granularity.
- Join retry policy mirrors server semantics: credential refusal is terminal, dropped live socket is retryable with a fresh ticket and catch-up replay.

Reliability posture:
- Re-sync uses in-place reset/rebuild of the same state object to keep UI bindings valid.
- Batch-apply errors are treated as unrecoverable local drift, resolved by reconnect/replay instead of speculative patching.

## Simplifications over time

Several difficult areas were intentionally reduced to fewer core mechanisms:
- Unified recipient/viewports replaced special-case delivery paths for late join and re-entry.
- Policy recomputation and composable permission rules replaced scattered one-off permission updates.
- Runtime replay pipeline unifies engine and simulation reconstruction, reducing split-brain risk.
- The concurrency model for a single game was collapsed from a race-prone multi-task model into one game task, exploiting a property of tokio select
arms where a selected arm runs to completion with no interleaving between other arms regardless of the existence of await points within that arm.

## Notable implemented features

- Multi-game hosting with per-game runtime isolation.
- Deterministic replay across engine + server-side simulation state.
- Privilege system with key capabilities, actor scopes, supervise/admin controls, and ticketed WS claims.
- Server-side time travel (forward catch-up + backward truncation/rebuild).
- Rich game systems: role kits, passives, notebooks, bugs, polls, prosecution phases, kidnappings, and scoped communication artifacts.

## Project scale

Current codebase scale is substantial and multi-language:
- **Rust**: 255 files, ~37k LOC (engine + server + runtime + shared wire/types).
- **TypeScript**: 34 files, ~6k LOC.
- **Svelte**: 108 files, ~8k LOC.

[requiem-dn.dev](https://requiem-dn.dev/)
