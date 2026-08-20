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

use std::{collections::HashMap, env::current_exe, io::ErrorKind, process::Stdio, time::Duration};

use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, Null},
    command::Command,
    common::{Time, Version, ViewportKey},
};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    select,
    sync::{mpsc, oneshot},
    time::{Instant, Interval, MissedTickBehavior, interval, sleep},
};
use tokio_util::sync::CancellationToken;

use crate::{
    auth::{Key, KeyHandle, Privileges, Ticket, to_flags},
    constants::{BOOT_MAX_RETRIES, BOOT_RETRY_BASE_MS, ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    delivery::{History, game_clock_output},
    state::{GameId, WrappedServerState, insert_handle, lock_state},
    store::{GameMeta, Store, wall_now},
    wire::{
        ActionOutcome, AdminControl, ControlOutcome, ControlResponse, ExecOutcome, MetaControl,
        Output, OutputData, ResponsePair, ServerInput, SimControl, SimControlData, SimOutput,
        VersionedInput,
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
}

// why a fresh game could not be created (so the platform admin is told, not left hanging).
#[derive(Debug, Clone, Copy)]
pub enum InitError {
    // the runtime could not be brought up / the initial replay never succeeded.
    BootFailed,
}

// how a game task is started. a fresh game has no durable record yet: the task itself generates
// the engine seed, builds InitializeEngine, prepends it to `creation_pack` (the admin-key minting
// and any other creation inputs), boots, and only then writes itself to the DB -- so a game that
// fails to boot is never written. `creation_reply` carries the result back to create_game: the
// game id plus the RESPONSES to the creation pack's inputs (one per input that produced one), so
// the caller can pull e.g. the minted admin key out of them. a resumed game already exists in the
// DB.
pub enum GameStart {
    Fresh {
        creation_pack: Vec<ServerInput>,
        creation_reply: oneshot::Sender<Result<(GameId, Vec<ExecOutcome>), InitError>>,
    },
    Resumed {
        game_id: GameId,
        inputs: Vec<VersionedInput>,
        keys: HashMap<Key, Privileges>,
        start_clock: Time,
    },
}

#[allow(clippy::large_enum_variant)]
pub enum GameInput {
    GameCommand(GameCommand),
    ServerInput(InputEnvelope),
}

// the only shape a runtime dispatch can end in. a successful dispatch returns the deserialized
// runtime output; anything else is a write or read failure -- and by the time it is returned,
// dispatch has already killed the child (see kill_runtime). the invariant a caller must uphold:
// after an Err, reboot before dispatching again, or the next dispatch will find no pipe.
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

    // resume the clock at an initial virtual time: a fresh game starts at 0; a resumed game at its
    // stored virtual time plus downtime (the caller computes it). since `start` resets each process,
    // setting the anchor to the target makes now() == target here and keep tracking real time.
    fn at(initial: Time) -> Self {
        Self {
            start: Instant::now(),
            anchor: initial as i128,
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
    game_id: GameId, // sentinel 0 until a fresh game writes its durable row
    server_state: WrappedServerState,
    store: Store,
    inbox: mpsc::UnboundedSender<GameInput>,
    cancel: CancellationToken,
    events: mpsc::UnboundedReceiver<GameInput>,

    // durability lifecycle: a fresh game is not durable until its first boot succeeds and it
    // writes itself to the DB; a resumed game already is. until registered, boot() skips
    // persisting progress (there is no row to update yet).
    registered: bool,
    // the create_game handshake (Fresh only): reports (id, the init responses) on success, or an
    // InitError if boot never came up.
    creation_reply: Option<oneshot::Sender<Result<(GameId, Vec<ExecOutcome>), InitError>>>,
    // the responses the runtime produced for the creation pack's inputs during the first boot.
    creation_responses: Vec<ExecOutcome>,

    // state
    last_reached: Time, // the latest timestamp executed by the engine, not necessarily
    // the highest ever executed. this distinction is important because time travel may change what
    // "the end of the timeline" is.
    clock: GameClock,              // sandboxed source of action timestamps
    accepted: Vec<VersionedInput>, // the single durable source of truth; replayed on boot
    // the engine's input version, queried from the runtime once per boot and stamped on every
    // newly-accepted input so a rebuild replays each under the semantics it was recorded with.
    engine_version: Version,
    // a fresh game's raw (unversioned) initial inputs -- InitializeEngine + the creation pack --
    // held only until the first boot learns the engine version and stamps them into `accepted`.
    // None once stamped, and always None for a resumed game (which loads already-versioned inputs).
    fresh_inputs: Option<Vec<ServerInput>>,
    history: History, // the in-memory command log every connection walks (a cache)
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
        server_state: WrappedServerState,
        start: GameStart,
        events: mpsc::UnboundedReceiver<GameInput>,
        inbox: mpsc::UnboundedSender<GameInput>,
        cancel: CancellationToken,
    ) -> Self {
        let store = lock_state(&server_state).store.clone();
        let mut tick = interval(Duration::from_secs(NULL_TICK_INTERVAL));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // a fresh game's first accepted input is always InitializeEngine, built here with a seed
        // the task generates itself -- the handler never sees the seed. the rest of the accepted
        // stream is the creation pack the handler supplied (the admin-key minting, etc). the engine
        // version is not known until boot queries the runtime, so a fresh stream is kept raw here
        // and stamped into `accepted` by boot() once the version is learned.
        let (game_id, registered, accepted, fresh_inputs, clock, keys_cache, creation_reply) =
            match start {
                GameStart::Fresh {
                    creation_pack,
                    creation_reply,
                } => {
                    let init = ActionRequest {
                        actor: ActionActor::System,
                        timestamp: 0,
                        payload: Action::InitializeEngine(
                            lawliet_types::action::InitializeEngine {
                                seed: crate::generate_seed(),
                            },
                        ),
                    };
                    let mut fresh = vec![ServerInput::Action(init)];
                    fresh.extend(creation_pack);
                    (
                        0,
                        false,
                        Vec::new(),
                        Some(fresh),
                        GameClock::new(),
                        HashMap::new(),
                        Some(creation_reply),
                    )
                }
                GameStart::Resumed {
                    game_id,
                    inputs,
                    keys,
                    start_clock,
                } => (
                    game_id,
                    true,
                    inputs,
                    None,
                    GameClock::at(start_clock),
                    keys,
                    None,
                ),
            };

        Self {
            game_id,
            server_state,
            store,
            inbox,
            cancel,
            events,
            registered,
            creation_reply,
            creation_responses: Vec::new(),
            last_reached: 0,
            clock,
            accepted,
            engine_version: 0,
            fresh_inputs,
            history: History::new(game_id),
            keys_cache,
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
        version: Version,
        caller: Option<&Key>,
    ) -> Result<RuntimeOutput, DispatchError> {
        let frame = PipeFrame {
            input: input.clone(),
            version,
            caller: caller.cloned(),
        };
        let line = to_line(&frame);
        if let Err(e) = self.write_line(&line).await {
            // a failed exchange abandons the pipe protocol mid-flight. the child must not survive
            // it: a child left running would eventually write the reply it owes, and a LATER
            // dispatch would read that stale line as its own -- every reply after that misaligned
            // by one. kill first, so a hang behaves exactly like a crash from here on.
            self.kill_runtime();
            return Err(e);
        }

        let text = match self.read_line().await {
            Ok(text) => text,
            Err(e) => {
                self.kill_runtime();
                return Err(e);
            }
        };
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

    // ask the runtime for the engine's input version. a server-issued GetVersion sim query that
    // changes nothing, so it is never persisted. None means the exchange failed -- which has
    // already killed the child -- so the caller must treat it as a failed attempt rather than
    // dispatch again. a reply of an unexpected shape still yields 0 (a degenerate fallback; boot
    // retries would have surfaced a real pipe failure).
    async fn query_engine_version(&mut self) -> Option<Version> {
        let control = SimControl {
            time: self.clock.now(),
            data: SimControlData::GetVersion,
        };
        let runtime_input = RuntimeInput::Sim(control);
        let Ok(output) = self.dispatch(&runtime_input, 0, None).await else {
            return None;
        };
        let Some(reply) = output.reply else {
            return Some(0);
        };
        match reply.outcome {
            ExecOutcome::Control(ControlOutcome::Ok(ControlResponse::EngineVersion(v))) => Some(v),
            _ => Some(0),
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

    // kill the runtime child and drop its pipes. dropping the Child triggers kill_on_drop (the
    // child is signaled and reaped) and dropping stdin closes the pipe, so a child abandoned
    // mid-exchange -- hung on a giant catch-up, or already dead -- can never have its late reply
    // misread as the answer to a later input. after this, the caller must reboot before the next
    // dispatch: stdin/stdout are gone, and dispatch expects them present.
    fn kill_runtime(&mut self) {
        self.child = None;
        self.stdin = None;
        self.stdout = None;
    }

    // boot: spawn a fresh runtime and bring it up to the state the old one had reached by re-feeding
    // the accepted stream. there is no rehydrate/rebuild split -- a boot always replays EVERY accepted
    // input (engine + sim) so the runtime reconstructs engine state, sim state, and the output
    // stream identically. history is a cache, so it is discarded and rebuilt fresh from the outputs
    // the runtime emits during replay.
    //
    // `truncate_to` is set by a backward time-travel jump: the accepted stream is first truncated to
    // everything up to the target, so the replay reconstructs only the timeline that still exists.
    //
    // Retries: on a failed spawn/replay the child is discarded and boot retries with exponential
    // backoff, giving up entirely after BOOT_MAX_RETRIES -- a game that cannot boot is reported as
    // failed rather than retrying forever. For a FRESH (not-yet-registered) game this is what stops
    // a broken game from ever being written to the DB.
    async fn boot(&mut self, truncate_to: Option<Time>) -> Result<(), ()> {
        if let Some(target) = truncate_to {
            self.truncate_accepted(target).await;
        }

        let mut attempts: u32 = 0;
        loop {
            let (stdin, stdout, child) = self.spawn_runtime();
            self.child = Some(child);
            self.stdin = Some(stdin);
            self.stdout = Some(BufReader::new(stdout).lines());

            // history is a cache: discard it and rebuild from the replay's outputs.
            self.history = History::new(self.game_id);
            // a fresh boot re-collects the creation pack's responses from scratch.
            self.creation_responses.clear();

            // learn the engine's input version for this boot, then stamp any still-unstamped fresh
            // inputs with it so the replay (and later persistence) carry it. boot runs first, so
            // the version is always known before the stream is executed. a failed exchange has
            // already killed the child (see dispatch), so it fails the attempt outright -- the
            // replay below must never dispatch into a dead pipe.
            let mut ok = true;
            match self.query_engine_version().await {
                Some(version) => self.engine_version = version,
                None => ok = false,
            }

            if ok
                && let Some(fresh) = self.fresh_inputs.take()
            {
                self.accepted = fresh
                    .into_iter()
                    .map(|input| VersionedInput {
                        version: self.engine_version,
                        input,
                    })
                    .collect();
            }

            let mut last_time: Time = 0;

            // re-feed the accepted stream. every input is server-issued (caller None): the stream
            // was already authorized when first accepted, so replay trusts it. the runtime
            // reconstructs engine + sim state and emits the output stream, which we append to
            // history. each input carries the version it was recorded under, so a rebuild replays
            // it under its own semantics. accepted is taken out for the replay (dispatch borrows
            // self mutably) and put back afterwards; it is still needed for later appends.
            if ok {
                let accepted = std::mem::take(&mut self.accepted);
                for versioned in &accepted {
                    let Some(runtime_input) = to_runtime_input(&versioned.input) else {
                        // meta controls are never stored in accepted.
                        continue;
                    };
                    match self.dispatch(&runtime_input, versioned.version, None).await {
                        Ok(output) => {
                            // the first boot of a fresh game also collects the responses to its
                            // creation pack (e.g. the minted admin key) to hand back to create_game.
                            if !self.registered
                                && let Some(reply) = &output.reply
                            {
                                self.creation_responses.push(reply.outcome.clone());
                            }
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
                self.accepted = accepted;
            }

            if ok {
                self.last_reached = last_time;
                // reconcile live key handles + privilege cache against the rebuilt sim key set.
                self.reconcile_key_handles();
                // anchor every client's game time on the rebuilt timeline.
                let at = self.append_game_clock();
                self.history.broadcast(&self.server_state, at, None);
                // once durable, keep the games row's progress current with the replay.
                if self.registered {
                    self.persist_progress().await;
                }
                return Ok(());
            }

            // the fresh child failed right out of the gate. discard it, back off, give up at a bound.
            self.child = None;
            self.stdin = None;
            self.stdout = None;
            attempts += 1;
            if attempts >= BOOT_MAX_RETRIES {
                return Err(());
            }
            // exponential: 500ms, 1s, 2s, ... (capped).
            let delay = BOOT_RETRY_BASE_MS.saturating_mul(1u64 << attempts.min(30));
            sleep(Duration::from_millis(delay)).await;
        }
    }

    // ===== INPUT HANDLING ===== //

    async fn handle_input(&mut self, input: GameInput) {
        match input {
            GameInput::GameCommand(GameCommand::Sync { ticket }) => {
                self.handle_sync(ticket);
            }
            GameInput::ServerInput(envelope) => self.handle_server_input(envelope).await,
        }
    }

    // a command for the game itself, handled immediately. no runtime involved.
    fn handle_sync(&mut self, ticket: Ticket) {
        self.history
            .deliver_sync(&self.server_state, ticket, self.clock.now());
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
                self.history.broadcast(
                    &self.server_state,
                    self.history.head(),
                    Some((ticket, pair)),
                );
            }
            ServerInput::Control(AdminControl::Sim(mut control)) => {
                // override the sim time at read time, as we do for engine actions.
                control.time = self.clock.now();

                let runtime_input = RuntimeInput::Sim(control.clone());
                match self
                    .dispatch(&runtime_input, self.engine_version, caller_key.as_ref())
                    .await
                {
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
                        // be replayed on the next boot. write-ahead before acknowledging; the game
                        // tears itself down if the write fails.
                        let sim_input = ServerInput::Control(AdminControl::Sim(control));
                        if self.persist_accepted(&sim_input).await.is_err() {
                            return;
                        }

                        let pair = output.reply.expect("runtime always replies to a sim input");
                        self.history.broadcast(
                            &self.server_state,
                            at,
                            Some((ticket, pair.into_pair())),
                        );
                    }
                    Err(_) => {
                        // the runtime crashed mid-control: record the crash, reboot, settle the caller.
                        let sim_input = ServerInput::Control(AdminControl::Sim(control));
                        self.record_crash(&sim_input).await;
                        self.reboot_after_crash().await;
                        let pair = ResponsePair {
                            response: ExecOutcome::Control(ControlOutcome::Denied),
                            input: sim_input,
                        };
                        self.history.broadcast(
                            &self.server_state,
                            self.history.head(),
                            Some((ticket, pair)),
                        );
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
                    let at = self.history.head();
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
                match self
                    .dispatch(&runtime_input, self.engine_version, caller_key.as_ref())
                    .await
                {
                    Ok(output) => {
                        self.update_server_projections(&output.outputs);
                        // a name left the server's secrecy in this action's outputs; rotate the
                        // simulation RNG before anything else draws, so the leaked name cannot
                        // predict the next epoch's.
                        let leak = emits_leak(&output.outputs);
                        let at = self.history.append(output.outputs);
                        // record the accepted action for the admin timeline.
                        let outcome = match &output.reply {
                            Some(r) => match &r.outcome {
                                ExecOutcome::Action(o) => o.clone(),
                                _ => ActionOutcome::EnginePanic,
                            },
                            None => ActionOutcome::EnginePanic,
                        };

                        // this input ran and was accepted, so it is part of the engine's state and
                        // must be replayed on the next boot. write-ahead before acknowledging; the
                        // game tears itself down if the write fails.
                        if self.persist_accepted(&reply_input).await.is_err() {
                            return;
                        }

                        let pair = ResponsePair {
                            response: ExecOutcome::Action(outcome),
                            input: reply_input,
                        };
                        self.history
                            .broadcast(&self.server_state, at, Some((ticket, pair)));
                        if leak {
                            self.inject_reseed().await;
                        }
                    }
                    Err(_) => {
                        // the runtime crashed mid-action: record the crash, reboot, then tell whoever
                        // sent it the engine panicked.
                        self.record_crash(&ServerInput::Action(request.clone()))
                            .await;
                        self.reboot_after_crash().await;
                        let at = self.history.head();
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
        self.keys_cache.get(key).is_some_and(|p| p.administers())
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

    // ===== SEED ROTATION ===== //

    // rotate the simulation RNG after a name leaked (see emits_leak): a fresh seed for the next
    // epoch so a name learned now cannot predict later ones. server-issued -- it is part of the
    // accepted stream and replays identically, and its reply reaches nobody.
    async fn inject_reseed(&mut self) {
        // the sim-time override applies to server-issued reseeds exactly as to client controls.
        let control = SimControl {
            time: self.clock.now(),
            data: SimControlData::ReSeed {
                seed: crate::generate_seed() as u64,
            },
        };
        let runtime_input = RuntimeInput::Sim(control.clone());
        if self
            .dispatch(&runtime_input, self.engine_version, None)
            .await
            .is_err()
        {
            // the runtime crashed mid-reseed: reboot, which replays it from the stream below.
            self.reboot_after_crash().await;
            return;
        }
        // write-ahead: the reseed is part of the sim's state and must be replayed on reboot.
        let sim_input = ServerInput::Control(AdminControl::Sim(control));
        let _ = self.persist_accepted(&sim_input).await;
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
            let request = ActionRequest {
                actor: ActionActor::System,
                timestamp: self.clock.now(),
                payload: Action::Null(Null {}),
            };
            let runtime_input = RuntimeInput::Action(request);
            match self
                .dispatch(&runtime_input, self.engine_version, None)
                .await
            {
                Ok(output) => {
                    self.last_reached = target;
                    self.update_server_projections(&output.outputs);
                    let at = self.history.append(output.outputs);
                    self.history.broadcast(&self.server_state, at, None);
                }
                Err(_) => {
                    // the engine hung (or died) mid-catch-up: a crash like any other. dispatch has
                    // already killed the child; the reboot replays the accepted stream, which does
                    // not contain this null (it was never accepted), so the engine comes back at
                    // the pre-jump state. the clock must be rolled back to match it -- a clock
                    // wound past a point the engine cannot reach within ENGINE_TIMEOUT would hang
                    // every subsequent tick the same way, forever.
                    self.clock.go_to(now);
                    self.reboot_after_crash().await;
                }
            }
            // re-anchor every client's game time on the current baseline (the jump target, or the
            // restored pre-jump time if the catch-up crashed).
            let at = self.append_game_clock();
            self.history.broadcast(&self.server_state, at, None);
            return;
        }

        // backward jump: truncate the accepted stream to the target and rebuild from there.
        self.last_reached = target;
        if self.boot(Some(target)).await.is_err() {
            // the rebuild never came up; the game cannot serve this timeline.
            self.cancel.cancel();
            return;
        }
        // every connection's view of the world was built on a timeline that no longer exists: reset
        // each one and replay the rebuilt history from the start.
        self.history
            .resync_all(&self.server_state, self.clock.now());
    }

    // ===== REWIND HELPERS ===== //

    // truncate the accepted stream to everything up to `target` (inclusive). sim controls carry
    // their own time; engine actions carry their timestamp. meta controls are never stored. the
    // durable half -- rows at seq >= the new length -- is deleted from the DB too; crash records
    // are untouched (separate table).
    async fn truncate_accepted(&mut self, target: Time) {
        let keep = |versioned: &VersionedInput| match &versioned.input {
            ServerInput::Action(request) => request.timestamp <= target,
            ServerInput::Control(AdminControl::Sim(control)) => control.time <= target,
            ServerInput::Control(AdminControl::Meta(_)) => false,
        };
        let retained_len = self.accepted.iter().filter(|i| keep(i)).count();
        if self.registered
            && let Err(e) = self
                .store
                .delete_inputs_from(self.game_id, retained_len as i64)
                .await
        {
            eprintln!(
                "failed to truncate inputs for game {} -- tearing down: {e}",
                self.game_id
            );
            self.cancel.cancel();
        }
        self.accepted.retain(keep);
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

    // // append the admin-visible record of one action request and its outcome to history, and return
    // // where it landed.
    // fn append_log_action(&mut self, request: &ActionRequest, outcome: ActionOutcome) -> usize {
    //     self.history
    //         .append(vec![log_action_output(request, outcome, request.timestamp)])
    // }
    //
    // // append a log action right after the engine commands of the action it describes (at `at`), so
    // // the record rides the same batch as the commands.
    // fn append_log_action_at(
    //     &mut self,
    //     request: &ActionRequest,
    //     outcome: ActionOutcome,
    //     _at: usize,
    // ) {
    //     self.append_log_action(request, outcome);
    // }

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
        match self
            .dispatch(&runtime_input, self.engine_version, None)
            .await
        {
            Ok(output) => {
                self.update_server_projections(&output.outputs);
                let at = self.history.append(output.outputs);
                self.history.broadcast(&self.server_state, at, None);
            }
            Err(_) => {
                // a hung tick is a crash like any other: dispatch has killed the child, so reboot
                // from the accepted stream. the null is server-issued and never persisted, so
                // there is nothing to roll back and nothing worth recording (the replayed stream
                // does not even contain the input that hung) -- the same policy as a failed
                // reseed.
                self.reboot_after_crash().await;
            }
        }
    }

    // ===== DURABILITY / REGISTRATION ===== //

    // write the current progress (last_reached, clock, keys) into the games row. called after a
    // boot of a durable game so the metadata the row caches (for on-demand boot) stays current.
    async fn persist_progress(&self) {
        let meta = GameMeta {
            last_reached: self.last_reached,
            clock: self.clock.now(),
            clock_wall: wall_now(),
            keys: self.keys_cache.clone(),
        };
        if let Err(e) = self.store.persist_progress(self.game_id, &meta).await {
            eprintln!(
                "failed to persist progress for game {} -- tearing down: {e}",
                self.game_id
            );
            self.cancel.cancel();
        }
    }

    // register this game's GameHandle in the shared registry. a fresh game calls this after its
    // first boot succeeds and its row is written; a resumed game is registered by main() before
    // its task is spawned.
    fn insert_handle(&self, keys: HashMap<Key, Privileges>) {
        insert_handle(
            &self.server_state,
            self.game_id,
            self.inbox.clone(),
            self.cancel.clone(),
            keys,
        );
    }

    // make a fresh game durable and report success: write the row + the whole accepted stream (the
    // task-generated InitializeEngine + the creation pack) as a group, then hand back the game id
    // and the creation pack's responses.
    async fn register_fresh(&mut self) {
        let id = match self.store.create_game(&self.accepted).await {
            Ok(id) => id,
            Err(e) => {
                // the game cannot become durable, so it cannot exist. close it and tell the
                // create_game caller it failed to boot, rather than leaving the endpoint hanging.
                eprintln!("failed to write fresh game to the db -- tearing down: {e}");
                self.cancel.cancel();
                if let Some(reply) = self.creation_reply.take() {
                    let _ = reply.send(Err(InitError::BootFailed));
                }
                return;
            }
        };
        self.game_id = id;
        self.registered = true;
        self.history.game_id = id;
        self.insert_handle(self.keys_cache.clone());
        // mint the live key handles (cancel token + tickets) for the rebuilt key set -- get_ticket
        // needs a handle for the admin key before any connection can form.
        self.reconcile_key_handles();
        if let Some(reply) = self.creation_reply.take() {
            let responses = std::mem::take(&mut self.creation_responses);
            let _ = reply.send(Ok((id, responses)));
        }
    }

    // WRITE-AHEAD append: persist one accepted input (and the game's latest progress) before the
    // caller acknowledges it. only pushed to the in-memory `accepted` after the write succeeds, so
    // RAM and DB never diverge. on ANY write failure the game tears itself down -- it cannot stay
    // durable, so it must not keep serving (callers get an Err and return; the cancel removes the
    // game, closes its connections, and drops the runtime).
    async fn persist_accepted(&mut self, input: &ServerInput) -> Result<(), sqlx::Error> {
        let seq = self.accepted.len() as i64;
        let meta = GameMeta {
            last_reached: self.last_reached,
            clock: self.clock.now(),
            clock_wall: wall_now(),
            keys: self.keys_cache.clone(),
        };
        let versioned = VersionedInput {
            version: self.engine_version,
            input: input.clone(),
        };
        match self
            .store
            .append_input(self.game_id, seq, &versioned, &meta)
            .await
        {
            Ok(()) => {
                self.accepted.push(versioned);
                Ok(())
            }
            Err(e) => {
                eprintln!(
                    "persist failed for game {} -- tearing down: {e}",
                    self.game_id
                );
                self.cancel.cancel();
                Err(e)
            }
        }
    }

    // record a crash (the accepted stream up to and including the crashing input) for later
    // debugging. inert -- never replayed, and survives rewind.
    async fn record_crash(&self, crashing: &ServerInput) {
        let mut sequence = self.accepted.clone();
        sequence.push(VersionedInput {
            version: self.engine_version,
            input: crashing.clone(),
        });
        if let Err(e) = self.store.record_crash(self.game_id, &sequence).await {
            eprintln!("failed to record crash for game {}: {e}", self.game_id);
        }
    }

    // reboot after an engine crash or hang; the old child is already dead (dispatch kills it), so
    // this is purely a fresh spawn + replay. if the fresh child cannot be brought up within the
    // retry bound, the game tears itself down (it cannot serve).
    async fn reboot_after_crash(&mut self) {
        if self.boot(None).await.is_err() {
            eprintln!(
                "game {} failed to reboot after crash -- tearing down",
                self.game_id
            );
            self.cancel.cancel();
        }
    }

    // ===== LOOP ===== //

    async fn run(mut self) {
        // FRESH: the first boot both proves the engine can run and collects the creation pack's
        // responses. it is only written to the DB if it succeeds -- a game that fails to boot is
        // reported as failed and never written. RESUMED: the game is already durable.
        match self.boot(None).await {
            Ok(()) => {
                if self.creation_reply.is_some() {
                    self.register_fresh().await;
                }
            }
            Err(()) => {
                if let Some(reply) = self.creation_reply.take() {
                    let _ = reply.send(Err(InitError::BootFailed));
                } else {
                    eprintln!(
                        "game {} failed to boot after max retries -- giving up",
                        self.game_id
                    );
                }
                return;
            }
        }

        loop {
            // once the cancellation token has fired the game is closing. gate the input and tick
            // arms on it not being cancelled, so a waiting input can never be selected (and
            // processed) after the game should have stopped -- select! would otherwise pick
            // nondeterministically among ready arms. with the token cancelled, only the cancel
            // arm remains selectable and the loop breaks.
            select! {
                Some(input) = self.events.recv(), if !self.cancel.is_cancelled() => {
                    self.handle_input(input).await;
                }
                _ = self.tick.tick(), if !self.cancel.is_cancelled() => {
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
    start: GameStart,
    events: mpsc::UnboundedReceiver<GameInput>,
    inbox: mpsc::UnboundedSender<GameInput>,
    cancel: CancellationToken,
) {
    Game::new(state, start, events, inbox, cancel).run().await;
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

// does an action's output put a name into someone's hands? a true name is revealed or sent
// (RevealTrueName, TrueNameUpdate) or a display roster is broadcast (ProfileRoster). any of these
// is a "leak": the receiver could use it as an oracle against the deterministic RNG, so the
// simulation RNG must rotate before the next name is drawn.
fn emits_leak(outputs: &[Output]) -> bool {
    outputs.iter().any(|output| {
        matches!(
            &output.data,
            OutputData::Engine(Command::RevealTrueName { .. })
                | OutputData::Engine(Command::TrueNameUpdate { .. })
                | OutputData::Sim(SimOutput::ProfileRoster { .. })
        )
    })
}
