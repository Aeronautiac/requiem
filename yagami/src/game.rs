// yagami's game task: one game's coordinator. it owns the accepted input stream (the single durable
// source of truth), spawns and feeds the yagami-runtime child process, walks its outputs into the
// in-memory history cache (rebuilt from the stream on every boot), and routes replies + batches to
// connections under a per-key privilege set.
//
// The simulation (engine + sim state: keys, profiles, all name drawing) lives in the runtime child.
// yagami holds only what the runtime cannot: the accepted stream, the history cache, the live
// connection/key-handle registry, the clock, time travel, and the admin timeline (LogAction).
//
// There is no rehydrate/rebuild split: a boot always re-feeds the whole (possibly truncated)
// accepted stream to a fresh child, which reconstructs engine + sim + outputs identically. Time
// travel is yagami's -- the runtime knows nothing of rewind.

use std::{
    collections::HashMap,
    env::current_exe,
    io::ErrorKind,
    process::Stdio,
    time::Duration,
};

use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, Null},
    command::Command,
    common::{Time, ViewportKey},
};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    select,
    sync::mpsc,
    time::{Instant, Interval, MissedTickBehavior, interval, sleep},
};
use tokio_util::sync::CancellationToken;

use crate::{
    auth::{Key, KeyHandle, Privileges, Ticket, to_flags},
    constants::{ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    delivery::{History, game_clock_output, log_action_output},
    state::{GameId, WrappedServerState, lock_state},
    wire::{
        ActionOutcome, AdminControl, ControlOutcome, ControlResponse, ExecOutcome, MetaControl,
        Output, OutputData, ResponsePair, ServerInput, SimControl, SimControlData, SimOutput,
    },
};

// the runtime child's pipe types.
use yagami_runtime::{PipeFrame, RuntimeInput, RuntimeOutput};

pub fn to_line<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json + "\n",
        Err(e) => {
            eprintln!("failed to serialize for the runtime pipe: {e} -- aborting");
            std::process::abort()
        }
    }
}

// carries the source ticket so the game task can route replies and enforce permissions.
pub struct InputEnvelope {
    pub ticket: Ticket,
    pub input: ServerInput,
}

// "do this thing" rather than "i want to do this thing".
// external server inputs. some other part of the server wants the game to do something.
pub enum GameCommand {
    // synchronize the connection to the game state given its current privileges
    Sync { ticket: Ticket },
    // run one sim control with no connection attached, and resolve the oneshot with the outcome.
    // used by create_game to mint the admin key: the runtime generates it (deterministic), and the
    // caller awaits the KeyCreated reply before responding.
    BootstrapControl {
        control: SimControl,
        reply: tokio::sync::oneshot::Sender<ControlOutcome>,
    },
}

#[allow(clippy::large_enum_variant)]
pub enum GameInput {
    GameCommand(GameCommand),
    ServerInput(InputEnvelope),
}

// the only shape a runtime dispatch can end in. a successful dispatch returns the deserialized
// runtime output; anything else is a write or read failure.
enum DispatchError {
    Write,
    Read,
}

// retryable is the "nothing wrong with the fd, the syscall just didn't finish" class: the call was
// interrupted (EINTR, retry now) or the pipe was momentarily not ready (WouldBlock, retry when
// ready). everything else -- broken pipe, reset, aborted, not connected, unexpected eof, wrote
// zero, timed out -- means the pipe relationship is genuinely gone, so dispatch surfacing it as an
// error (and booting a fresh child) is the only thing that makes progress.
fn is_retryable(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock)
}

// The hard part about time sandboxing is handling time travel.
// We can visualize a real start time, and an "anchor".
// The anchor is the start position of virtual time, which is offset some distance from the real start time.
// On time travel, we ask: "how much do I need to shift the anchor such that virtual time becomes
// the target time?"
// This is answered by getting the distance between the current virtual time, and the target time.
// offset = target - now()
// We shift the anchor by this distance. This works with any sequence of time travel events.
struct GameClock {
    start: Instant,
    anchor: i128,
}

impl GameClock {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            anchor: 0,
        }
    }

    fn now(&self) -> Time {
        // it isnt possible for the anchor to cause overflow. the target time can only ever be >= 0.
        (self.start.elapsed().as_millis() as i128 + self.anchor) as Time
    }

    fn go_to(&mut self, target: Time) {
        let offset: i128 = target as i128 - self.now() as i128;
        self.anchor += offset;
    }
}

struct Game {
    // identity / routing
    game_id: GameId,
    server_state: WrappedServerState,
    cancel: CancellationToken,
    events: mpsc::UnboundedReceiver<GameInput>,

    // state
    last_reached: Time, // the latest timestamp executed by the engine, not necessarily
    // the highest ever executed. this distinction is important because time travel may change what
    // "the end of the timeline" is.
    clock: GameClock,            // sandboxed source of action timestamps
    accepted: Vec<ServerInput>, // the single durable source of truth; replayed on boot
    history: History,           // the in-memory command log every connection walks (a cache)
    // the sim's key set, intercepted from the runtime's KeyRoster outputs. the authoritative copy
    // lives in the runtime; this cache is what yagami uses for meta-control auth and key-handle
    // reconciliation. updated whenever a KeyRoster passes through.
    keys_cache: HashMap<Key, Privileges>,
    // the world-data viewport, learned from the engine's MapViewport(WorldData) announcement riding
    // the output stream. the runtime owns the authoritative copy; yagami mirrors it so it can
    // address its own world-data outputs (the GameClock) correctly.
    data_viewport: Option<ViewportKey>,

    // the runtime child. held here so it stays alive for as long as this task does:
    // dropping it would trigger kill_on_drop.
    child: Option<tokio::process::Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,

    tick: Interval,
}

impl Game {
    fn new(
        game_id: GameId,
        server_state: WrappedServerState,
        cancel: CancellationToken,
        events: mpsc::UnboundedReceiver<GameInput>,
        initial_accepted: Vec<ServerInput>,
    ) -> Self {
        let mut tick = interval(Duration::from_secs(NULL_TICK_INTERVAL));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            game_id,
            server_state,
            cancel,
            events,
            last_reached: 0,
            clock: GameClock::new(),
            accepted: initial_accepted,
            history: History::new(game_id),
            keys_cache: HashMap::new(),
            data_viewport: None,
            child: None,
            stdin: None,
            stdout: None,
            tick,
        }
    }

    // ===== LOW-LEVEL COMMS ===== //

    // write one line to the runtime under a timeout, retrying errors that a retry fixes (EINTR).
    // returns Write on a fatal write failure or a timeout.
    async fn write_line(&mut self, line: &str) -> Result<(), DispatchError> {
        let stdin = self.stdin.as_mut().expect("stdin present during dispatch");
        loop {
            match select! {
                res = stdin.write_all(line.as_bytes()) => res,
                _ = sleep(Duration::from_secs(ENGINE_TIMEOUT)) => return Err(DispatchError::Write),
            } {
                Ok(()) => return Ok(()),
                Err(e) if is_retryable(&e) => continue,
                Err(_) => return Err(DispatchError::Write),
            }
        }
    }

    // read one line from the runtime under a timeout. returns Read on EOF or a fatal read failure.
    async fn read_line(&mut self) -> Result<String, DispatchError> {
        let stdout = self
            .stdout
            .as_mut()
            .expect("stdout present during dispatch");
        loop {
            match select! {
                res = stdout.next_line() => res,
                _ = sleep(Duration::from_secs(ENGINE_TIMEOUT)) => return Err(DispatchError::Read),
            } {
                Ok(Some(text)) => return Ok(text),
                Ok(None) => return Err(DispatchError::Read), // EOF
                Err(e) if is_retryable(&e) => continue,
                Err(_) => return Err(DispatchError::Read),
            }
        }
    }

    // write a frame, then read and deserialize the response. the whole exchange is linear and
    // completes or fails as one unit.
    async fn dispatch(
        &mut self,
        input: &RuntimeInput,
        caller: Option<&Key>,
    ) -> Result<RuntimeOutput, DispatchError> {
        let frame = PipeFrame {
            input: input.clone(),
            caller: caller.cloned(),
        };
        let line = to_line(&frame);
        self.write_line(&line).await?;

        let text = self.read_line().await?;
        match serde_json::from_str(&text) {
            Ok(output) => Ok(output),
            // an undeserializable line means the two binaries disagree about the wire format. a
            // deploy mistake, not a runtime condition.
            Err(e) => {
                eprintln!("runtime output failed to deserialize: {e} -- aborting");
                std::process::abort()
            }
        }
    }

    // ===== PROCESS MANAGEMENT ===== //

    fn spawn_runtime(&mut self) -> (ChildStdin, ChildStdout, tokio::process::Child) {
        let mut child = tokio::process::Command::new(
            current_exe()
                .expect("failed to get current exe")
                .parent()
                .expect("failed to get parent path")
                .join(format!("yagami-runtime{}", std::env::consts::EXE_SUFFIX)),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("failed to boot yagami-runtime");

        let stdin = child.stdin.take().expect("child stdin piped");
        let stdout = child.stdout.take().expect("child stdout piped");
        (stdin, stdout, child)
    }

    // boot: spawn a fresh runtime and bring it up to the state the old one had reached by re-feeding
    // the accepted stream. there is no rehydrate/rebuild split -- a boot always replays EVERY accepted
    // input (engine + sim) so the runtime reconstructs engine state, sim state, and the output
    // stream identically. history is a cache, so it is discarded and rebuilt fresh from the outputs
    // the runtime emits during replay.
    //
    // `truncate_to` is set by a backward time-travel jump: the accepted stream is first truncated to
    // everything up to the target, so the replay reconstructs only the timeline that still exists.
    async fn boot(&mut self, truncate_to: Option<Time>) {
        // TODO:
        // exponential backoff between retries, and eventual total game failure once retries exceed
        // some bound, rather than retrying forever.
        if let Some(target) = truncate_to {
            self.truncate_accepted(target);
        }

        loop {
            let (stdin, stdout, child) = self.spawn_runtime();
            self.child = Some(child);
            self.stdin = Some(stdin);
            self.stdout = Some(BufReader::new(stdout).lines());

            // history is a cache: discard it and rebuild from the replay's outputs.
            self.history = History::new(self.game_id);

            let mut ok = true;
            let mut last_time: Time = 0;

            // re-feed the accepted stream. every input is server-issued (caller None): the stream
            // was already authorized when first accepted, so replay trusts it. the runtime
            // reconstructs engine + sim state and emits the output stream, which we append to
            // history.
            for input in self.accepted.clone() {
                let Some(runtime_input) = to_runtime_input(&input) else {
                    // meta controls are never stored in accepted.
                    continue;
                };
                match self.dispatch(&runtime_input, None).await {
                    Ok(output) => {
                        for out in &output.outputs {
                            if out.time > last_time {
                                last_time = out.time;
                            }
                        }
                        self.update_server_projections(&output.outputs);
                        self.history.append(output.outputs);
                    }
                    Err(_) => {
                        ok = false;
                        break;
                    }
                }
            }

            if ok {
                self.last_reached = last_time;
                // reconcile live key handles + privilege cache against the rebuilt sim key set.
                self.reconcile_key_handles();
                // anchor every client's game time on the rebuilt timeline.
                let at = self.append_game_clock();
                self.history.broadcast(&self.server_state, at, None);
                break;
            }

            // the fresh child failed right out of the gate. discard it and try again.
            self.child = None;
            self.stdin = None;
            self.stdout = None;
        }
    }

    // ===== INPUT HANDLING ===== //

    async fn handle_input(&mut self, input: GameInput) {
        match input {
            GameInput::GameCommand(GameCommand::Sync { ticket }) => {
                self.handle_sync(ticket);
            }
            GameInput::GameCommand(GameCommand::BootstrapControl { control, reply }) => {
                let outcome = self.process_bootstrap_control(control).await;
                let _ = reply.send(outcome);
            }
            GameInput::ServerInput(envelope) => self.handle_server_input(envelope).await,
        }
    }

    // a command for the game itself, handled immediately. no runtime involved.
    fn handle_sync(&mut self, ticket: Ticket) {
        self.history
            .deliver_sync(&self.server_state, ticket, self.clock.now());
    }

    // run one sim control with no connection attached, and return the outcome. used by create_game
    // to mint the admin key before any connection exists: dispatch, append, save, return.
    //
    // the bootstrap control's time is left exactly as sent (0) -- it is part of the game's creation
    // and MUST survive any rewind (which can only ever go to a time >= 0), so it is never re-stamped
    // with the live clock.
    async fn process_bootstrap_control(&mut self, mut control: SimControl) -> ControlOutcome {
        control.time = 0;
        let runtime_input = RuntimeInput::Sim(control.clone());
        match self.dispatch(&runtime_input, None).await {
            Ok(output) => {
                self.update_server_projections(&output.outputs);
                self.history.append(output.outputs);
                self.accepted
                    .push(ServerInput::Control(AdminControl::Sim(control)));
                // the minted key must reach the shell (keys + key_handles) before the caller's
                // get_ticket can resolve it.
                self.reconcile_key_handles();
                match output.reply {
                    Some(r) => match r.outcome {
                        ExecOutcome::Control(o) => o,
                        _ => ControlOutcome::Denied,
                    },
                    None => ControlOutcome::Denied,
                }
            }
            Err(_) => {
                self.boot(None).await;
                ControlOutcome::Denied
            }
        }
    }

    async fn handle_server_input(&mut self, envelope: InputEnvelope) {
        let InputEnvelope { ticket, input } = envelope;

        // resolve the caller's key once -- used for runtime auth and for the reply.
        let caller_key = {
            let server_state = lock_state(&self.server_state);
            server_state
                .games
                .get(&self.game_id)
                .and_then(|game| game.tickets.get(&ticket))
                .cloned()
        };

        match input {
            ServerInput::Control(AdminControl::Meta(MetaControl::GoToTime { time })) => {
                // meta controls are yagami's own time-travel mechanic and never reach the runtime.
                // authorize via the live keys map (the runtime holds the authoritative keys, but
                // meta never touches sim state, so the live cache suffices for the Administer check).
                let outcome = if !self.authorize_meta(&ticket) {
                    ControlOutcome::Denied
                } else {
                    self.go_to_time(time).await;
                    ControlOutcome::Ok(ControlResponse::TimeSet)
                };
                let pair = ResponsePair {
                    response: ExecOutcome::Control(outcome),
                    input: ServerInput::Control(AdminControl::Meta(MetaControl::GoToTime { time })),
                };
                self.history
                    .broadcast(&self.server_state, self.history.head(), Some((ticket, pair)));
            }
            ServerInput::Control(AdminControl::Sim(mut control)) => {
                // override the sim time at read time, as we do for engine actions.
                control.time = self.clock.now();

                let runtime_input = RuntimeInput::Sim(control.clone());
                match self.dispatch(&runtime_input, caller_key.as_ref()).await {
                    Ok(output) => {
                        self.update_server_projections(&output.outputs);
                        let at = if output.outputs.is_empty() {
                            self.history.head()
                        } else {
                            self.history.append(output.outputs)
                        };
                        // fold the key set change into the shell (keys + key_handles): a newly
                        // created key must be connectable, a privilege change must take effect, a
                        // revocation must tear down the handle + connection.
                        self.reconcile_key_handles();
                        // a privilege change (capabilities/scope) or revocation resyncs the
                        // affected key's live connections so their clients see the new standing.
                        if let SimControlData::SetCapabilities { key, .. }
                        | SimControlData::SetActorScope { key, .. }
                        | SimControlData::RevokeKey { key } = &control.data
                        {
                            self.resync_key(key);
                        }
                        // this sim control was accepted, so it is part of the sim's state and must
                        // be replayed on the next boot.
                        self.accepted
                            .push(ServerInput::Control(AdminControl::Sim(control)));

                        let pair = output.reply.expect("runtime always replies to a sim input");
                        self.history
                            .broadcast(&self.server_state, at, Some((ticket, pair.into_pair())));
                    }
                    Err(_) => {
                        // the runtime crashed mid-control: reboot, then settle the caller.
                        self.boot(None).await;
                        let pair = ResponsePair {
                            response: ExecOutcome::Control(ControlOutcome::Denied),
                            input: ServerInput::Control(AdminControl::Sim(control)),
                        };
                        self.history
                            .broadcast(&self.server_state, self.history.head(), Some((ticket, pair)));
                    }
                }
            }
            ServerInput::Action(mut request) => {
                // override the client's reported value. only the game task truly knows the game's
                // virtual clock, and a client cannot be trusted.
                request.timestamp = self.clock.now();

                // pre-authorize at the yagami level so a denied request is recorded for the host
                // and never reaches the runtime (and is not part of the accepted stream). the
                // runtime's own auth is the real gate.
                if !self.authorize_action(&ticket, &request) {
                    let at = self.append_log_action(&request, ActionOutcome::Denied);
                    let pair = ResponsePair {
                        response: ExecOutcome::Action(ActionOutcome::Denied),
                        input: ServerInput::Action(request),
                    };
                    self.history
                        .broadcast(&self.server_state, at, Some((ticket, pair)));
                    return;
                }

                // the reply echoes the request, so clone for the dispatch and keep the original to
                // record as accepted.
                let reply_input = ServerInput::Action(request.clone());
                let runtime_input = RuntimeInput::Action(request.clone());
                match self.dispatch(&runtime_input, caller_key.as_ref()).await {
                    Ok(output) => {
                        self.update_server_projections(&output.outputs);
                        let at = self.history.append(output.outputs);
                        // record the accepted action for the admin timeline.
                        let outcome = match &output.reply {
                            Some(r) => match &r.outcome {
                                ExecOutcome::Action(o) => o.clone(),
                                _ => ActionOutcome::EnginePanic,
                            },
                            None => ActionOutcome::EnginePanic,
                        };
                        self.append_log_action_at(&request, outcome.clone(), at);

                        // this input ran and was accepted, so it is part of the engine's state and
                        // must be replayed on the next boot.
                        self.accepted.push(reply_input.clone());

                        let pair = ResponsePair {
                            response: ExecOutcome::Action(outcome),
                            input: reply_input,
                        };
                        self.history
                            .broadcast(&self.server_state, at, Some((ticket, pair)));
                    }
                    Err(_) => {
                        // the runtime crashed mid-action: reboot, then tell whoever sent it the
                        // engine panicked and record the crash for the host.
                        self.boot(None).await;
                        let at = self.append_log_action(&request, ActionOutcome::EnginePanic);
                        let pair = ResponsePair {
                            response: ExecOutcome::Action(ActionOutcome::EnginePanic),
                            input: ServerInput::Action(request),
                        };
                        self.history
                            .broadcast(&self.server_state, at, Some((ticket, pair)));
                    }
                }
            }
        }
    }

    // ===== AUTHORIZATION ===== //

    // actions are pre-authorized at the yagami level so denials can be recorded without reaching the
    // runtime. the runtime's own auth is the real gate; this is a mirror for the denial path.
    fn authorize_action(&self, ticket: &Ticket, request: &ActionRequest) -> bool {
        lock_state(&self.server_state)
            .games
            .get(&self.game_id)
            .and_then(|game| game.privileges(ticket))
            .is_some_and(|privileges| privileges.can_act_as(&request.actor))
    }

    // meta controls operate on the timeline, not the sim. the runtime never sees them, so yagami
    // authorizes via the keys cache (intercepted from the runtime's KeyRoster outputs). Administer
    // is required.
    fn authorize_meta(&self, ticket: &Ticket) -> bool {
        let server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get(&self.game_id) else {
            return false;
        };
        let Some(key) = game.tickets.get(ticket) else {
            return false;
        };
        self.keys_cache
            .get(key)
            .is_some_and(|p| p.administers())
    }

    // intercept the sim-derived facts riding the runtime's outputs: the key set (from KeyRoster,
    // full-snapshot replacement -- the authoritative copy lives in the runtime, this is what yagami
    // uses for meta-control auth and key-handle reconciliation) and the world-data viewport (from
    // the engine's MapViewport(WorldData) announcement, so yagami can address its own world-data
    // outputs like the GameClock).
    fn update_server_projections(&mut self, outputs: &[Output]) {
        for output in outputs {
            match &output.data {
                OutputData::Sim(SimOutput::KeyRoster { keys }) => {
                    self.keys_cache = keys
                        .iter()
                        .map(|(key, priv_set)| {
                            (
                                key.clone(),
                                Privileges {
                                    actors: priv_set.actors.clone(),
                                    capabilities: to_flags(&priv_set.capabilities),
                                },
                            )
                        })
                        .collect();
                }
                OutputData::Engine(Command::MapViewport { viewport, kind })
                    if *kind == lawliet_types::viewport::ViewportKind::WorldData =>
                {
                    self.data_viewport = Some(*viewport);
                }
                _ => {}
            }
        }
    }

    // ===== TIME TRAVEL ===== //

    // GoToTime: move the game to a past (or future) instant.
    //
    // Going FORWARD costs nothing structural: the engine is already at the current time and will
    // reach `target` naturally on its next tick, so all we do is advance the sandbox clock to it
    // and drive a null tick so time-based state settles there.
    //
    // Going BACKWARD is a rebuild: we truncate the accepted stream to everything up to the target,
    // set the clock there, and boot the runtime fresh from that base -- so it replays only the
    // state that existed up to the target time. history (a cache) is discarded and rebuilt; the
    // live key handles are reconciled against the rebuilt key set.
    //
    // It should also be noted that back to some point in time does not go to BEFORE that time.
    // Everything that happened AT that time remains.
    async fn go_to_time(&mut self, target: Time) {
        let now = self.clock.now();
        self.clock.go_to(target);

        // forward jump: no rebuild needed -- the engine is already at the current time. we still
        // drive it forward to `target` with a null tick so time-based state actually settles there,
        // rather than waiting for the next scheduled tick.
        if target >= now {
            self.last_reached = target;
            let request = ActionRequest {
                actor: ActionActor::System,
                timestamp: self.clock.now(),
                payload: Action::Null(Null {}),
            };
            let runtime_input = RuntimeInput::Action(request);
            if let Ok(output) = self.dispatch(&runtime_input, None).await {
                self.update_server_projections(&output.outputs);
                let at = self.history.append(output.outputs);
                self.history.broadcast(&self.server_state, at, None);
            }
            // the clock was wound forward; re-anchor every client's game time on the new baseline.
            let at = self.append_game_clock();
            self.history.broadcast(&self.server_state, at, None);
            return;
        }

        // backward jump: truncate the accepted stream to the target and rebuild from there.
        self.last_reached = target;
        self.boot(Some(target)).await;
        // every connection's view of the world was built on a timeline that no longer exists: reset
        // each one and replay the rebuilt history from the start.
        self.history
            .resync_all(&self.server_state, self.clock.now());
    }

    // ===== REWIND HELPERS ===== //

    // truncate the accepted stream to everything up to `target` (inclusive). sim controls carry
    // their own time; engine actions carry their timestamp. meta controls are never stored.
    fn truncate_accepted(&mut self, target: Time) {
        self.accepted.retain(|input| match input {
            ServerInput::Action(request) => request.timestamp <= target,
            ServerInput::Control(AdminControl::Sim(control)) => control.time <= target,
            ServerInput::Control(AdminControl::Meta(_)) => false,
        });
    }

    // reconcile the live key handles + privilege cache against the rebuilt key set. keys that no
    // longer exist in the sim (rolled back by a rewind) are torn down the same way a revocation
    // tears them down: cancel the token + drop the socket, so clients are TOLD they were
    // disconnected. keys the sim now holds but yagami has no handle for get a fresh one.
    //
    // the sim's key set is reconstructed by the runtime during boot; yagami learns it from the last
    // KeyRoster the runtime emitted (the final roster reflects the rebuilt set).
    fn reconcile_key_handles(&mut self) {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return;
        };

        // the rebuilt key set + privileges from the keys cache (intercepted from the runtime's
        // KeyRoster outputs during boot replay).
        game.keys = self.keys_cache.clone();

        // tear down handles for keys no longer in the sim.
        let dropped: Vec<Key> = game
            .key_handles
            .keys()
            .filter(|key| !game.keys.contains_key(key))
            .cloned()
            .collect();
        for key in dropped {
            if let Some(handle) = game.key_handles.remove(&key) {
                handle.cancel.cancel();
                for ticket in &handle.tickets {
                    game.tickets.remove(ticket);
                    if let Some(conn) = game.connections.get_mut(ticket) {
                        conn.dropped = true;
                    }
                }
            }
        }

        // fresh handles for keys the sim holds but yagami has no handle for.
        let cancel = game.cancel.clone();
        let missing: Vec<Key> = game
            .keys
            .keys()
            .filter(|key| !game.key_handles.contains_key(key))
            .cloned()
            .collect();
        for key in missing {
            game.key_handles.insert(
                key.clone(),
                KeyHandle {
                    cancel: cancel.child_token(),
                    tickets: std::collections::HashSet::new(),
                },
            );
        }
    }

    // deliver a full resync to every live connection held by `key` -- the "Initialize batch" reset
    // that rebuilds the client's view under the key's current privileges. used on a privilege change.
    fn resync_key(&self, key: &Key) {
        let tickets: Vec<Ticket> = {
            let server_state = lock_state(&self.server_state);
            match server_state.games.get(&self.game_id) {
                Some(game) => game
                    .tickets
                    .iter()
                    .filter(|(_, k)| *k == key)
                    .map(|(ticket, _)| ticket.clone())
                    .collect(),
                None => Vec::new(),
            }
        };
        for ticket in tickets {
            self.history
                .deliver_sync(&self.server_state, ticket, self.clock.now());
        }
    }

    // ===== HISTORY HELPERS ===== //

    // append the admin-visible record of one action request and its outcome to history, and return
    // where it landed.
    fn append_log_action(&mut self, request: &ActionRequest, outcome: ActionOutcome) -> usize {
        self.history
            .append(vec![log_action_output(request, outcome, request.timestamp)])
    }

    // append a log action right after the engine commands of the action it describes (at `at`), so
    // the record rides the same batch as the commands.
    fn append_log_action_at(
        &mut self,
        request: &ActionRequest,
        outcome: ActionOutcome,
        _at: usize,
    ) {
        self.append_log_action(request, outcome);
    }

    // snapshot the game's current clock into history as a live GameClock, aimed at the world-data
    // viewport (learned from the runtime's output stream). returns the position to walk from when
    // broadcasting.
    fn append_game_clock(&mut self) -> usize {
        let start = self.history.head();
        let Some(data_viewport) = self.data_viewport else {
            // the engine has not announced the world-data viewport yet.
            return start;
        };
        let now = self.clock.now();
        let sent_at = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .expect("wall clock before epoch")
            .as_millis();
        self.history
            .append(vec![game_clock_output(data_viewport, now, sent_at)]);
        start
    }

    // ===== TICK ===== //
    // drive time forward. the engine only advances when an action arrives, so without this nothing
    // time-based ever happens. a Null does nothing itself and only exists to carry the clock.
    async fn tick(&mut self) {
        let request = ActionRequest {
            actor: ActionActor::System,
            timestamp: self.clock.now(),
            payload: Action::Null(Null {}),
        };
        let runtime_input = RuntimeInput::Action(request);
        if let Ok(output) = self.dispatch(&runtime_input, None).await {
            self.update_server_projections(&output.outputs);
            let at = self.history.append(output.outputs);
            self.history.broadcast(&self.server_state, at, None);
        }
    }

    // ===== LOOP ===== //

    async fn run(mut self) {
        self.boot(None).await;

        loop {
            select! {
                Some(input) = self.events.recv() => {
                    self.handle_input(input).await;
                }
                _ = self.tick.tick() => {
                    self.tick().await;
                }
                _ = self.cancel.cancelled() => {
                    break;
                }
            }
        }
        // dropping `self` runs Drop, which tears the game out of state and cancels it, and dropping
        // the child reaps the runtime (kill_on_drop).
    }
}

impl Drop for Game {
    // Graceful teardown whether `run` finishes or the task panics mid-way: on either path `self`
    // is dropped and this runs. Remove the game from the registry and cancel its token tree so its
    // connections' sockets close (and their clients are told they were disconnected) instead of
    // leaking a half-alive game. Nothing here is async, and the fields needed are sync, so Drop can
    // do this without an async teardown step that a panic would skip.
    fn drop(&mut self) {
        self.cancel.cancel();
        let mut state = lock_state(&self.server_state);
        state.games.remove(&self.game_id);
    }
}

// permission enforcement, input executions, live client updates, and runtime process management.
pub async fn game(
    state: WrappedServerState,
    game_id: GameId,
    events: mpsc::UnboundedReceiver<GameInput>,
    cancel: CancellationToken,
    initial_accepted: Vec<ServerInput>,
) {
    Game::new(game_id, state, cancel, events, initial_accepted).run().await;
}

// convert a ServerInput into the RuntimeInput the runtime actually processes, if it is one the
// runtime sees. meta controls are yagami's concern (time travel) and never reach the runtime.
fn to_runtime_input(input: &ServerInput) -> Option<RuntimeInput> {
    match input {
        ServerInput::Action(a) => Some(RuntimeInput::Action(a.clone())),
        ServerInput::Control(AdminControl::Sim(sim)) => Some(RuntimeInput::Sim(sim.clone())),
        ServerInput::Control(AdminControl::Meta(_)) => None,
    }
}
