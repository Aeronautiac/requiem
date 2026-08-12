// One game, one task.
//
// A select arm, once chosen, runs to completion and no other arm may interleave. That single fact
// collapses the old supervisor/watchdog/coordinator pile into one task and a handful of linear
// async helpers: spawn+boot, dispatch (write then read), and a select loop that feeds inputs and
// ticks through them one at a time. There is no supervisor task, and no boot handler arm.

use std::{
    collections::HashSet, env::current_exe, io::ErrorKind, process::Stdio, time::Duration, vec,
};

use lawliet_types::{
    ability::AbilityBehaviour,
    action::{Action, ActionActor, ActionRequest, AddPlayer, InitializeEngine, Null, SetTrueName},
    command::{Command, CommandPayload},
    common::{ActorKey, Seed, Time},
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
    auth::{ActorScope, Capability, Key, KeyData, Privileges, Ticket, to_flags},
    constants::{ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    delivery::{
        History, game_clock_output, key_roster_output, log_action_output, profile_roster_output,
    },
    generate_seed,
    names::NamePool,
    state::{GameHandle, GameId, WrappedServerState, lock_state},
    wire::{
        ActionOutcome, AdminControl, ControlError, ControlOutcome, ControlResponse, ExecOutcome,
        Profile, ResponsePair, ServerInput,
    },
};

// BUG:
// - display names are not properly maintained across time travel jumps
// - deaths are not rendered on the client across time jumps

pub fn to_line<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json + "\n",
        Err(e) => {
            eprintln!("failed to serialize for the engine pipe: {e} -- aborting");
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
    seed: Seed,
    last_reached: Time, // the latest timestamp executed by the engine, not necessarily
    // the highest ever executed. this distinction is important because time travel may change what
    // "the end of the timeline" is.
    clock: GameClock,             // sandboxed source of action timestamps
    accepted: Vec<ActionRequest>, // accepted engine inputs, replayed on boot
    history: History,             // the command log every connection walks from
    true_names: NamePool,         // true names drawn for unnamed players, tracking what is in play
    display_names: NamePool,      // display names drawn for unnamed players, tracking those in play

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
            true_names: NamePool::new(),
            display_names: NamePool::new(),
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
    // returns the outcome: Some when the engine processed the action (whether it accepted it or
    // refused), None when the engine crashed mid-action and had to be rebooted.
    async fn execute(
        &mut self,
        request: &ActionRequest,
        reply: Option<(Ticket, ServerInput)>,
    ) -> Option<ActionOutcome> {
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
                self.record_true_names(&commands);
                self.record_data_viewport(&commands);
                // a fresh player slot draws a display name before the roster is broadcast, so it
                // rides out in the same batch as the MapActor that entitles a connection to it.
                let profiles_changed = self.assign_display_names(&commands);

                let at = self.history.append_engine(commands);

                // the roster follows the engine commands it describes, inside the same broadcast.
                if profiles_changed {
                    self.append_profile_roster();
                }

                // a connection-requested action (one with a reply) is a request by a person, so its
                // record rides this same broadcast, after its own commands. server-initiated actions
                // (ticks, forward jumps) have no reply and are not recorded.
                if reply.is_some() {
                    self.append_log_action(request, outcome.clone());
                }

                let returned = outcome.clone();
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
                Some(returned)
            }
            Err(_) => {
                self.boot().await;
                // the action died in flight: tell whoever sent it, and record that it crashed.
                if let Some((ticket, input)) = reply {
                    let at = self.append_log_action(request, ActionOutcome::EnginePanic);
                    let pair = ResponsePair {
                        response: ExecOutcome::Action(ActionOutcome::EnginePanic),
                        input,
                    };
                    self.history
                        .broadcast(&self.server_state, at, Some((ticket, pair)));
                }
                None
            }
        }
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

    // scan one execution's output for the engine's world-data viewport announcement and remember
    // it, so server state (profiles) can be addressed to the same viewport actor existence rides.
    fn record_data_viewport(&self, commands: &[CommandPayload]) {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return;
        };
        record_data_viewport(game, commands);
    }

    // scan one execution's output for true-name announcements and tell the pool, so a later draw
    // never hands out a name that is already in play -- whether it was drawn by the server or typed
    // by an admin straight into the action. TrueNameUpdate is emitted on every SetTrueName,
    // including a player's first, so the pool learns the whole roster as it is built.
    fn record_true_names(&mut self, commands: &[CommandPayload]) {
        for payload in commands {
            if let Command::TrueNameUpdate { true_name, .. } = &payload.cmd {
                self.true_names.mark_taken(true_name);
            }
        }
    }

    // A newly-mapped PLAYER slot gets a display name drawn for it, so nobody is ever left rendering
    // as a raw `player-<slot>`. Cosmetic, like a colour in a lobby, and drawn independently of the
    // true name -- a display name may coincide with a true name (the two pools never share their
    // drawn-name sets), but never with another drawn display name. An admin replaces it with
    // SetProfile when they want.
    //
    // Encountering the MapActor for a player is when the profile is created: existence (MapActor)
    // and identity (profile) meet here, so a profile only ever exists for a slot that does. Returns
    // whether any profile changed, so the caller can append the roster behind this execution.
    fn assign_display_names(&mut self, commands: &[CommandPayload]) -> bool {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return false;
        };
        assign_display_names(game, &mut self.display_names, commands)
    }

    // The server names people, not their clients. An unnamed AddPlayer or SetTrueName is how an
    // admin asks for a drawn name; a TrueNameReroll is a player's own ability, so its name is ALWAYS
    // replaced -- letting the user choose is the whole thing being prevented. A name a request
    // carries is kept as sent. An exhausted reservoir leaves the request untouched, and the engine
    // refuses it as a duplicate -- a wrong name is worse than a failed action.
    fn assign_name(&mut self, request: &mut ActionRequest) {
        let unnamed = match &mut request.payload {
            Action::UseAbility(use_ability) => match &mut use_ability.ability_args {
                AbilityBehaviour::TrueNameReroll(reroll) => Some(&mut reroll.true_name),
                _ => None,
            },
            Action::AddPlayer(AddPlayer { true_name, .. }) if true_name.is_empty() => {
                Some(true_name)
            }
            Action::SetTrueName(SetTrueName { true_name, .. }) if true_name.is_empty() => {
                Some(true_name)
            }
            _ => None,
        };
        if let Some(slot) = unnamed
            && let Some(name) = self.true_names.draw()
        {
            *slot = name;
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

            // history is rebuilt from the replay as the engine is brought up: everything an
            // initialization action emits lands in the log, so a freshly-synced client can
            // reconstruct the same world a live one saw. Rebuild, not append, so a re-boot (or a
            // rewind) never duplicates commands that are already there, and truncated timelines
            // drop what never happened.
            self.history = History::new(self.game_id);

            let mut ok = true;

            // replay every accepted input, folding each one's commands into the rebuilt history and
            // re-recording it in the host's timeline, so a rebuilt history still shows who did what.
            let accepted = std::mem::take(&mut self.accepted);
            for request in &accepted {
                let Some(outcome) = self.fold_dispatch(request).await else {
                    ok = false;
                    break;
                };
                self.append_log_action(request, outcome);
            }
            self.accepted = accepted;

            // jump the engine up to the clock the previous child had reached, so jobs already
            // executed are not re-run. success responses are discarded.
            let t = self.last_reached;
            if ok && t > 0 {
                let request = ActionRequest {
                    actor: ActionActor::System,
                    timestamp: t,
                    payload: Action::Null(Null {}),
                };
                if self.fold_dispatch(&request).await.is_none() {
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
                match self.fold_dispatch(&request).await {
                    Some(outcome) => {
                        self.append_log_action(&request, outcome);
                        self.accepted.push(request);
                    }
                    None => ok = false,
                }
            }

            if ok {
                // history was rebuilt from scratch and rebuilt from the init replay: every
                // connection reconstructs against it. Lead it with the ledgers as they stand at
                // boot, so a freshly-joined client has the sets before replay fills the world -- a
                // rewind that prunes either ledger rests the roster on the pruned reality.
                self.append_key_roster();
                self.append_profile_roster();
                self.append_game_clock();
                self.history
                    .resync_all(&self.server_state, self.clock.now());
                return;
            }

            // the fresh child failed right out of the gate. discard it and try again.
            self.child = None;
            self.stdin = None;
            self.stdout = None;
        }
    }

    // dispatch one replay request and fold its command output into history. returns the action's
    // outcome: Some when the engine processed it, None when the dispatch failed (nothing folded).
    // the caller decides whether to record the action itself -- a boot's replay records each
    // accepted request so the host's timeline survives a rebuild, while the resume null tick does
    // not (it is server-initiated).
    async fn fold_dispatch(&mut self, request: &ActionRequest) -> Option<ActionOutcome> {
        match self.dispatch(request).await {
            Ok(inner) => {
                let outcome = match &inner {
                    Ok((response, _)) => ActionOutcome::Ok(response.clone()),
                    Err((error, _)) => ActionOutcome::Err(error.clone()),
                };
                self.append_execution(inner);
                Some(outcome)
            }
            Err(_) => None,
        }
    }

    // fold one execution's command context into history. an error still carries the world
    // progression that ran before the action failed, so a failed initialization still contributes
    // whatever it minted.
    fn append_execution(&mut self, result: ExecutionResult) {
        let commands = match result {
            Ok((_, context)) => context.commands,
            Err((_, context)) => context.commands,
        };
        // boot's own replay never passes through execute, so the data-viewport announcement it
        // makes is learned here, not only on the live path.
        self.record_data_viewport(&commands);
        self.history.append_engine(commands);
    }

    // append the admin-visible record of one action request and its outcome to history, right after
    // its engine commands, and return where it landed. same place in the log as the backing
    // `accepted` entry, so a boot rebuild replays both together and a host's timeline survives a
    // reboot or a rewind.
    fn append_log_action(&mut self, request: &ActionRequest, outcome: ActionOutcome) -> usize {
        self.history
            .append_server(vec![log_action_output(request, outcome, request.timestamp)])
    }

    // ===== INITIALIZATION AND TEARDOWN ===== //

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
                // replay the whole entitled history to this connection and install its cursor:
                // from now on `broadcast` advances it live. starts a sync before the socket reads a
                // single frame, so the connection is caught up before anything it sends is executed.
                self.history
                    .deliver_sync(&self.server_state, ticket, self.clock.now());
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
                self.execute_control(ticket, control).await;
            }
            ServerInput::Action(mut request) => {
                // override the client's reported value. only the game task truly knows the game's
                // virtual clock, and a client cannot be trusted.
                request.timestamp = self.clock.now();
                // ... and, like the timestamp, the server names people rather than their clients:
                // an unnamed request gets a fresh draw before the engine ever sees it.
                self.assign_name(&mut request);

                if !self.authorize_action(&ticket, &request) {
                    // a denied request is still a request, so it is recorded for the host; nothing
                    // else was appended, so the reply rides from the record it sits right after.
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
                if self
                    .execute(&request, Some((ticket, reply_input)))
                    .await
                    .is_none()
                {
                    // the crash is answered (and recorded) inside execute.
                    return;
                }
                // this input ran and was accepted, so it is part of the engine's state and
                // must be replayed on the next boot.
                self.accepted.push(request);
            }
        }
    }

    // ===== CONTROL EXECUTION ===== //

    // carry out one control -- the control analog of `execute`. It authorizes, then hands each
    // control variant to its own handler; no variant is special-cased out of the match, and a
    // control is just another execution -- one that writes the server's ledger (keys, profiles)
    // instead of the engine's, so each handler appends whatever roster its change rewrites and
    // the reply rides one broadcast from there.
    async fn execute_control(&mut self, ticket: Ticket, control: AdminControl) {
        if !self.authorize_control(&ticket) {
            let pair = ResponsePair {
                response: ExecOutcome::Control(ControlOutcome::Denied),
                input: ServerInput::Control(control),
            };
            self.history.broadcast(
                &self.server_state,
                self.history.head(),
                Some((ticket, pair)),
            );
            return;
        }
        let now = self.clock.now();
        match control {
            AdminControl::GoToTime { time } => {
                self.go_to_time(time).await;
                // the jump itself already replayed (or rebuilt) history and resyncs every
                // connection; all this reply does is settle the requesting control so the client's
                // positional reply queue is not left with a dangling entry for the NEXT action.
                // (see session #waiting -- every control must answer.)
                let pair = ResponsePair {
                    response: ExecOutcome::Control(ControlOutcome::Ok(ControlResponse::TimeSet)),
                    input: ServerInput::Control(AdminControl::GoToTime { time }),
                };
                self.history.broadcast(
                    &self.server_state,
                    self.history.head(),
                    Some((ticket, pair)),
                );
            }
            AdminControl::CreateKey {
                actors,
                capabilities,
            } => self.create_key_control(ticket, now, actors, capabilities),
            AdminControl::RevokeKey { key } => self.revoke_key_control(ticket, now, key),
            AdminControl::SetCapabilities { key, capabilities } => {
                self.set_capabilities_control(ticket, now, key, capabilities)
            }
            AdminControl::SetActorScope { key, actors } => {
                self.set_actor_scope_control(ticket, now, key, actors)
            }
            AdminControl::SetProfile { actor, profile } => {
                self.set_profile_control(ticket, now, actor, profile)
            }
        }
    }

    fn create_key_control(
        &mut self,
        ticket: Ticket,
        now: Time,
        actors: ActorScope,
        capabilities: Vec<Capability>,
    ) {
        let outcome = self.ledger_control(&ticket, now, |game, _caller, supervises, now| {
            let capabilities = to_flags(&capabilities);
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
        });
        let at = if matches!(outcome, ControlOutcome::Ok(_)) {
            self.append_key_roster()
        } else {
            self.history.head()
        };
        self.broadcast_control(
            ticket,
            AdminControl::CreateKey {
                actors,
                capabilities,
            },
            outcome,
            at,
        );
    }

    fn revoke_key_control(&mut self, ticket: Ticket, now: Time, key: Key) {
        let outcome = self.ledger_control(&ticket, now, |game, caller, supervises, _now| {
            may_manage(game, caller, supervises, &key)?;
            revoke_key(game, &key);
            Ok(ControlResponse::KeyRevoked)
        });
        let at = if matches!(outcome, ControlOutcome::Ok(_)) {
            self.append_key_roster()
        } else {
            self.history.head()
        };
        self.broadcast_control(ticket, AdminControl::RevokeKey { key }, outcome, at);
    }

    fn set_capabilities_control(
        &mut self,
        ticket: Ticket,
        now: Time,
        key: Key,
        capabilities: Vec<Capability>,
    ) {
        let outcome = self.ledger_control(&ticket, now, |game, caller, supervises, _now| {
            may_manage(game, caller, supervises, &key)?;
            let capabilities = to_flags(&capabilities);
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }
            let key_data = game
                .keys
                .get_mut(&key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.capabilities = capabilities;
            apply_privilege_change(game, &key, before);
            Ok(ControlResponse::CapabilitiesSet)
        });
        let at = if matches!(outcome, ControlOutcome::Ok(_)) {
            let at = self.append_key_roster();
            // a privilege change resets the affected key's live connections so its clients see the
            // new set -- and any narrowed scope -- from scratch.
            self.resync_key(&key);
            at
        } else {
            self.history.head()
        };
        self.broadcast_control(
            ticket,
            AdminControl::SetCapabilities { key, capabilities },
            outcome,
            at,
        );
    }

    fn set_actor_scope_control(&mut self, ticket: Ticket, now: Time, key: Key, actors: ActorScope) {
        let outcome = self.ledger_control(&ticket, now, |game, caller, supervises, _now| {
            may_manage(game, caller, supervises, &key)?;
            let key_data = game
                .keys
                .get_mut(&key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.actors = actors.clone();
            apply_privilege_change(game, &key, before);
            Ok(ControlResponse::ActorScopeSet)
        });
        let at = if matches!(outcome, ControlOutcome::Ok(_)) {
            let at = self.append_key_roster();
            self.resync_key(&key);
            at
        } else {
            self.history.head()
        };
        self.broadcast_control(
            ticket,
            AdminControl::SetActorScope { key, actors },
            outcome,
            at,
        );
    }

    fn set_profile_control(
        &mut self,
        ticket: Ticket,
        now: Time,
        actor: ActorKey,
        profile: Profile,
    ) {
        let outcome = self.ledger_control(&ticket, now, |game, _caller, _supervises, now| {
            game.profiles.insert(actor, profile.clone());
            game.profile_created.insert(actor, now);
            Ok(ControlResponse::ProfileSet)
        });
        let at = if matches!(outcome, ControlOutcome::Ok(_)) {
            self.append_profile_roster()
        } else {
            self.history.head()
        };
        self.broadcast_control(
            ticket,
            AdminControl::SetProfile { actor, profile },
            outcome,
            at,
        );
    }

    // resolve the caller's authority and run one ledger mutation under the lock, returning its
    // outcome. an unreachable caller entry denies.
    fn ledger_control(
        &self,
        ticket: &Ticket,
        now: Time,
        run: impl FnOnce(&mut GameHandle, &Key, bool, Time) -> Result<ControlResponse, ControlError>,
    ) -> ControlOutcome {
        let mut server_state = lock_state(&self.server_state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return ControlOutcome::Denied;
        };
        let Some(caller_key) = game.tickets.get(ticket).cloned() else {
            return ControlOutcome::Denied;
        };
        let Some(caller) = game.keys.get(&caller_key) else {
            return ControlOutcome::Denied;
        };
        let supervises = caller
            .privileges
            .capabilities
            .contains(Capability::Supervise);
        match run(game, &caller_key, supervises, now) {
            Ok(response) => ControlOutcome::Ok(response),
            Err(error) => ControlOutcome::Err(error),
        }
    }

    // reply to a control from `at` -- where its roster was appended, or head for a no-op --
    // carrying both the outcome and any output the change appended in one broadcast.
    fn broadcast_control(
        &self,
        ticket: Ticket,
        control: AdminControl,
        outcome: ControlOutcome,
        at: usize,
    ) {
        let pair = ResponsePair {
            response: ExecOutcome::Control(outcome),
            input: ServerInput::Control(control),
        };
        self.history
            .broadcast(&self.server_state, at, Some((ticket, pair)));
    }

    // snapshot the whole current key ledger into history as a live KeyRoster. Rosters travel the
    // log like any other output -- see the TODO by the imports -- so a fresh admin connection
    // replays the set in the state each change left it, and the broadcast that follows a control
    // delivers this entry to every admin that passes its Admin gate.
    //
    // Returns the history position to walk from when broadcasting, so the caller's control reply
    // rides a batch that includes this roster: `head()` before the append. A no-op (game gone)
    // returns the current head, i.e. nothing new to deliver.
    fn append_key_roster(&mut self) -> usize {
        let start = self.history.head();
        let now = self.clock.now();
        let output = {
            let server_state = lock_state(&self.server_state);
            match server_state.games.get(&self.game_id) {
                Some(game) => key_roster_output(&game.keys, now),
                None => return start,
            }
        };
        self.history.append_server(vec![output]);
        start
    }

    // snapshot the whole profile ledger into history, aimed at the world-data viewport (the one
    // actor existence rides -- see record_data_viewport). Because it shares that viewport, anyone
    // who can read it has been walked the mappings it names. Appended on every profile change and
    // at boot, so a rewind that prunes profiles rewrites the roster its clients replay.
    //
    // Returns the history position to walk from when broadcasting, as @[append_key_roster].
    fn append_profile_roster(&mut self) -> usize {
        let start = self.history.head();
        let now = self.clock.now();
        let output = {
            let server_state = lock_state(&self.server_state);
            let Some(game) = server_state.games.get(&self.game_id) else {
                return start;
            };
            let Some(viewport) = game.data_viewport else {
                return start; // the world-data viewport has not been announced yet
            };
            profile_roster_output(viewport, &game.profiles, now)
        };
        self.history.append_server(vec![output]);
        start
    }

    // snapshot the game's current clock into history as a live GameClock, aimed at the world-data
    // viewport like the ProfileRoster it sits beside. It is game-wide state, not connection context,
    // so it is appended to the shared log and replayed -- a fresh boot (or a rewind, which boots
    // again) rewrites it, and any client that can read the world sees the current anchor. A client
    // derives the current game time from `time` + (wall_now - sent_at).
    //
    // Returns the history position to walk from when broadcasting, as @[append_key_roster]. A
    // no-op (game gone, or the world-data viewport not yet announced) returns the current head.
    fn append_game_clock(&mut self) -> usize {
        let start = self.history.head();
        let now = self.clock.now();
        let output = {
            let server_state = lock_state(&self.server_state);
            let Some(game) = server_state.games.get(&self.game_id) else {
                return start;
            };
            let Some(viewport) = game.data_viewport else {
                return start; // the world-data viewport has not been announced yet
            };
            let sent_at = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .expect("wall clock before epoch")
                .as_millis();
            game_clock_output(viewport, now, sent_at)
        };
        self.history.append_server(vec![output]);
        start
    }

    // deliver a full resync to every live connection held by `key` -- the "an Initialize batch"
    // reset that rebuilds the client's view of the game under the key's current privileges. Used on
    // a privilege change (narrowing and widening both) so the client's standing and reach are told
    // from the new ledger, never a stale one.
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
            // the clock was wound forward; re-anchor every client's game time on the new baseline.
            let at = self.append_game_clock();
            self.history.broadcast(&self.server_state, at, None);
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
        self.history
            .resync_all(&self.server_state, self.clock.now());
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
        // dropping `self` runs Drop, which tears the game out of state and cancels it, and dropping
        // the child reaps the engine (kill_on_drop).
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

// permission enforcement, input executions, live client updates, and engine process management.
pub async fn game(
    state: WrappedServerState,
    game_id: GameId,
    events: mpsc::UnboundedReceiver<GameInput>,
    cancel: CancellationToken,
) {
    Game::new(game_id, state, cancel, events).run().await;
}

// ===== KEY AUTHORITY ===== //

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

// see Game::assign_display_names.
fn assign_display_names(
    game: &mut GameHandle,
    display_names: &mut NamePool,
    commands: &[CommandPayload],
) -> bool {
    let mut changed = false;
    for payload in commands {
        let Command::MapActor {
            actor_id,
            kind: lawliet_types::actor::ActorKind::Player,
        } = &payload.cmd
        else {
            continue;
        };
        if game.profiles.contains_key(actor_id) {
            continue;
        }
        let Some(display_name) = display_names.draw() else {
            continue;
        };
        game.profiles.insert(
            *actor_id,
            Profile {
                display_name: Some(display_name),
            },
        );
        game.profile_created.insert(*actor_id, payload.timestamp);
        changed = true;
    }
    changed
}

// learn which viewport the engine treats as the world-data viewport -- the one actor existence
// rides and profiles must ride too -- from its own announcement. announced exactly once, on boot,
// so the first (and only) match sticks.
fn record_data_viewport(game: &mut GameHandle, commands: &[CommandPayload]) {
    for payload in commands {
        if let Command::MapViewport { viewport, kind } = &payload.cmd
            && *kind == lawliet_types::viewport::ViewportKind::WorldData
        {
            game.data_viewport = Some(*viewport);
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

    // a key minted after `target` did not exist yet, so it is cut -- and every connection under it
    // is torn down the same way a revocation is (cancel the token + drop the socket), so its clients
    // are TOLD they were disconnected rather than left socket-open-but-starved until they go silent.
    let dropped: Vec<Key> = game
        .keys
        .iter()
        .filter(|(key, _)| !game.key_created.get(key).is_some_and(|t| *t <= target))
        .map(|(key, _)| key.clone())
        .collect();
    for key in dropped {
        revoke_key(game, &key);
    }

    // a surviving key keeps its identity but loses scope over actors that did not exist yet.
    for key_data in game.keys.values_mut() {
        if let crate::auth::ActorScope::Only(actors) = &mut key_data.privileges.actors {
            actors.retain(|actor| game.actor_created.get(actor).is_some_and(|t| *t <= target));
        }
    }
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
            data_viewport: None,
        }
    }

    fn map_actor(who: ActorKey, at: Time) -> CommandPayload {
        map_actor_kind(who, lawliet_types::actor::ActorKind::Player, at)
    }

    fn map_actor_kind(
        who: ActorKey,
        kind: lawliet_types::actor::ActorKind,
        at: Time,
    ) -> CommandPayload {
        CommandPayload {
            timestamp: at,
            recipient: CommandRecipient::Viewport(KeyData::from_ffi(1 | (1 << 32)).into()),
            cmd: Command::MapActor {
                actor_id: who,
                kind,
            },
        }
    }

    fn mint_key(game: &mut GameHandle, actors: &[ActorKey], at: Time) -> Key {
        // plain state setup for the rewind tests: a key with an enumerated scope and none of the
        // capabilities. nothing here is under test -- it just stands in for a minted key.
        let key = Key::generate();
        game.keys.insert(
            key.clone(),
            crate::auth::KeyData {
                cancel: CancellationToken::new(),
                tickets: HashSet::new(),
                privileges: Privileges {
                    actors: ActorScope::Only(actors.iter().copied().collect()),
                    capabilities: BitFlags::empty(),
                },
            },
        );
        game.key_created.insert(key.clone(), at);
        key
    }

    fn set_profile(game: &mut GameHandle, who: ActorKey, at: Time) {
        // plain state setup for the rewind tests: stamp a profile for a slot at a time.
        game.profiles.insert(who, Profile::default());
        game.profile_created.insert(who, at);
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

    #[test]
    fn a_player_mapping_draws_a_profile_and_names_never_collide() {
        let mut game = handle();
        let mut names = NamePool::new();
        // two players mapped in the same execution.
        let cmds = vec![map_actor(actor(1), 10), map_actor(actor(2), 20)];

        assert!(assign_display_names(&mut game, &mut names, &cmds));

        let name_1 = game
            .profiles
            .get(&actor(1))
            .and_then(|p| p.display_name.as_deref())
            .expect("player 1 is named");
        let name_2 = game
            .profiles
            .get(&actor(2))
            .and_then(|p| p.display_name.as_deref())
            .expect("player 2 is named");
        assert_ne!(name_1, name_2);
    }

    #[test]
    fn an_existing_profile_is_never_overwritten() {
        let mut game = handle();
        let mut names = NamePool::new();
        set_profile(&mut game, actor(1), 5);

        // the slot is mapped later, but its profile already exists (an admin set one).
        assert!(!assign_display_names(
            &mut game,
            &mut names,
            &[map_actor(actor(1), 10)]
        ));
        // the pre-existing profile (default: no display name) is untouched.
        assert_eq!(game.profiles.get(&actor(1)).unwrap().display_name, None);
    }

    #[test]
    fn non_player_mappings_get_no_profile() {
        let mut game = handle();
        let mut names = NamePool::new();
        let cmds = vec![map_actor_kind(
            actor(1),
            lawliet_types::actor::ActorKind::Org(
                lawliet_types::organization::OrganizationName::SPK,
            ),
            10,
        )];

        assert!(!assign_display_names(&mut game, &mut names, &cmds));
        assert!(!game.profiles.contains_key(&actor(1)));
    }
}
