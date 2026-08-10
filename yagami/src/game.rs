// One game, one task.
//
// A select arm, once chosen, runs to completion and no other arm may interleave. That single fact
// collapses the old supervisor/watchdog/coordinator pile into one task and a handful of linear
// async helpers: spawn+boot, dispatch (write then read), and a select loop that feeds inputs and
// ticks through them one at a time. There is no supervisor task, and no boot handler arm.

use std::{collections::HashSet, env::current_exe, io::ErrorKind, process::Stdio, time::Duration};

use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, InitializeEngine, Null},
    command::{Command, CommandPayload},
    common::{Seed, Time},
    engine::ExecutionResult,
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
    auth::{Capability, Key, KeyData, Privileges, Ticket, to_flags},
    constants::{ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    delivery::History,
    generate_seed,
    state::{GameHandle, GameId, WrappedServerState, lock_state},
    wire::{
        ActionOutcome, AdminControl, ControlError, ControlOutcome, ControlResponse, ExecOutcome,
        ResponsePair, ServerInput,
    },
};

// TODO:
// - server data deliveries. people need to see keys, profiles, and other similar information.
// simply construct a server command on change, append it to history, and deliver it as you would
// anything else.
// - client migration to the new server model.

pub fn to_line<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json + "\n",
        Err(e) => {
            eprintln!("failed to serialize for the engine pipe: {e} -- aborting");
            std::process::abort()
        }
    }
}

pub enum GameError {
    PlayerDoesntExist,
}

// server extension of the engine's error set
pub enum InputError {
    EngineErr(lawliet_types::action::ActionError),
    GameErr(GameError),
}

// carries the source ticket so the game task can route replies and enforce permissions.
pub struct InputEnvelope {
    pub ticket: Ticket,
    pub input: ServerInput,
}

// "do this thing" rather than "i want to do this thing".
// external server inputs. some other part of the server wants the game to do something.
pub enum GameCommand {
    // synchronize the connection to the game state given its current permissions
    Sync { ticket: Ticket },
}

#[allow(clippy::large_enum_variant)]
pub enum GameInput {
    GameCommand(GameCommand),
    ServerInput(InputEnvelope),
}

// the only shape an engine interaction can end in. a successful dispatch returns the deserialized
// execution result; anything else is a write or read failure.
enum DispatchError {
    Write,
    Read,
}

// an error that leaves the child in a usable state, so retrying is safe rather than a rebuild.
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
        let offset = target - self.now();
        self.anchor += offset as i128;
    }
}

struct Game {
    // identity / routing
    game_id: GameId,
    server_state: WrappedServerState,
    cancel: CancellationToken,
    events: mpsc::UnboundedReceiver<GameInput>,

    // state
    seed: Seed,
    last_reached: Time, // the latest timestamp executed by the engine, not necessarily
    // the highest ever executed. this distinction is important because time travel may change what
    // "the end of the timeline" is.
    clock: GameClock,             // sandboxed source of action timestamps
    accepted: Vec<ActionRequest>, // accepted engine inputs, replayed on boot
    history: History,             // the command log every connection walks from

    // process comms. the child is held here so the engine stays alive for as long as this task does:
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
    ) -> Self {
        let mut tick = interval(Duration::from_secs(NULL_TICK_INTERVAL));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            game_id,
            server_state,
            cancel,
            events,
            seed: generate_seed(),
            last_reached: 0,
            clock: GameClock::new(),
            accepted: Vec::new(),
            history: History::new(game_id),
            child: None,
            stdin: None,
            stdout: None,
            tick,
        }
    }

    // ===== LOW-LEVEL COMMS ===== //

    // write one line to the engine under a timeout, retrying errors that a retry fixes (EINTR).
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

    // read one line from the engine under a timeout. returns Read on EOF or a fatal read failure.
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

    // write a request, then read and deserialize the response. the whole exchange is linear and
    // completes or fails as one unit.
    async fn dispatch(
        &mut self,
        request: &ActionRequest,
    ) -> Result<ExecutionResult, DispatchError> {
        let line = to_line(request);
        self.write_line(&line).await?;

        let text = self.read_line().await?;
        match serde_json::from_str(&text) {
            Ok(result) => Ok(result),
            // an undeserializable line means the two binaries disagree about the wire format. a
            // deploy mistake, not a runtime condition.
            Err(e) => {
                eprintln!("engine output failed to deserialize: {e} -- aborting");
                std::process::abort()
            }
        }
    }

    // dispatch a request and fold in the bookkeeping common to every successful exchange: on
    // failure, reboot; on success, fold the context into the shared log and fan the batch out to
    // every connection, with the outcome riding to the one that asked.
    //
    // `reply` echoes whatever was sent (see ResponsePair); `None` means a server-issued action with
    // no originating connection -- a tick or a jump -- whose commands still reach everyone.
    // also logs the current last executed time on success.
    async fn execute(
        &mut self,
        request: &ActionRequest,
        reply: Option<(Ticket, ServerInput)>,
    ) -> bool {
        match self.dispatch(request).await {
            Ok(result) => {
                self.last_reached = request.timestamp;
                let (outcome, commands) = match result {
                    Ok((response, context)) => (ActionOutcome::Ok(response), context.commands),
                    Err((error, context)) => (ActionOutcome::Err(error), context.commands),
                };
                // an error still carries the world progression that ran before the action failed
                // (see the ExecutionResult alias), so actor mappings in it are still real.
                self.record_actor_creations(&commands);

                let at = self.history.append_engine(commands);
                let reply = reply.map(|(ticket, input)| {
                    (
                        ticket,
                        ResponsePair {
                            response: ExecOutcome::Action(outcome),
                            input,
                        },
                    )
                });
                self.history.broadcast(&self.server_state, at, reply);
                true
            }
            Err(_) => {
                self.boot().await;
                // the action died in flight: tell whoever sent it.
                if let Some((ticket, input)) = reply {
                    let pair = ResponsePair {
                        response: ExecOutcome::Action(ActionOutcome::EnginePanic),
                        input,
                    };
                    self.history.broadcast(
                        &self.server_state,
                        self.history.head(),
                        Some((ticket, pair)),
                    );
                }
                false
            }
        }
    }

    // hand one connection a reply with no new commands -- a deny, or a crash already handled above.
    // rides an empty broadcast so the batch reaches that one socket and no other.
    fn deliver_reply(&self, ticket: Ticket, input: ServerInput, outcome: ExecOutcome) {
        let pair = ResponsePair {
            response: outcome,
            input,
        };
        self.history.broadcast(
            &self.server_state,
            self.history.head(),
            Some((ticket, pair)),
        );
    }

    // scan one execution's output for actor mappings and record when each actor slot came into
    // being. this is what a rewind uses to know whether a profile or key stands on an actor that
    // did not exist yet at the target time.
    fn record_actor_creations(&self, commands: &[CommandPayload]) {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return;
        };
        record_actor_creations(game, commands);
    }

    // ===== PROCESS MANAGEMENT ===== //

    fn spawn_engine(&mut self) -> (ChildStdin, ChildStdout, tokio::process::Child) {
        let mut child = tokio::process::Command::new(
            current_exe()
                .expect("failed to get current exe")
                .parent()
                .expect("failed to get parent path")
                .join(format!("lawliet-runtime{}", std::env::consts::EXE_SUFFIX)),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("failed to boot lawliet runtime");

        let stdin = child.stdin.take().expect("child stdin piped");
        let stdout = child.stdout.take().expect("child stdout piped");
        (stdin, stdout, child)
    }

    // spawn a fresh engine and bring it up to the state the old one had reached. rehydrates via
    // dispatch, discarding every success response and retrying on failure.
    async fn boot(&mut self) {
        // TODO:
        // exponential backoff between retries, and eventual total game failure once retries exceed
        // some bound, rather than retrying forever.
        loop {
            let (stdin, stdout, child) = self.spawn_engine();
            self.child = Some(child);
            self.stdin = Some(stdin);
            self.stdout = Some(BufReader::new(stdout).lines());

            let mut ok = true;

            // replay every accepted input. success responses are discarded.
            let accepted = std::mem::take(&mut self.accepted);
            for request in &accepted {
                if self.dispatch(request).await.is_err() {
                    ok = false;
                    break;
                }
            }
            self.accepted = accepted;

            // jump the engine up to the clock the previous child had reached, so jobs already
            // executed are not re-run. success responses are discarded.
            if ok && self.last_reached > 0 {
                let request = ActionRequest {
                    actor: ActionActor::System,
                    timestamp: self.last_reached,
                    payload: Action::Null(Null {}),
                };
                if self.dispatch(&request).await.is_err() {
                    ok = false;
                }
            }

            // first boot only: hand the engine its seed. once it succeeds it lives in `accepted`
            // like any other input, so a later rebuild replays it with the same seed.
            if ok && self.accepted.is_empty() {
                let request = ActionRequest {
                    actor: ActionActor::System,
                    timestamp: self.clock.now(),
                    payload: Action::InitializeEngine(InitializeEngine { seed: self.seed }),
                };
                match self.dispatch(&request).await {
                    Ok(_) => self.accepted.push(request),
                    Err(_) => ok = false,
                }
            }

            if ok {
                return;
            }

            // the fresh child failed right out of the gate. discard it and try again.
            self.child = None;
            self.stdin = None;
            self.stdout = None;
        }
    }

    // ===== INITIALIZATION AND TEARDOWN ===== //

    // gather whatever the game needs before the engine boots. a stub for now.
    async fn initialize(&mut self) {
        // TODO:
        // read the game's persistent state (config, players, keys) from the database.
    }

    // cleanup after finishing
    async fn teardown(&mut self) {
        let mut state = lock_state(&self.server_state);
        self.cancel.cancel();
        state.games.remove(&self.game_id);
    }

    // ===== INPUT HANDLING ===== //

    async fn handle_input(&mut self, input: GameInput) {
        match input {
            GameInput::GameCommand(command) => self.handle_command(command),
            GameInput::ServerInput(envelope) => self.handle_server_input(envelope).await,
        }
    }

    // a command for the game itself, handled immediately. no engine involved.
    fn handle_command(&mut self, command: GameCommand) {
        match command {
            GameCommand::Sync { ticket } => {
                // replay the whole entitled history to this connection and install its cursor:
                // from now on `broadcast` advances it live. starts a sync before the socket reads a
                // single frame, so the connection is caught up before anything it sends is executed.
                self.history.deliver_sync(&self.server_state, ticket);
            }
        }
    }

    // the connection behind `ticket` may act as `request.actor` under its current privilege set.
    // this is the whole of a connection's authority: a key may act as any actor its scope names
    // (plus Admin as Administer) and nothing else. checked fresh each time, so a narrowed key takes
    // effect on its live sockets at once.
    fn authorize_action(&self, ticket: &Ticket, request: &ActionRequest) -> bool {
        lock_state(&self.server_state)
            .games
            .get(&self.game_id)
            .and_then(|game| game.privileges(ticket))
            .is_some_and(|privileges| privileges.can_act_as(&request.actor))
    }

    // controls operate ON the game, not in the fiction, so the only authority they need is that the
    // connection holds Administer.
    fn authorize_control(&self, ticket: &Ticket) -> bool {
        lock_state(&self.server_state)
            .games
            .get(&self.game_id)
            .and_then(|game| game.privileges(ticket))
            .is_some_and(|privileges| privileges.administers())
    }

    async fn handle_server_input(&mut self, envelope: InputEnvelope) {
        let InputEnvelope { ticket, input } = envelope;

        match input {
            ServerInput::Control(control) => {
                if !self.authorize_control(&ticket) {
                    self.deliver_reply(
                        ticket,
                        ServerInput::Control(control),
                        ExecOutcome::Control(ControlOutcome::Denied),
                    );
                    return;
                }
                match control {
                    AdminControl::GoToTime { time } => self.go_to_time(time).await,
                    _ => {
                        let outcome = handle_control(
                            &self.server_state,
                            self.game_id,
                            &ticket,
                            self.clock.now(),
                            &control,
                        );
                        self.deliver_control_feedback(&ticket, &control, outcome);
                    }
                }
            }
            ServerInput::Action(mut request) => {
                // override the client's reported value. only the game task truly knows the game's
                // virtual clock, and a client cannot be trusted.
                request.timestamp = self.clock.now();

                if !self.authorize_action(&ticket, &request) {
                    self.deliver_reply(
                        ticket,
                        ServerInput::Action(request),
                        ExecOutcome::Action(ActionOutcome::Denied),
                    );
                    return;
                }
                // the reply echoes the request, so clone for the dispatch and keep the original to
                // record as accepted.
                let reply_input = ServerInput::Action(request.clone());
                if !self.execute(&request, Some((ticket, reply_input))).await {
                    // the crash is answered inside execute.
                    return;
                }
                // this input ran and was accepted, so it is part of the engine's state and
                // must be replayed on the next boot.
                self.accepted.push(request);
            }
        }
    }

    // reply to a control. the feedback is immediate -- it never touches the engine -- so it rides
    // out as its own payload rather than a response to a dispatch.
    fn deliver_control_feedback(
        &self,
        ticket: &Ticket,
        control: &AdminControl,
        outcome: ControlOutcome,
    ) {
        self.deliver_reply(
            ticket.clone(),
            ServerInput::Control((*control).clone()),
            ExecOutcome::Control(outcome),
        );
    }

    // GoToTime: move the game to a past (or future) instant.
    //
    // Going FORWARD costs nothing structural: the engine is already at the current time and will
    // reach `target` naturally on its next tick, so all we do is advance the sandbox clock to it.
    //
    // Going BACKWARD is a rebuild: we truncate the accepted log to everything up to the target, set
    // the clock there, and boot the engine fresh from that base -- so it replays only the state that
    // existed up to the target time.
    //
    // It should also be noted that back to some point in time does not go to BEFORE that time. Everything that
    // happened AT that time remains.
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
            // a forward jump has no originating connection, but its commands still reach everyone.
            self.execute(&request, None).await;
            return;
        }

        // anything accepted after the target never happened, so it is dropped from what a boot
        // replays. the engine no longer knows, or is responsible for, the invalidated future.
        self.accepted.retain(|request| request.timestamp <= target);
        self.last_reached = target;
        self.discard_after(target);
        self.boot().await;

        // every connection's view of the world was built on a timeline that no longer exists: reset
        // each one and replay the truncated history from the start.
        self.history.resync_all(&self.server_state);
    }

    // ===== REWIND ===== //

    // prune server-side state that no longer has a foundation at `target`, after a backward jump.
    //
    // an actor slot created after `target` did not exist yet, so anything standing on it -- a
    // profile for that actor -- is discarded whole. a key is kept unless it was minted after
    // `target`, but its scope is trimmed to drop any actor that did not exist yet: authority over a
    // player who has not been mapped is authority over nothing, and it cannot be exercised or
    // delivered, so it is cut rather than carried forward as a dangling reference.
    fn discard_after(&mut self, target: Time) {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return;
        };
        discard_after(game, target);
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
        // no originating connection; the tick's commands still reach everyone.
        self.execute(&request, None).await;
    }

    // ===== LOOP ===== //

    async fn run(mut self) {
        self.initialize().await;
        self.boot().await;

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

        // dropping `self` drops the child, and kill_on_drop reaps the engine.
        self.teardown().await;
    }
}

// permission enforcement, input executions, live client updates, and engine process management.
pub async fn game(
    state: WrappedServerState,
    game_id: GameId,
    events: mpsc::UnboundedReceiver<GameInput>,
    cancel: CancellationToken,
) {
    Game::new(game_id, state, cancel, events).run().await;
}

// ===== CONTROL / KEY MANAGEMENT ===== //

// may the caller act on this target key? the single authority rule for every key-management control,
// kept in one place so a control added later cannot quietly skip it.
//
// the two rules below combine into the property that keeps a game administrable: a key holding
// Administer is reachable ONLY from a Supervise holder, and a Supervise holder cannot reach its own
// key -- so the LAST Supervise holder can be neither revoked nor demoted, by anyone. there is always
// at least one key holding Administer. changing either rule breaks that, so change them together.
fn may_manage(
    game: &GameHandle,
    caller_key: &Key,
    supervises: bool,
    target: &Key,
) -> Result<(), ControlError> {
    let Some(target_data) = game.keys.get(target) else {
        return Err(ControlError::KeyNotFound);
    };

    if target == caller_key {
        return if supervises {
            Err(ControlError::CannotActOnSelf)
        } else {
            Ok(())
        };
    }

    if target_data
        .privileges
        .capabilities
        .contains(Capability::Administer)
        && !supervises
    {
        return Err(ControlError::RequiresSupervise);
    }

    Ok(())
}

// withdraw a key and everything standing on it.
fn revoke_key(game: &mut GameHandle, key: &Key) {
    let Some(key_data) = game.keys.remove(key) else {
        return;
    };

    key_data.cancel.cancel();

    for ticket in key_data.tickets {
        game.tickets.remove(&ticket);

        if let Some(conn) = game.connections.get_mut(&ticket) {
            conn.dropped = true;
        }
    }
}

#[allow(unused)]
fn apply_privilege_change(_game: &mut GameHandle, _key: &Key, _before: Privileges) {}

// carry out one control. authority over the target is checked per-control rather than up front,
// because CreateKey has no target. `now` is the control's timestamp, stamped onto any server-side
// state the control mints so a rewind can discard it.
fn manage(
    game: &mut GameHandle,
    caller_key: &Key,
    supervises: bool,
    now: Time,
    control: &AdminControl,
) -> Result<ControlResponse, ControlError> {
    match control {
        AdminControl::CreateKey {
            actors,
            capabilities,
        } => {
            let capabilities = to_flags(capabilities);
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            let key = Key::generate();
            game.keys.insert(
                key.clone(),
                KeyData {
                    cancel: game.cancel.child_token(),
                    tickets: HashSet::new(),
                    privileges: Privileges {
                        actors: actors.clone(),
                        capabilities,
                    },
                },
            );
            game.key_created.insert(key.clone(), now);

            Ok(ControlResponse::KeyCreated { key })
        }

        AdminControl::RevokeKey { key } => {
            may_manage(game, caller_key, supervises, key)?;
            revoke_key(game, key);
            Ok(ControlResponse::KeyRevoked)
        }

        AdminControl::SetCapabilities { key, capabilities } => {
            may_manage(game, caller_key, supervises, key)?;

            let capabilities = to_flags(capabilities);
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            let key_data = game
                .keys
                .get_mut(key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.capabilities = capabilities;

            apply_privilege_change(game, key, before);

            Ok(ControlResponse::CapabilitiesSet)
        }

        AdminControl::SetActorScope { key, actors } => {
            may_manage(game, caller_key, supervises, key)?;

            let key_data = game
                .keys
                .get_mut(key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.actors = actors.clone();

            apply_privilege_change(game, key, before);

            Ok(ControlResponse::ActorScopeSet)
        }

        AdminControl::SetProfile { actor, profile } => {
            game.profiles.insert(*actor, profile.clone());
            game.profile_created.insert(*actor, now);
            Ok(ControlResponse::ProfileSet)
        }

        // time travel is the game task's own mechanic, handled by go_to_time, not key management.
        AdminControl::GoToTime { .. } => unreachable!(),
    }
}

fn handle_control(
    state: &WrappedServerState,
    game_id: GameId,
    ticket: &Ticket,
    now: Time,
    control: &AdminControl,
) -> ControlOutcome {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return ControlOutcome::Denied; // game is gone
    };

    let Some(caller_key) = game.tickets.get(ticket).cloned() else {
        return ControlOutcome::Denied;
    };
    let Some(caller) = game.keys.get(&caller_key) else {
        return ControlOutcome::Denied;
    };

    let capabilities = caller.privileges.capabilities;

    if !capabilities.contains(Capability::Administer) {
        return ControlOutcome::Denied;
    }

    match manage(
        game,
        &caller_key,
        capabilities.contains(Capability::Supervise),
        now,
        control,
    ) {
        Ok(response) => ControlOutcome::Ok(response),
        Err(error) => ControlOutcome::Err(error),
    }
}

// see Game::record_actor_creations.
fn record_actor_creations(game: &mut GameHandle, commands: &[CommandPayload]) {
    for payload in commands {
        if let Command::MapActor { actor_id, .. } = &payload.cmd {
            game.actor_created
                .entry(*actor_id)
                .or_insert(payload.timestamp);
        }
    }
}

// see Game::discard_after.
fn discard_after(game: &mut GameHandle, target: Time) {
    // only slots that existed by `target` survive; a later one is a branch that was rolled back.
    game.actor_created.retain(|_, time| *time <= target);
    game.profile_created.retain(|_, time| *time <= target);
    game.key_created.retain(|_, time| *time <= target);

    // a profile whose actor was not mapped by `target` (or was minted itself after it) is
    // standing on nothing and goes.
    game.profiles.retain(|actor, _| {
        let actor_existed = game.actor_created.get(actor).is_some_and(|t| *t <= target);
        let minted_by = game
            .profile_created
            .get(actor)
            .is_some_and(|t| *t <= target);
        actor_existed && minted_by
    });

    // a key keeps its identity but loses scope over actors that did not exist yet.
    game.keys.retain(|key, key_data| {
        game.key_created.get(key).is_some_and(|t| *t <= target) && {
            if let crate::auth::ActorScope::Only(actors) = &mut key_data.privileges.actors {
                actors.retain(|actor| game.actor_created.get(actor).is_some_and(|t| *t <= target));
            }
            true
        }
    });
}

#[cfg(test)]
mod game_tests {
    use std::collections::{HashMap, HashSet};

    use enumflags2::BitFlags;
    use lawliet_types::command::{Command, CommandPayload, CommandRecipient};
    use lawliet_types::common::ActorKey;
    use slotmap::KeyData;

    use super::*;
    use crate::auth::{ActorScope, Privileges};
    use crate::wire::Profile;

    fn actor(n: u64) -> ActorKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn handle() -> GameHandle {
        let (inbox, _rx) = mpsc::unbounded_channel();
        GameHandle {
            cancel: CancellationToken::new(),
            inbox,
            tickets: HashMap::new(),
            connections: HashMap::new(),
            keys: HashMap::new(),
            profiles: HashMap::new(),
            actor_created: HashMap::new(),
            key_created: HashMap::new(),
            profile_created: HashMap::new(),
        }
    }

    fn map_actor(who: ActorKey, at: Time) -> CommandPayload {
        CommandPayload {
            timestamp: at,
            recipient: CommandRecipient::Viewport(KeyData::from_ffi(1 | (1 << 32)).into()),
            cmd: Command::MapActor {
                actor_id: who,
                kind: lawliet_types::actor::ActorKind::Player,
            },
        }
    }

    fn mint_key(game: &mut GameHandle, actors: &[ActorKey], at: Time) -> Key {
        let control = AdminControl::CreateKey {
            actors: ActorScope::Only(actors.iter().copied().collect()),
            capabilities: vec![],
        };
        let response = match manage(game, &Key::generate(), false, at, &control) {
            Ok(r) => r,
            Err(e) => panic!("create key failed"),
        };
        match response {
            ControlResponse::KeyCreated { key } => key,
            _ => panic!("expected a key"),
        }
    }

    fn set_profile(game: &mut GameHandle, who: ActorKey, at: Time) {
        let control = AdminControl::SetProfile {
            actor: who,
            profile: Profile::default(),
        };
        match manage(game, &Key::generate(), false, at, &control) {
            Ok(_) => {}
            Err(e) => panic!("set profile failed"),
        }
    }

    fn mint_all_scope_key(game: &mut GameHandle, at: Time) -> Key {
        let key = Key::generate();
        game.keys.insert(
            key.clone(),
            crate::auth::KeyData {
                cancel: CancellationToken::new(),
                tickets: HashSet::new(),
                privileges: Privileges {
                    actors: ActorScope::All,
                    capabilities: BitFlags::empty(),
                },
            },
        );
        game.key_created.insert(key.clone(), at);
        key
    }

    #[test]
    fn record_actor_creations_stamps_the_birth_time() {
        let mut game = handle();
        record_actor_creations(
            &mut game,
            &[map_actor(actor(1), 10), map_actor(actor(2), 20)],
        );
        assert_eq!(game.actor_created.get(&actor(1)), Some(&10));
        assert_eq!(game.actor_created.get(&actor(2)), Some(&20));
    }

    #[test]
    fn first_mapping_wins_for_an_actor() {
        let mut game = handle();
        record_actor_creations(
            &mut game,
            &[map_actor(actor(1), 10), map_actor(actor(1), 50)],
        );
        // a slot's birth time is when it first appeared; a later re-map cannot move it forward.
        assert_eq!(game.actor_created.get(&actor(1)), Some(&10));
    }

    #[test]
    fn profile_standing_on_a_rolled_back_actor_is_discarded() {
        let mut game = handle();
        // actor 1 is mapped at t=10, dies, and is mapped again at t=30? no -- treat it as a single
        // birth. actor 2 only appears after the rewind target.
        record_actor_creations(
            &mut game,
            &[map_actor(actor(1), 10), map_actor(actor(2), 30)],
        );
        set_profile(&mut game, actor(1), 15);
        set_profile(&mut game, actor(2), 35);

        discard_after(&mut game, 20);

        // profile for actor 1 (born before target, actor exists) survives.
        assert!(game.profiles.contains_key(&actor(1)));
        // profile for actor 2 (actor did not exist by target) is gone.
        assert!(!game.profiles.contains_key(&actor(2)));
    }

    #[test]
    fn profile_minted_after_target_is_discarded() {
        let mut game = handle();
        record_actor_creations(&mut game, &[map_actor(actor(1), 10)]);
        set_profile(&mut game, actor(1), 30);

        discard_after(&mut game, 20);

        assert!(!game.profiles.contains_key(&actor(1)));
    }

    #[test]
    fn key_minted_after_target_is_discarded() {
        let mut game = handle();
        record_actor_creations(&mut game, &[map_actor(actor(1), 10)]);
        let key = mint_key(&mut game, &[actor(1)], 30);

        discard_after(&mut game, 20);

        assert!(!game.keys.contains_key(&key));
    }

    #[test]
    fn key_survives_but_drops_dangling_actor_refs() {
        let mut game = handle();
        record_actor_creations(
            &mut game,
            &[map_actor(actor(1), 10), map_actor(actor(2), 30)],
        );
        let key = mint_key(&mut game, &[actor(1), actor(2)], 15);

        discard_after(&mut game, 20);

        let key_data = game
            .keys
            .get(&key)
            .expect("key minted before target survives");
        match &key_data.privileges.actors {
            ActorScope::Only(actors) => {
                assert!(actors.contains(&actor(1)));
                assert!(!actors.contains(&actor(2)));
            }
            _ => panic!("expected an Only scope"),
        }
    }

    #[test]
    fn key_minted_before_target_but_wholly_rolled_back_actors_keeps_empty_scope() {
        let mut game = handle();
        record_actor_creations(&mut game, &[map_actor(actor(2), 30)]);
        let key = mint_key(&mut game, &[actor(2)], 15);

        discard_after(&mut game, 20);

        let key_data = game.keys.get(&key).expect("key survives");
        match &key_data.privileges.actors {
            ActorScope::Only(actors) => assert!(actors.is_empty()),
            _ => panic!("expected an Only scope"),
        }
    }

    #[test]
    fn all_scope_key_minted_before_target_survives_intact() {
        let mut game = handle();
        let key = mint_all_scope_key(&mut game, 15);

        discard_after(&mut game, 20);

        let key_data = game
            .keys
            .get(&key)
            .expect("key minted before target survives");
        // All scope reads every actor unconditionally, so there is nothing to trim -- but the key
        // still owes its survival to its mint time, not to the scope.
        assert!(matches!(key_data.privileges.actors, ActorScope::All));
    }

    #[test]
    fn all_scope_key_minted_after_target_is_discarded() {
        let mut game = handle();
        let key = mint_all_scope_key(&mut game, 30);

        discard_after(&mut game, 20);

        // minting governs every key, whatever its scope -- All has no actors to trim away, but the
        // key itself still postdates the target and goes.
        assert!(!game.keys.contains_key(&key));
    }
}
