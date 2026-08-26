# Requiem

Requiem is a deterministic, server-hosted social deduction platform built around a strict event-log architecture.  
The design philosophy is: keep simulation authoritative and replayable, keep transport/persistence separate, and keep clients as renderers of ordered server output.

The game implemented on this stack is a multi-day Death Note-inspired deduction game with hidden roles, asymmetric abilities, notebooks, investigations, kidnappings, prosecutions, world events, and role-scoped/private communication spaces.

## Engine (lawliet)

`lawliet` is the core simulation engine in Rust. It owns world state, dynamic config, and scheduled jobs, and executes actions deterministically (validate first, then mutate).  
State is advanced through typed actions and emitted as addressed commands (system/actor/viewport), which are the only thing downstream layers need to deliver UI state.  
The engine is intentionally crash-recoverable by replay: timelines are reconstructed from action history rather than snapshots.

## Server (yagami + yagami-runtime)

`yagami` is the authoritative multiplayer server. It hosts many game instances, manages auth/keys/tickets, handles websocket/HTTP surfaces, persists accepted input streams, and routes batches to connections by privilege + viewport access.  
Each game runs through `yagami-runtime`, a child process that combines `lawliet` with simulation-side state (profiles/keys/name generation), so replaying the accepted stream rebuilds both engine state and emitted output stream identically.  
Time travel/rewind is server-orchestrated by truncating and replaying accepted inputs.

## Client (amane)

`amane` is a Svelte client split into a reusable core (`amane-core`) and a browser host (`amane-webhost`).  
It applies ordered output batches into per-view state, enforces no local authority, and treats reconnection as full state reconstruction from server replay.

[requiem-dn.dev](https://requiem-dn.dev/)
