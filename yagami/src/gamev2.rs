// One game, one task.
//
// A select arm, once chosen, runs to completion and no other arm may interleave. That single fact
// collapses the old supervisor/watchdog/coordinator pile into one task and a handful of linear
// async helpers: spawn+boot, dispatch (write then read), and a select loop that feeds inputs and
// ticks through them one at a time. There is no supervisor task, and no boot handler arm.

use std::{env::current_exe, io::ErrorKind, process::Stdio, time::Duration};

use lawliet_types::{
    action::{Action, ActionActor, ActionRequest, InitializeEngine, Null},
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
    auth::Ticket,
    constants::{ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    control::handle_control,
    generate_seed,
    http::req,
    state::{GameId, WrappedServerState, lock_state},
    wire::{ControlOutcome as WireControlOutcome, GameControl},
    wirev2::{AdminControl, ControlOutcome, ServerInput},
};

// delivery pipeline:
// engine reply:
// iterate over every command, match the command, and do something if it has a designated handler
// turn the command into a server command, and add it to the batch
// do this for every command
// hand it off the delivery pipeline to finish it off (for things like viewports)
// let delivery handle the networking
//
// game command/control:
// no initial transformation step. just create the batch and hand off to delivery.

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

    // dispatch a request and fold in the bookkeeping common to every successful exchange.
    // on failure, reboot.
    // returns whether the exchange succeeded, so callers can do their own follow-up
    // (accepted log, delivery).
    // also logs the current last executed time on success.
    async fn execute(&mut self, request: &ActionRequest) -> bool {
        match self.dispatch(request).await {
            Ok(_result) => {
                self.last_reached = request.timestamp;
                true
            }
            Err(_) => {
                self.boot().await;
                false
            }
        }
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
                // TODO:
                // synchronize the connection behind `ticket` to the current game state.
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
                    // TODO:
                    // deliver ControlOutcome::Denied to `ticket`.
                    return;
                }
                match control {
                    AdminControl::GoToTime { time } => self.go_to_time(time).await,
                    AdminControl::CreateKey {
                        actors,
                        capabilities,
                    } => {
                        let outcome = self.handle_key_control(
                            &ticket,
                            &GameControl::CreateKey {
                                actors,
                                capabilities,
                            },
                        );
                        self.deliver_control_feedback(&ticket, outcome);
                    }
                    AdminControl::RevokeKey { key } => {
                        let outcome =
                            self.handle_key_control(&ticket, &GameControl::RevokeKey { key });
                        self.deliver_control_feedback(&ticket, outcome);
                    }
                    AdminControl::SetCapabilities { key, capabilities } => {
                        let outcome = self.handle_key_control(
                            &ticket,
                            &GameControl::SetCapabilities { key, capabilities },
                        );
                        self.deliver_control_feedback(&ticket, outcome);
                    }
                    AdminControl::SetActorScope { key, actors } => {
                        let outcome = self.handle_key_control(
                            &ticket,
                            &GameControl::SetActorScope { key, actors },
                        );
                        self.deliver_control_feedback(&ticket, outcome);
                    }
                    AdminControl::SetProfile { actor, profile } => {
                        let outcome = self.handle_key_control(
                            &ticket,
                            &GameControl::SetProfile { actor, profile },
                        );
                        self.deliver_control_feedback(&ticket, outcome);
                    }
                }
            }
            ServerInput::Action(mut request) => {
                // override the client's reported value. only the game task truly knows the game's
                // virtual clock, and a client cannot be trusted.
                request.timestamp = self.clock.now();

                if !self.authorize_action(&ticket, &request) {
                    // TODO:
                    // deliver ActionOutcome::Denied to `ticket`.
                    return;
                }
                if !self.execute(&request).await {
                    // TODO:
                    // respond to `ticket` with an engine failure / crash outcome.
                    return;
                }
                // this input ran and was accepted, so it is part of the engine's state and
                // must be replayed on the next boot.
                self.accepted.push(request);
                // TODO:
                // pre-processing of the response (log, extend history, mirror state).
                // TODO:
                // payload delivery.
            }
        }
    }

    // run one key-management control through the shared control.rs logic and adapt its outcome to
    // the v2 wire form. authorization (caller holds Administer) is re-checked inside handle_control.
    fn handle_key_control(&self, ticket: &Ticket, control: &GameControl) -> ControlOutcome {
        let outcome = handle_control(
            &self.server_state,
            self.game_id,
            ticket,
            control,
            &self.cancel,
        );
        match outcome {
            WireControlOutcome::Ok(response) => ControlOutcome::Ok(response),
            WireControlOutcome::Err(error) => ControlOutcome::Err(error),
            WireControlOutcome::Denied => ControlOutcome::Denied,
        }
    }

    // reply to a control. the feedback is immediate -- it never touches the engine -- so it rides
    // out as its own payload rather than a response to a dispatch.
    fn deliver_control_feedback(&self, _ticket: &Ticket, _outcome: ControlOutcome) {
        // TODO:
        // send `_outcome` to `_ticket` as immediate feedback payload.
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
            if !self.execute(&request).await {
                return;
            }
            // TODO:
            // pre-processing.
            // TODO:
            // payload delivery.
            return;
        }

        // anything accepted after the target never happened, so it is dropped from what a boot
        // replays. the engine no longer knows, or is responsible for, the invalidated future.
        self.accepted.retain(|request| request.timestamp <= target);
        self.last_reached = target;
        self.boot().await;

        // TODO:
        // trigger a resync (reinitialize) on every connection -- wirev2::BatchKind::Initialize.

        // TODO:
        // server-side state minted after `target` -- keys, profiles, etc. -- must also be
        // discarded. they were built on a timeline that no longer exists, so keeping them would
        // leave opinions standing on a foundation that was rolled back.

        // TODO:
        // the time after `target` is only truly invalidated once a new input arrives at the new
        // base. that will need a timeline pointer to know which later branch is live.
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
        if !self.execute(&request).await {
            return;
        }
        // TODO:
        // pre-processing.
        // TODO:
        // payload delivery.
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
