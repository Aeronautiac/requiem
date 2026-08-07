use std::{collections::HashMap, env::current_exe, io::ErrorKind, process::Stdio, time::Duration};

use lawliet_types::{
    ability::AbilityBehaviour,
    action::{Action, ActionActor, ActionError, ActionRequest, InitializeEngine, Null},
    actor::ActorKind,
    command::{Command, CommandPayload, TapInOutcome},
    common::{ActorKey, Seed, Time},
    engine::ExecutionResult,
};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    select,
    sync::mpsc::{self},
    time::{Instant, Interval, MissedTickBehavior, interval, sleep_until},
};
use tokio_util::sync::CancellationToken;

use crate::{
    auth::{Key, Privileges, Ticket},
    constants::{ENGINE_TIMEOUT, NULL_TICK_INTERVAL},
    control::handle_control,
    delivery::{History, broadcast, deliver_catchup, deliver_crash, deliver_widening},
    generate_seed,
    names::NamePool,
    now,
    state::{GameId, WrappedServerState, lock_state},
    wire::{ActionOutcome, ExecOutcome, GameControl, Profile, ResponsePair, ServerInput},
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
    EngineErr(ActionError),
    GameErr(GameError),
}

pub struct ShellOutput {}

// carries the source ticket so the game task can route replies and enforce permissions.
pub struct InputEnvelope {
    pub ticket: Ticket,
    pub input: ServerInput,
}

pub enum GameEvent {
    Attach { ticket: Ticket },
    Widen { ticket: Ticket, before: Privileges },
    Input(InputEnvelope),
}

// the harness ticks, not the shell
// on every tick, it ticks forward all shells

// history becomes a harness level concern.
// a harness can potentially manage multiple game shells. imagine parallel timelines.

// key creation trace:
// CreateKey {
//  privileges
// }
// -> harness
// -> InsertKey {
//  privileges
//  token
// } -> shell
// -> harness
// -> Ok(ServerCommand (show ticket to admin))/ShellErr

struct GameHarness {
    // identity / routing
    game_id: GameId,
    server_state: WrappedServerState,
    cancel: CancellationToken,
}

// the action the engine is currently working on.
pub struct InFlight {
    ticket: Option<Ticket>,
    logged: bool,
    request: ActionRequest,
}

// this becomes part of the game shell
struct Coordinator {
    // identity / routing
    game_id: GameId,
    server_state: WrappedServerState,
    cancel: CancellationToken,

    // channels
    events: mpsc::UnboundedReceiver<GameEvent>,
    fd_out: mpsc::UnboundedReceiver<(ChildStdin, ChildStdout)>,
    kill_in: mpsc::Sender<()>,

    // state
    seed: Seed,
    names: NamePool,
    true_names: HashMap<ActorKey, String>,
    clock: Time,                  // current engine time
    history: History,             // commands
    accepted: Vec<ActionRequest>, // accepted inputs

    // process comms
    stdin: Option<ChildStdin>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,

    // lifecycle
    in_flight: Option<InFlight>,
    to_discard: usize,
    deadline: Option<Instant>,
    tick: Interval,
}

impl Coordinator {
    fn new(
        game_id: GameId,
        server_state: WrappedServerState,
        cancel: CancellationToken,
        events: mpsc::UnboundedReceiver<GameEvent>,
        fd_out: mpsc::UnboundedReceiver<(ChildStdin, ChildStdout)>,
        kill_in: mpsc::Sender<()>,
    ) -> Self {
        // Delay rather than the default Burst: if the coordinator was busy through several tick
        // periods, firing all of them back-to-back is pure waste. a tick catches the engine up to
        // NOW, so one late tick does everything the missed ones would have.
        let mut tick = interval(Duration::from_secs(NULL_TICK_INTERVAL));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        Self {
            game_id,
            server_state,
            cancel,
            events,
            fd_out,
            kill_in,

            // drawn once, here, rather than per boot: it rides in the InitializeEngine action, so a
            // rebuilt child replaying that action reproduces the same RNG stream. a fresh seed per
            // boot would make every crash silently fork the game's randomness.
            seed: generate_seed(),
            names: NamePool::new(),
            // Every player's current true name, mirrored from what the engine announced. The engine
            // stays the authority; this exists only so a reroll can be handed a name nobody holds,
            // which is a question the coordinator cannot ask the engine directly -- it lives in
            // another process.
            true_names: HashMap::new(),
            // the highest timestamp the engine has actually executed at.
            //
            // The engine's progress through its job queue is driven entirely by the timestamps of
            // the actions it receives -- execute() pops every job at or before the action's stamp.
            // So `accepted` alone does NOT describe how far the engine got: a null tick and a
            // rejected action both run the queue and have their commands logged, and neither is
            // replayable. Rebuilding from `accepted` leaves those jobs sitting in the fresh child's
            // queue, and the next tick pops them a second time -- re-emitting commands that are
            // already in `history`, after the discard count is spent, so they are appended and fanned
            // out for real. That is duplicated deaths and world events, permanently in the log, on
            // every reconnect.
            //
            // Replaying one Null at this timestamp is what closes it: it says "the engine had reached
            // here", which is the fact `accepted` cannot carry.
            clock: 0,
            // every command ever emitted, in emission order, plus its viewport index. "per-recipient
            // logs" are a FILTER over this, not separate storage -- one log is what keeps
            // cross-recipient order intact for free.
            history: History::default(),
            // every action the engine accepted, in order. the source of truth for rebuilding a fresh
            // child, and what a persistent log would store.
            accepted: Vec::new(),

            stdin: None,
            stdout: None,

            in_flight: None,
            // replay responses still to be swallowed. a rebuilt child re-emits every command it
            // already emitted; `history` already holds them, so the echoes must not be logged or
            // fanned out again.
            to_discard: 0,
            // watchdog: when the engine owes us a line, when we stop waiting for it. armed and
            // disarmed in one place, at the bottom of the loop.
            deadline: None,
            tick,
        }
    }

    // THIS HAS BEEN VERIFIED BY ME, DO THE OTHER ONES NEXT
    // this runs when the game task receives a new set of file descriptors. this is the supervisor's
    // boot signal.
    // a boot is either a crash, an explicit reboot (time travel, which will be added soon), or the initial boot.
    async fn handle_boot(&mut self, fds: (ChildStdin, ChildStdout)) {
        let state = &self.server_state;
        let game_id = self.game_id;

        // must be reset on reboot, because it is possible for us to jump to a different branch
        // DURING the rehydration period, and this is intentional, because without the reader
        // running concurrently, it would be possible for us to fill the pipe and deadlock with
        // large logs as pipe transport is handled by a fixed size buffer in the kernel.
        //
        // one potential failure case if this were not reset to 0:
        // we fail at some point during rehydration, and we restart with a to_discard value larger
        // than 0, a stale value from a previous rehydration cycle.
        // rehydration increments it, and we succeed in full rehydration.
        // the value is permanently offset by the stale value, and the first few real player inputs
        // following the rehydration have their responses discarded because the value does not
        // directly map to the actual rehydration input count.
        self.to_discard = 0;

        // drain to the newest pair. a crash-loop can queue several, and every pair but the last
        // belongs to a child that is already dead -- dropping them closes those pipes, which is
        // exactly what we want.
        let mut fds = fds;
        while let Ok(newer) = self.fd_out.try_recv() {
            fds = newer;
        }
        let (new_in, new_out) = fds;

        // an action that was in flight when the pipe died is the prime suspect for having killed it,
        // so it is NOT added to `accepted` and never replayed. its originator is told, rather than
        // left waiting on a reply that will never come.
        if let Some(InFlight {
            ticket, request, ..
        }) = self.in_flight.take()
        {
            // TODO:
            // delivery rework
            deliver_crash(state, game_id, ticket, request);
        }

        self.stdin = Some(new_in);
        self.stdout = Some(BufReader::new(new_out).lines());

        // initial rehydration
        // when time travel is added, send only while the input's time is <= target time
        for request in &self.accepted {
            let line = to_line(request);
            if self
                .stdin
                .as_mut()
                .unwrap()
                .write_all(line.as_bytes())
                // it is possible for us to jump to a different branch
                // here, because stdin has already been set by this point.
                .await
                .is_err()
            {
                self.discard_engine();
                // the rest of the boot handler initializates the engine on first boot, and jumps
                // forward to the last previous engine time otherwise. it all relies on stdin being
                // available, so it is safe to return here.
                return;
            }
            // must mutate the value directly here rather than accumulating and then setting it all
            // at once, because if we move to another task during a rehydration, and we have not
            // updated this value yet, but we receive a response, the commands may not be discarded
            // even though they should have been.
            self.to_discard += 1;
        }

        // jump to previous engine time, or when time travel is added, the remaining distance to the
        // target time
        if self.clock > 0 && self.stdin.is_some() {
            let line = to_line(&ActionRequest {
                actor: ActionActor::System,
                timestamp: self.clock,
                payload: Action::Null(Null {}),
            });
            if self
                .stdin
                .as_mut()
                .unwrap()
                .write_all(line.as_bytes())
                .await
                .is_err()
            {
                self.discard_engine();
                return;
            } else {
                self.to_discard += 1;
            }
        }

        // issued only when nothing has been accepted yet, which is always the first boot. once it
        // succeeds it lives in `accepted` like any other action, so every subsequent rebuild replays
        // it with its original seed, keeping the engine's RNG stream identical across a crash.
        if self.accepted.is_empty() && self.stdin.is_some() {
            let request = ActionRequest {
                actor: ActionActor::System,
                timestamp: now(),
                payload: Action::InitializeEngine(InitializeEngine { seed: self.seed }),
            };

            let line = to_line(&request);
            if self
                .stdin
                .as_mut()
                .unwrap()
                .write_all(line.as_bytes())
                .await
                .is_err()
            {
                self.discard_engine();
            } else {
                self.in_flight = Some(InFlight {
                    ticket: None,
                    logged: true,
                    request,
                });
            }
        }
    }

    // our options:
    // - loop through every connection, and attach the ones which have not yet been attached
    // - have connections send an explicit attach signal on creation
    // an explicit attach signal is more efficient.
    // an initialization batch is sent on attach, or in any case requiring client resynchronization
    // under new conditions.

    // a connection attaching, or an input from one. gated on the engine being reachable with nothing
    // in flight, so the channel itself is the queue -- and because it is ONE channel, a connection's
    // Attach is always handled before anything it goes on to send.
    // a connection attaching, or an input from one. gated on the engine being reachable with nothing
    // in flight, so the channel itself is the queue -- and because it is ONE channel, a connection's
    // Attach is always handled before anything it goes on to send.
    async fn handle_input(&mut self, event: GameEvent) {
        let state = &self.server_state;
        let game_id = self.game_id;

        let InputEnvelope { ticket, input } = match event {
            GameEvent::Attach { ticket } => {
                deliver_catchup(state, game_id, &self.history, &ticket);
                return;
            }
            GameEvent::Widen { ticket, before } => {
                deliver_widening(state, game_id, &self.history, &ticket, &before);
                return;
            }
            GameEvent::Input(envelope) => envelope,
        };

        match input {
            // controls are handled HERE and never forwarded: they act ON the timeline, not in the
            // fiction, and the engine has no concept of them.
            ServerInput::Control(control) => {
                self.handle_control_input(ticket, control);
            }
            ServerInput::Action(request) => self.handle_action(ticket, request).await,
        }
    }

    // controls carry no actor, so authority is a matter of capabilities and of the target key -- all
    // of which lives in handle_control. replied to even for EndGame, where the teardown races the
    // send and will usually win: uniform beats special-casing a reply nobody is waiting on.
    fn handle_control_input(&mut self, ticket: Ticket, control: GameControl) {
        let state = &self.server_state;
        let game_id = self.game_id;

        let outcome = handle_control(state, game_id, &ticket, &control, &self.cancel);
        let pair = ResponsePair {
            input: ServerInput::Control(control),
            output: ExecOutcome::Control(outcome),
        };
        broadcast(
            state,
            game_id,
            &self.history,
            self.history.head(),
            Some((ticket, pair)),
        );
    }

    // auth, naming, and dispatch of one action into the engine.
    async fn handle_action(&mut self, ticket: Ticket, mut request: ActionRequest) {
        // auth: a connection may only act as an actor its key's privilege set permits. checked here
        // rather than at the socket because the privilege set is resolved fresh every time, so a
        // narrowed key takes effect on its live sockets at once.
        if !self.authorize(&ticket, &request) {
            self.deny(ticket, request);
            return;
        }

        self.assign_name(&mut request);
        self.dispatch(ticket, request).await
    }

    // whether the connection behind `ticket` is permitted to act as `request.actor` under its
    // current privilege set.
    fn authorize(&self, ticket: &Ticket, request: &ActionRequest) -> bool {
        let server_state = lock_state(&self.server_state);
        server_state
            .games
            .get(&self.game_id)
            .and_then(|game| game.privileges(ticket))
            .is_some_and(|privileges| privileges.can_act_as(&request.actor))
    }

    // reply to a denied action. nothing was logged, so there are no commands to go with it.
    fn deny(&self, ticket: Ticket, request: ActionRequest) {
        let pair = ResponsePair {
            input: ServerInput::Action(request),
            output: ExecOutcome::Action(ActionOutcome::Denied),
        };
        broadcast(
            &self.server_state,
            self.game_id,
            &self.history,
            self.history.head(),
            Some((ticket, pair)),
        );
    }

    // The server names people, not their clients: a true name a player picked for itself would be
    // worth nothing to anybody. Same treatment as the timestamp, but done here rather than at the
    // socket because only the coordinator knows which names are already in play.
    //
    // A reroll is ALWAYS replaced -- it is a player's own ability, and letting the user choose is
    // the whole thing being prevented. AddPlayer and SetTrueName are admin actions, so a name they
    // carry is deliberate and kept as sent; an empty one is how an admin asks for a drawn name
    // instead.
    //
    // An exhausted reservoir leaves whatever arrived, and the engine refuses it as a duplicate -- a
    // wrong name is worse than a failed action.
    fn assign_name(&mut self, request: &mut ActionRequest) {
        let unnamed = match &mut request.payload {
            Action::UseAbility(use_ability) => match &mut use_ability.ability_args {
                AbilityBehaviour::TrueNameReroll(reroll) => Some(&mut reroll.true_name),
                _ => None,
            },
            Action::AddPlayer(add) if add.true_name.is_empty() => Some(&mut add.true_name),
            Action::SetTrueName(set) if set.true_name.is_empty() => Some(&mut set.true_name),
            _ => None,
        };
        if let Some(slot) = unnamed
            && let Some(name) = self
                .names
                .draw(|name| self.true_names.values().any(|held| held == name))
        {
            *slot = name;
        }
    }

    // hand the action to the engine, or -- if the pipe died on the write -- reply with a crash and
    // arm the supervisor to replace the child.
    async fn dispatch(&mut self, ticket: Ticket, request: ActionRequest) {
        let state = &self.server_state;
        let game_id = self.game_id;

        let line = to_line(&request);
        if self
            .stdin
            .as_mut()
            .unwrap()
            .write_all(line.as_bytes())
            .await
            .is_err()
        {
            // the pipe died on the write, so this action never ran. same story as a crash; the fd
            // arm will rebuild and resaturate.
            //
            // ask for the kill too. a failed write means the read end is gone, so the child is all
            // but certainly dead already and the supervisor is on its way to replacing it -- but if
            // it somehow is not, nothing else would ever notice: with stdin gone this arm is
            // disabled and the watchdog is not armed, so the game would wedge in silence.
            self.stdin = None;
            let _ = self.kill_in.try_send(());
            let pair = ResponsePair {
                input: ServerInput::Action(request),
                output: ExecOutcome::Action(ActionOutcome::Crashed),
            };
            broadcast(
                state,
                game_id,
                &self.history,
                self.history.head(),
                Some((ticket, pair)),
            );
        } else {
            self.in_flight = Some(InFlight {
                ticket: Some(ticket),
                logged: true,
                request,
            });
        }
    }

    // the current engine cannot be trusted, or is in some way broken. it must be discarded.
    fn discard_engine(&mut self) {
        self.stdin = None;
        self.stdout = None;
        let _ = self.kill_in.try_send(());
    }

    fn autopsy() {}

    fn tap_in() {}

    fn handle_output(
        &mut self,
        ticket: Option<Ticket>,
        request: ActionRequest,
        output: ActionOutcome,
        commands: Vec<CommandPayload>,
    ) {
        let state = &self.server_state;
        let game_id = self.game_id;

        // Read before the commands are handed to the log, which takes them by value.
        // TrueNameUpdate is emitted on every SetTrueName, including a player's first, so this mirror
        // is complete without any special case for creation.
        for payload in &commands {
            if let Command::TrueNameUpdate {
                target_id,
                true_name,
            } = &payload.cmd
            {
                self.true_names.insert(*target_id, true_name.clone());
            }

            if let Command::RevealAutopsyMessages {
                log,
                range,
                redact_names,
            } = &payload.cmd
            {
                let filtered = self
                    .history
                    .filtered_log(*log, |cmd| {
                        let start = self.clock - range;
                        cmd.timestamp >= start
                    })
                    .expect("the log should exist");

                // TODO:
                // if redaction is enabled, send to deepseek v4 flash for in a bunch of
                // parallel chunks, then recombine, and send filtered log dump, else, send raw log.
                // images are a potential issue once those are added. they might contain sensitive
                // info, but that can't easily be redacted. options here:
                // - dont add images/gifs
                // - redact all images/gifs
                // - somehow filter images as well using some kind of ai model
                // - send any images to the host to await approval
            }

            if let Command::TapInResult {
                contact_id,
                outcome,
            } = &payload.cmd
                && let TapInOutcome::Found { log, range } = outcome
            {
                let filtered = self
                    .history
                    .filtered_log(*log, |cmd| {
                        range.is_none_or(|range| {
                            let start = self.clock - range;
                            cmd.timestamp >= start
                        })
                    })
                    .expect("the log should exist");

                // TODO:
                // send log dump
            }
        }

        // A new player slot gets a display name drawn for it, so nobody is ever left rendering as a
        // raw slot. Cosmetic, like a colour in a lobby, and drawn independently of the true name. An
        // admin replaces it with SetProfile.
        //
        // Written before the broadcast below, which is what makes it ride out with the MapActor that
        // entitles a connection to it (see actors_introduced_by). The guard is for a rebuilt game:
        // profiles are server state and outlive the child.
        {
            let mut server_state = lock_state(state);
            if let Some(game) = server_state.games.get_mut(&game_id) {
                for payload in &commands {
                    let Command::MapActor {
                        actor_id: player_id,
                        kind: ActorKind::Player,
                    } = &payload.cmd
                    else {
                        continue;
                    };
                    if game.profiles.contains_key(player_id) {
                        continue;
                    }
                    let taken = |name: &str| {
                        game.profiles
                            .values()
                            .any(|profile| profile.display_name.as_deref() == Some(name))
                    };
                    let Some(display_name) = self.names.draw(taken) else {
                        continue;
                    };
                    game.profiles.insert(
                        *player_id,
                        Profile {
                            display_name: Some(display_name),
                        },
                    );
                }
            }
        }

        let at = self.history.extend(commands);

        // commands go to everyone entitled to them either way; only the reply needs an originating
        // connection, and a server-issued action has none.
        let reply = ticket.map(|ticket| {
            let pair = ResponsePair {
                input: ServerInput::Action(request),
                output: ExecOutcome::Action(output),
            };
            (ticket, pair)
        });
        broadcast(state, game_id, &self.history, at, reply);
    }

    // THIS HAS BEEN VERFIED BY ME, VERIFY THE OTHERS NEXT
    // there is a fairly clean split between game logic extension + delivery, and the messy stuff like
    // fault and process management.
    // it may be wise to create a new delivery/game task more focused on the higher level logic,
    // while this coordinator manages the hard parts.
    // it'd be a handoff to the delivery, and an arm awaiting a response. you would not be able to
    // send input during delivery. this is to prevent unbounded memory growth.
    // the issue im trying to solve here is the one where server level game logic needs watchdog
    // awareness, because delivery too may take a while especially for long running games.
    // we've received a response from the engine.
    async fn handle_response(&mut self, line: Result<Option<String>, std::io::Error>) {
        let text: String;
        match line {
            Ok(Some(txt)) => {
                text = txt;
            }
            // the engine has closed its pipe (EOF), and is still active. the best thing to do here
            // is to discard and restart it.
            Ok(None) => {
                self.discard_engine();
                return;
            }
            Err(e) => match e.kind() {
                ErrorKind::BrokenPipe => {
                    self.discard_engine();
                    return;
                }
                // the operation was interrupted, but the engine is not necessarily broken.
                // we can continue later.
                ErrorKind::Interrupted => {
                    return;
                }
                // the pipe was set to non-blocking mode, and no data was available. in practice,
                // this should not happen.
                ErrorKind::WouldBlock => {
                    return;
                }
                // in most other cases, it makes sense to create a new engine. even if they can
                // technically be recovered from, at worst, a restart will be redundant.
                _ => {
                    self.discard_engine();
                    return;
                }
            },
        }

        // an undeserializable line means the two binaries disagree about the wire format.
        // this is a deploy mistake, not a runtime failure.
        let result: ExecutionResult = match serde_json::from_str(&text) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("engine output failed to deserialize: {e} -- aborting");
                std::process::abort()
            }
        };

        // this was a response from a resaturation input.
        if self.to_discard > 0 {
            self.to_discard -= 1;
            // the watchdog measures SILENCE, not how long a computation takes.
            // without this, a long log would trip it and get a healthy child killed, this
            // is an endless crash loop.
            self.deadline =
                (self.to_discard > 0).then(|| Instant::now() + Duration::from_secs(ENGINE_TIMEOUT));
            return;
        }

        let Some(InFlight {
            ticket,
            logged,
            request,
        }) = self.in_flight.take()
        else {
            // the engine said something when there was nothing to respond to. this is not
            // recoverable, because we can't know what caused it. restarting it would just make it
            // happen again. aborting would ruin every other game on the server.
            // TODO: log this loudly (which engine, what it said) and carry on.
            return;
        };

        // jumping to some point in time internally pops jobs off of the engine's scheduler, if any, so
        // on reboot, we need to jump to the last reached time beyond just replaying the logged actions.
        self.clock = self.clock.max(request.timestamp);

        let (output, commands) = match result {
            Ok((response, context)) => {
                // not all actions are logged (for instance, null ticks).
                if logged {
                    self.accepted.push(request.clone());
                }
                (ActionOutcome::Ok(response), context.commands)
            }
            // a rejected action may still carry a batch of commands
            Err((error, context)) => (ActionOutcome::Err(error), context.commands),
        };

        self.handle_output(ticket, request, output, commands);
    }

    // THIS HAS BEEN VERIFIED BY ME. VERIFY THE OTHERS NEXT.
    //
    // drive time forward. the engine only advances when an action arrives, so without this nothing
    // time-based ever happens: a scheduled kill, a poll timeout, a prosecution phase or a release
    // would all sit in the job queue until some player happened to act. this is what makes them land
    // on the clock instead.
    //
    // a Null action does nothing itself. it exists only as an update and time advancement mechanism.
    // execute() runs the job queue for everything up to its timestamp before the action, and
    // ActionExt::execute runs Update (polls, prosecutions, deferred-command flush) after every action regardless.
    // an empty action collects all of it. sending Update explicitly would just run that sweep twice.
    async fn tick(&mut self) {
        let request = ActionRequest {
            actor: ActionActor::System,
            timestamp: now(),
            payload: Action::Null(Null {}),
        };

        // this arm may only run when stdin exists, so the unwrap is safe.
        // the next await point is on the write.
        // there is no period before the unwrap where stdin can become None.
        let line = to_line(&request);
        if self
            .stdin
            .as_mut()
            .unwrap()
            .write_all(line.as_bytes())
            .await
            .is_err()
        {
            // either the engine has closed its pipe, or it has crashed.
            self.stdin = None;
            // we trigger an explicit kill on the supervisor's end just in case the engine closed
            // its pipe for some reason. in the case of a crash, this is discarded.
            let _ = self.kill_in.try_send(());
        } else {
            // TODO:
            // expand inflight to ALL server inputs rather than just action requests
            self.in_flight = Some(InFlight {
                ticket: None,
                logged: false,
                request,
            });
        }
    }

    // THIS FUNCTION HAS BEEN VERIFIED BY ME. CONTINUE VERIFYING THE OTHER ARMS.
    //
    // treat hangs as crashes. a hang is just a silent crash, and without being handled, it could
    // actually be more dangerous than a real crash.
    fn on_hang(&mut self) {
        let state = &self.server_state;
        let game_id = self.game_id;

        // if we didnt handle this here, the watchdog may fire again before the boot handler is called,
        // causing an endless crash cycle.
        if let Some(InFlight {
            ticket, request, ..
        }) = self.in_flight.take()
        {
            // TODO:
            // delivery rework
            deliver_crash(state, game_id, ticket, request);
        }

        // abandon the pipes: a late line from a child we have given up on must not be mistaken for a
        // live response, and nothing more may be written to it.
        self.stdin = None;
        self.stdout = None;
        self.to_discard = 0;

        let _ = self.kill_in.try_send(());
    }

    // the coordinator is an asynchronous state machine. it takes a clone of the cancel token so the
    // outer task still holds one to clean up with after aborting.
    async fn run(&mut self) {
        loop {
            tokio::select! {
                // engine boot signal
                Some(fds) = self.fd_out.recv() => {
                    self.handle_boot(fds).await;
                }

                // we can only process one input at a time. we must know which input is in flight so
                // we can guarantee a response while the server is active.
                // we cannot take input if there is no engine available. most input is sent directly to the engine.
                // even there are some inputs which dont technically rely on an active engine, it's
                // safer and cleaner to just wait.
                // we cannot take input during a reboot period, because if we did, we could have
                // player input interleaved with rehydration inputs, which would offset the discard mechanism,
                // and would invalidate the engine's state as a player input would be inserted into
                // the existing timeline rather than be appended.
                Some(event) = self.events.recv(), if self.stdin.is_some() && self.in_flight.is_none() && self.to_discard == 0 => {
                    self.handle_input(event).await
                }

                // we've received a response from the engine process.
                // it is not possible for stdout to be None within this async block, because the
                // block only runs if stdout is confirmed to be Some(_).
                line = async { self.stdout.as_mut().unwrap().next_line().await }, if self.stdout.is_some() => {
                    self.handle_response(line).await
                }

                // drive time forward if there is an active engine, we are not processing an input,
                // and we are not rehydrating.
                _ = self.tick.tick(), if self.stdin.is_some() && self.in_flight.is_none() && self.to_discard == 0 => {
                    self.tick().await
                }

                // watchdog timer.
                // the engine owes us an output and has not produced one. treat it exactly as a crash.
                // the only difference is that a hang never announces itself, so it must be caught
                // externally.
                _ = async { sleep_until(self.deadline.unwrap()).await }, if self.deadline.is_some() => {
                    self.on_hang();
                }

                // every branch disabled: no pipe, and no supervisor left to hand us a new one. select!
                // PANICS on that rather than parking, and a panic here dies inside a spawned task, so
                // exit deliberately instead -- nothing can make progress again either way.
                else => break,
            };

            // the watchdog is armed when there is no watchdog timer, and we are owed something.
            // either the engine has an input in flight, or we are in a rehydration period.
            // an existing watchdog is left alone. re-arming it would indefinitely extend the watchdog timer, making it useless.
            self.deadline = match (
                self.in_flight.is_some() || self.to_discard > 0,
                self.deadline,
            ) {
                (false, _) => None,
                (true, Some(running)) => Some(running),
                (true, None) => Some(Instant::now() + Duration::from_secs(ENGINE_TIMEOUT)),
            };
        }
    }
}

// permission enforcement,
// input executions,
// live client updates,
// and engine process management
pub async fn game(
    state: WrappedServerState,
    game_id: GameId,
    events: mpsc::UnboundedReceiver<GameEvent>,
    cancel: CancellationToken,
) {
    // the supervisor hands over each fresh child's pipes. unbounded rather than a size-1 channel
    // because the coordinator has to be able to TAKE ownership of the pair (the pipe halves aren't
    // Clone, so nothing that only lends a borrow works here); the coordinator drains to the newest
    // pair on wake, which is what keeps a crash-loop from feeding it a dead child's descriptors.
    let (fd_in, fd_out) = mpsc::unbounded_channel::<(ChildStdin, ChildStdout)>();

    // the coordinator's only way to reach the child: it holds the pipes, but the supervisor owns the
    // process. a hung engine cannot be dislodged by closing stdin -- it is not reading -- so killing
    // it has to happen over here.
    //
    // capacity 1 because the request carries no information: one pending kill says everything two
    // would, so the coordinator can try_send and drop the duplicate.
    let (kill_in, mut kill_out) = mpsc::channel::<()>(1);

    let mut process_supervisor = tokio::spawn(async move {
        loop {
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

            // a kill request queued against the child we just replaced would otherwise be consumed
            // below and shoot down this brand new one -- a self-sustaining crash loop. at capacity 1
            // there is at most one to throw away.
            let _ = kill_out.try_recv();

            if fd_in
                .send((child.stdin.take().unwrap(), child.stdout.take().unwrap()))
                .is_err()
            {
                break;
            }

            select! {
                _ = child.wait() => {} // died on its own
                // the coordinator declared it hung. a hang is a crash that just doesn't announce
                // itself, so the response is the same: replace the child and rebuild.
                Some(()) = kill_out.recv() => {
                    let _ = child.kill().await;
                }
            }
        }
    });

    // only one action can be processed at a time. this means that the two tasks (read + write) are
    // coupled by nature and should not be run in parallel. you cannot execute multiple things at a
    // time. however, multiple inputs may be waiting in the game handle's queue at any given time.
    //
    // if receiving a new set of file descriptors while an input is in flight, and that input is an
    // engine level mechanism, it means that the input most likely triggered a crash, and so they
    // should receive an engine crashed response. if they didnt trigger a crash, it was likely some
    // kind of freak incident out of our control.
    //
    // only after the action is confirmed to be valid, and the action has been saved does the client
    // receive acknowledgement.
    //
    // commands live in ONE log in emission order; a recipient's "own log" is a filter over it. that
    // is what keeps cross-recipient order intact, so a replay can never hand out an Actor command
    // before the BasePlayer command that created what it refers to.
    //
    // sequence numbers are not saved for any given key. they are determined at runtime.
    // how it works:
    // every connection gets its own sequence number starting at 0 at runtime
    // on attach, walk the log once, keep what that connection may see, and send it as one batch --
    // which grants every actor available to that connection immediately.
    //
    // a batch consists of a command buffer and an optional structure containing both the requested
    // input and its response (if applicable).
    //
    // responses are not saved to command sequences. responses are meant only for specific
    // connections.
    //
    // batches are sent on a best effort basis. if a client's outbox is full, the client is cut. the
    // alternatives are potentially unboundedly growing memory, or having the client lose mandatory
    // info.
    let coordinator_cancel = cancel.clone();
    let coordinator_state = state.clone();
    let mut coordinator = tokio::spawn(async move {
        let mut coordinator = Coordinator::new(
            game_id,
            coordinator_state,
            coordinator_cancel,
            events,
            fd_out,
            kill_in,
        );
        coordinator.run().await
    });

    // &mut so the handles survive the race; whichever arm wins, abort the others. aborting the
    // supervisor drops the Child it owns, and kill_on_drop reaps the engine -- so the process goes
    // even when teardown came from outside and nobody asked it to stop.
    select! {
        _ = cancel.cancelled() => {}
        _ = &mut process_supervisor => {}
        _ = &mut coordinator => {}
    }
    process_supervisor.abort();
    coordinator.abort();

    // cleanup. cancel first: this is the only path that runs it, and one of the two ways in here is a
    // task ending on its own, where nothing has cancelled anything yet. dropping a CancellationToken
    // does NOT cancel it, so removing the handle before this would strand every live socket waiting
    // on a token that will never fire, until its heartbeat eventually reaped it.
    cancel.cancel();

    // dropping the handle closes the inbox, so any connection task still on its way out finds its
    // send failing. their ClaimGuards already tolerate the game being gone.
    lock_state(&state).games.remove(&game_id);
}
