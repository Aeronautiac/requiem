// One game's coordinator task.
//
// It owns the engine child process, the single global command log, and the loop that turns
// inputs into executions. Everything a game knows lives here for the duration of the process:
// the log is the source of truth, and a crashed child is rebuilt by replaying it.
//
// One channel in, not two, so ordering is free -- a connection's Attach is queued ahead of
// anything it goes on to send, and is therefore always replayed before it can act.

// things like profiles and keys are inherently tied to the live state of a game.
// say for instance you were to create a key with access to an actor created later into a game,
// and then you were to rewind time, then create a new, different actor which happened to share that same
// actor id because it took up that free slot at a different point in time.
// this is essentially a temporal ABA problem.
// you cannot reconcile in this case, because you cannot confirm on the server level whether or not
// that is the same actor that you previously gave that key access to, nor can you throw out the data,
// because that'd require a wipe of every key's actor scope, potentially requiring the admin to
// manually modify every existing key.
//
// websocket server inputs directly modify a live game, just as engine inputs directly modify the
// engine.
//
// the proposed solution is to treat a "game" as a direct extension of the engine, and to replay not only
// engine inputs, but also server inputs on the websocket layer.
//
// then, the database storage format, networking, and server boot handling all remain exceptionally
// simple.
//
// the key invalidation scenario is resolved.
// with the new model:
// a rewind to some point in time removes any key from existence which was created for some state
// that no longer exists.
//
// there is more than one layer to a game.
// there is a harness (the raw server messiness manager), a shell (the server's extension of the engine,
// the game's data), and a core (the engine process itself)
//
// a rewind is not part of the shell layer. it is part of the harness layer.
// a key creation is part of the shell layer.
// a profile creation is part of the shell layer.
// an action request is part of the engine layer.
//
// furthermore, i've realized that a harness doesn't even need to run while nobody is connected to it,
// and given this new model, we can instantly reconstruct the server's state for that game as well
// when someone connects. all we need to do is store a cache of game keys for every game in the
// database, so connections can be handled via REST. this saves a large chunk of memory and compute.
//
// given an input, a game shell produces an output.

// redesign idea:
// The harness manages the shell, and acts as a bridge between the shell, the database, and the network.
// It takes input from connections, matches on the type of input, and either routes it to the shell,
// or performs some higher level action ABOVE the shell, i.e., discarding the old shell and
// resaturating for time manipulation.
//
// It also acts as the first permissions gate. By reading the key state (outputted as responses to
// key related inputs), it determines if the connection is allowed to do something that typically
// requires administrator privileges. The shell only manages its internal state. It does not care
// about permissions enforcement outside of what the engine says. Note that this kind of thing being
// an input with a response trivially solves the issue of clients not being able to see current key
// state, because keys become a meta-engine level construct, and the key state is delivered just
// like everything else. The shell doesn't care about a database. That is the job of the harness.
// All the shell does is focus entirely on managing itself.
//
// After sending something to the shell, it awaits a response, and if the response was not
// a crash or engine rejection, it saves the valid input to the database. It then delivers the
// shell's output to every connection that is entitled to the information it returned.
//
// The shell manages the engine process. It is a server level engine wrapper.
// It does not tick on its own. It is similar to the engine in the sense that it takes one input, and
// always sends out exactly one output.
//
// The reason we aren't creating our own runtime here as the game shell that simply wraps around an
// engine process is because we already have a generic engine runtime, and the stuff in the shell is
// specific to only this server. Either would work. It's just nicer to keep all the server data in
// one program. A shell's handle here is not a process. It is a tokio task.

use std::{collections::HashMap, env::current_exe, process::Stdio, time::Duration};

use lawliet_types::{
    ability::AbilityBehaviour,
    action::{Action, ActionActor, ActionRequest, InitializeEngine, Null},
    actor::ActorKind,
    command::{Command, TapInOutcome},
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
    auth::{Privileges, Ticket},
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

// remember, a game shell can take in

struct GameHarness {
    // identity / routing
    game_id: GameId,
    server_state: WrappedServerState,
    cancel: CancellationToken,
    // coordinator handle(s)
    // tokio task(s)
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

// what a select arm hands back to the loop: `Continue` skips the watchdog re-arm at the bottom
// (used by arms that `continue`d in the original, and by the discard arm which restarts the window
// itself), `Proceed` runs it.
enum Flow {
    Continue,
    Proceed,
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

    // crash or initial boot -- resaturate the new child with everything it accepted before.
    async fn handle_boot(&mut self, fds: (ChildStdin, ChildStdout)) {
        let state = &self.server_state;
        let game_id = self.game_id;

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
            deliver_crash(state, game_id, ticket, request);
        }

        self.stdin = Some(new_in);
        self.stdout = Some(BufReader::new(new_out).lines());

        // count only what actually made it down the pipe: if the write fails partway, the remaining
        // actions never ran, so their echoes will never arrive to be discarded.
        let mut written = 0;
        for request in &self.accepted {
            let line = to_line(request);
            if self
                .stdin
                .as_mut()
                .unwrap()
                .write_all(line.as_bytes())
                .await
                .is_err()
            {
                self.stdin = None;
                break;
            }
            written += 1;
        }

        // Bring the fresh child up to the clock the old one had reached. Without this every job
        // executed since the last ACCEPTED action is still queued here, and the next tick would emit
        // its commands a second time. See `clock`.
        //
        // Collapsing however many ticks into one is safe: execute() pops jobs individually in
        // timestamp order either way, so the same jobs run in the same order -- only their batching
        // into contexts differs, and every one of these contexts is discarded below.
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
                self.stdin = None;
            } else {
                written += 1;
            }
        }

        self.to_discard = written;

        // a game is initialized by the server, not by whoever connects first: an uninitialized
        // engine rejects everything, so leaving it to an admin's first action would mean every game
        // has a broken window before someone opens it.
        //
        // issued only when nothing has been accepted yet, which is exactly the first boot. once it
        // succeeds it lives in `accepted` like any other action, so every subsequent rebuild replays
        // it -- with its original seed, keeping the engine's RNG stream identical across a crash.
        if self.accepted.is_empty() && self.stdin.is_some() {
            let request = ActionRequest {
                // the server's own voice. no key can ask for this (System is unreachable from a
                // connection) and no connection is waiting on it.
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
                self.stdin = None;
            } else {
                self.in_flight = Some(InFlight {
                    ticket: None,
                    logged: true,
                    request,
                });
            }
        }
    }

    // a connection attaching, or an input from one. gated on the engine being reachable with nothing
    // in flight, so the channel itself is the queue -- and because it is ONE channel, a connection's
    // Attach is always handled before anything it goes on to send.
    // a connection attaching, or an input from one. gated on the engine being reachable with nothing
    // in flight, so the channel itself is the queue -- and because it is ONE channel, a connection's
    // Attach is always handled before anything it goes on to send.
    async fn handle_input(&mut self, event: GameEvent) -> Flow {
        let state = &self.server_state;
        let game_id = self.game_id;

        let InputEnvelope { ticket, input } = match event {
            GameEvent::Attach { ticket } => {
                deliver_catchup(state, game_id, &self.history, &ticket);
                return Flow::Continue;
            }
            GameEvent::Widen { ticket, before } => {
                deliver_widening(state, game_id, &self.history, &ticket, &before);
                return Flow::Continue;
            }
            GameEvent::Input(envelope) => envelope,
        };

        match input {
            // controls are handled HERE and never forwarded: they act ON the timeline, not in the
            // fiction, and the engine has no concept of them.
            ServerInput::Control(control) => {
                self.handle_control_input(ticket, control);
                Flow::Continue
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
    async fn handle_action(&mut self, ticket: Ticket, mut request: ActionRequest) -> Flow {
        // auth: a connection may only act as an actor its key's privilege set permits. checked here
        // rather than at the socket because the privilege set is resolved fresh every time, so a
        // narrowed key takes effect on its live sockets at once.
        if !self.authorize(&ticket, &request) {
            self.deny(ticket, request);
            return Flow::Continue;
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
    async fn dispatch(&mut self, ticket: Ticket, request: ActionRequest) -> Flow {
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

        Flow::Proceed
    }

    fn autopsy() {}

    fn tap_in() {}

    // a response from the child.
    async fn handle_response(&mut self, line: Result<Option<String>, std::io::Error>) -> Flow {
        let state = &self.server_state;
        let game_id = self.game_id;

        let Ok(Some(text)) = line else {
            self.stdout = None;
            return Flow::Continue;
        };

        // the runtime is the other half of this protocol. an undeserializable line
        // means the two binaries disagree about the wire format -- a deploy mistake, not a runtime
        // condition, and not something to limp along with.
        let result: ExecutionResult = match serde_json::from_str(&text) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("engine output failed to deserialize: {e} -- aborting");
                std::process::abort()
            }
        };

        if self.to_discard > 0 {
            self.to_discard -= 1; // resaturation echo; its commands are already in `log`
            // the watchdog measures SILENCE, not how long the whole replay takes -- a long enough log
            // would otherwise trip it and get a perfectly healthy child killed, then do it again on
            // every rebuild. an echo is proof of progress, so the window restarts (and closes once
            // nothing more is owed).
            self.deadline =
                (self.to_discard > 0).then(|| Instant::now() + Duration::from_secs(ENGINE_TIMEOUT));
            return Flow::Continue;
        }

        let Some(InFlight {
            ticket,
            logged,
            request,
        }) = self.in_flight.take()
        else {
            // the engine spoke with nothing owed. there is no good response available: aborting
            // punishes every other game on the box for one child's weirdness, and rebooting would
            // just reproduce whatever caused it.
            // TODO: log this loudly (which engine, what it said) and carry on.
            return Flow::Continue;
        };

        // The engine ran the job queue up to this stamp, whether or not the action itself was
        // accepted, so this is what a rebuild has to reach. Recorded for BOTH arms below -- a
        // rejected action and a null tick each advance the engine and neither goes into `accepted`.
        self.clock = self.clock.max(request.timestamp);

        let (output, commands) = match result {
            Ok((response, context)) => {
                // a null tick is not logged: it carries no intent of its own, only a clock, and
                // `clock` above is how that clock survives a rebuild. its COMMANDS are logged and
                // fanned out like anything else -- what the catchup actually did is real state, and a
                // client reconnecting must still see it.
                if logged {
                    self.accepted.push(request.clone());
                }
                (ActionOutcome::Ok(response), context.commands)
            }
            // a rejected action changed nothing of its own, so it is not replayed. it can still carry
            // catchup commands: the job queue runs on the way in and its effects are real regardless
            // of what the requested action did. those jobs are not lost on a rebuild either -- `clock`
            // covers them.
            Err((error, context)) => (ActionOutcome::Err(error), context.commands),
        };

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

        Flow::Proceed
    }

    // drive time forward. the engine only advances when an action arrives, so without this nothing
    // time-based ever happens: a scheduled kill, a poll timeout, a prosecution phase or a release
    // would all sit in the job queue until some player happened to act. this is what makes them land
    // on the clock instead.
    //
    // a Null action is the right instrument precisely because it does nothing itself. execute() runs
    // the job queue for everything up to its timestamp before the action, and ActionExt::execute
    // runs Update (polls, prosecutions, deferred-command flush) after every action regardless -- so
    // an empty action collects all of it. sending Update explicitly would just run that sweep twice.
    async fn tick(&mut self) -> Flow {
        let state = &self.server_state;
        let game_id = self.game_id;

        // nobody is watching, so there is nothing to deliver. skipping keeps an idle game from waking
        // its engine every few seconds forever; the jobs are not lost, they run on the catchup of
        // whichever action or tick comes next.
        let watched = {
            let server_state = lock_state(state);
            server_state
                .games
                .get(&game_id)
                .is_some_and(|game| game.connections.values().any(|conn| !conn.dropped))
        };
        if !watched {
            return Flow::Continue;
        }

        let request = ActionRequest {
            actor: ActionActor::System,
            timestamp: now(),
            payload: Action::Null(Null {}),
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
            self.stdin = None;
            let _ = self.kill_in.try_send(());
        } else {
            self.in_flight = Some(InFlight {
                ticket: None,
                logged: false,
                request,
            });
        }

        Flow::Proceed
    }

    // the engine owes us a line and has not produced one. treat it exactly as a crash -- the only
    // difference is that a hang never announces itself, so we have to notice.
    fn on_hang(&mut self) {
        let state = &self.server_state;
        let game_id = self.game_id;

        // reply here rather than leaving it to the fd arm: clearing in_flight now is what stops the
        // watchdog re-arming and firing a second kill before the child is gone.
        if let Some(InFlight {
            ticket, request, ..
        }) = self.in_flight.take()
        {
            deliver_crash(state, game_id, ticket, request);
        }

        // abandon the pipes: a late line from a child we have given up on must not be mistaken for a
        // live response, and nothing more may be written to it.
        self.stdin = None;
        self.stdout = None;
        self.to_discard = 0;

        // full means a kill is already pending, which says everything this one would.
        let _ = self.kill_in.try_send(());
    }

    // the coordinator is an asynchronous state machine. it takes a clone of the cancel token so the
    // outer task still holds one to clean up with after aborting.
    async fn run(&mut self) {
        loop {
            let flow = tokio::select! {
                // crash or initial boot -- resaturate the new child with everything it accepted before
                Some(fds) = self.fd_out.recv() => {
                    self.handle_boot(fds).await;
                    Flow::Proceed
                }

                // a connection attaching, or an input from one. gated on the engine being reachable
                // with nothing in flight, so the channel itself is the queue -- and because it is ONE
                // channel, a connection's Attach is always handled before anything it goes on to send.
                Some(event) = self.events.recv(), if self.stdin.is_some() && self.in_flight.is_none() => {
                    self.handle_input(event).await
                }

                // response from child
                line = async { self.stdout.as_mut().unwrap().next_line().await }, if self.stdout.is_some() => {
                    self.handle_response(line).await
                }

                // drive time forward.
                _ = self.tick.tick(), if self.stdin.is_some() && self.in_flight.is_none() => {
                    self.tick().await
                }

                // the engine owes us a line and has not produced one. treat it exactly as a crash --
                // the only difference is that a hang never announces itself, so we have to notice.
                _ = async { sleep_until(self.deadline.unwrap()).await }, if self.deadline.is_some() => {
                    self.on_hang();
                    Flow::Proceed
                }

                // every branch disabled: no pipe, and no supervisor left to hand us a new one. select!
                // PANICS on that rather than parking, and a panic here dies inside a spawned task, so
                // exit deliberately instead -- nothing can make progress again either way.
                else => break,
            };

            if matches!(flow, Flow::Continue) {
                continue;
            }

            // keep the watchdog honest: armed exactly while the engine owes us something -- a reply to
            // an in-flight action, or an outstanding replay echo. arms that `continue` skip this and
            // are responsible for their own bookkeeping (the echo branch restarts the window itself,
            // since receiving a line is progress).
            //
            // an already-running deadline is deliberately left alone. re-deriving it on every wakeup
            // would push it forward whether or not the engine did anything, which is the classic way a
            // watchdog quietly stops being one. (`sleep_until` takes an absolute instant, so rebuilding
            // the future each pass is free of that hazard -- `sleep(duration)` would restart the clock.)
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
