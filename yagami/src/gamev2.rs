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
    generate_seed,
    state::{GameId, WrappedServerState, lock_state},
    wirev2::{AdminControl, ServerInput},
};

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

// A sandboxed clock. It never reads the wall clock: a timestamp is a boot base offset plus how long
// this boot has been running on a monotonic Instant. The engine's job queue runs by action
// timestamp, so the stream must always move forward.
//
// The base is advanced on every boot to just past the last time the engine reached. That is the
// whole point of changing it per boot: if it were left fixed while `start` reset, the first ticks of
// the fresh child would rewind toward the old base -- likely below the last-executed time -- and the
// engine would re-run jobs already done. A new base per boot is what keeps offsets correct.
struct GameClock {
    base: Time,
    start: Instant,
}

impl GameClock {
    fn new() -> Self {
        Self {
            base: 0,
            start: Instant::now(),
        }
    }

    // the current sandboxed time: this boot's base plus elapsed since it began.
    fn now(&self) -> Time {
        self.base + self.start.elapsed().as_millis() as Time
    }

    // begin a fresh boot continuing from `reached`, the last time the previous engine executed up to.
    // bumping the base ensures strictly forward timestamps; leaving it fixed would let them rewind.
    fn reboot(&mut self, reached: Time) -> Time {
        self.base = self.base.max(reached + 1);
        self.start = Instant::now();
        self.now()
    }

    // move the base to `at` and restart the elapsed counter, so the sandbox clock sits exactly at
    // `at`. this allows motion in either direction -- which boot's forward-only clamp refuses --
    // and is what resetting `start` here makes correct for a forward jump that never boots.
    fn set_base(&mut self, at: Time) {
        self.base = at;
        self.start = Instant::now();
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
    last_reached: Time,           // highest timestamp the engine has executed
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

    // dispatch a request and fold in the bookkeeping common to every successful exchange: the engine
    // has now reached at least `request.timestamp`. on failure, reboot. returns whether the exchange
    // succeeded, so callers can do their own follow-up (accepted log, delivery).
    async fn execute(&mut self, request: &ActionRequest) -> bool {
        match self.dispatch(request).await {
            Ok(_result) => {
                self.last_reached = self.last_reached.max(request.timestamp);
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
            // begin a fresh, sandboxed time that continues past the old engine's last-executed
            // timestamp. see GameClock::reboot for why the base must change on every boot.
            self.clock.reboot(self.last_reached);

            let (stdin, stdout, child) = self.spawn_engine();
            self.child = Some(child);
            self.stdin = Some(stdin);
            self.stdout = Some(BufReader::new(stdout).lines());

            let mut ok = true;

            // replay every accepted input. success responses are discarded.
            let mut accepted = std::mem::take(&mut self.accepted);
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

    // ===== INITIALIZATION ===== //

    // gather whatever the game needs before the engine boots. a stub for now.
    async fn initialize(&mut self) {
        // TODO:
        // read the game's persistent state (config, players, keys) from the database.
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
                    // TODO:
                    // the key-management controls (create/revoke keys, capabilities, actor scope,
                    // profile). they live in control.rs.
                    _ => {}
                }
            }
            ServerInput::Action(request) => {
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

    // GoToTime: move the game to a past (or future) instant.
    //
    // Going FORWARD costs nothing structural: the engine is already at the current time and will
    // reach `target` naturally on its next tick, so all we do is advance the sandbox clock to it.
    //
    // Going BACKWARD is a rebuild: we truncate the accepted log to everything up to the target, set
    // the clock there, and boot the engine fresh from that base -- so it replays only the state that
    // existed up to the target time.
    async fn go_to_time(&mut self, target: Time) {
        // forward jump: no rebuild needed -- the engine is already at the current time. we still
        // drive it forward to `target` with a null tick so time-based state actually settles there,
        // rather than waiting for the next scheduled tick.
        if target >= self.clock.now() {
            self.clock.set_base(target);
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
        // allow the base to go BACKWARD; boot's reboot would clamp it forward and refuse.
        self.clock.set_base(target);
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
        lock_state(&self.server_state).games.remove(&self.game_id);
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
