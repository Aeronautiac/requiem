// One game's coordinator task.
//
// It owns the engine child process, the single global command log, and the loop that turns
// inputs into executions. Everything a game knows lives here for the duration of the process:
// the log is the source of truth, and a crashed child is rebuilt by replaying it.
//
// One channel in, not two, so ordering is free -- a connection's Attach is queued ahead of
// anything it goes on to send, and is therefore always replayed before it can act.

use std::{collections::HashMap, env::current_exe, process::Stdio, time::Duration};

use lawliet_types::{
    ability::AbilityBehaviour,
    action::{Action, ActionActor, ActionRequest, InitializeEngine, Null},
    command::Command,
    common::{ActorKey, Time},
    engine::ExecutionResult,
};
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{ChildStdin, ChildStdout},
    select,
    sync::mpsc::{self},
    time::{Instant, MissedTickBehavior, interval, sleep_until},
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
    wire::{ActionOutcome, ExecOutcome, Profile, ResponsePair, ServerInput},
};

// carries the source ticket so the game task can route replies and enforce permissions.
pub struct InputEnvelope {
    pub ticket: Ticket,
    pub input: ServerInput,
}

// the action the engine is currently working on.
pub struct InFlight {
    // who is waiting on it. `None` for actions the server issues on its own behalf -- engine
    // initialization, null ticks -- which have no originating connection to reply to. their commands
    // are still logged and fanned out like any other; only the reply has nowhere to go.
    ticket: Option<Ticket>,
    // whether to append this to the action log on success, and so replay it into a rebuilt child.
    // false for null ticks: a tick carries no intent of its own, it only asks the engine to catch up
    // to the clock, so a rebuilt child reaches the same state from the real actions alone. logging
    // them would grow the log without bound for a game where nothing happens.
    logged: bool,
    request: ActionRequest,
}

// everything the game task hears about. one channel, not two, so ordering is free: a connection's
// Attach is queued before any input it goes on to send, so it is always replayed before it can act.
pub enum GameEvent {
    // a websocket finished upgrading and wants its catch-up replay.
    Attach { ticket: Ticket },
    // this connection's key was widened and is owed the history it could not see before. carries
    // the PREVIOUS privilege set because the delivery is the difference between the two, and the
    // ledger already holds the new one by the time this is handled.
    //
    // narrowing has no event: it needs no log, so it is applied in place under the control's own
    // lock (see control::apply_privilege_change).
    Widen { ticket: Ticket, before: Privileges },
    Input(InputEnvelope),
}

// serialize something of ours for the wire. a failure is a bug in this process, not a runtime
// condition, so abort loudly rather than paper over a half-written protocol.
pub fn to_line<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(json) => json + "\n",
        Err(e) => {
            eprintln!("failed to serialize for the engine pipe: {e} -- aborting");
            std::process::abort()
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
    mut events: mpsc::UnboundedReceiver<GameEvent>,
    cancel: CancellationToken,
) {
    // the supervisor hands over each fresh child's pipes. unbounded rather than a size-1 channel
    // because the coordinator has to be able to TAKE ownership of the pair (the pipe halves aren't
    // Clone, so nothing that only lends a borrow works here); the coordinator drains to the newest
    // pair on wake, which is what keeps a crash-loop from feeding it a dead child's descriptors.
    let (fd_in, mut fd_out) = mpsc::unbounded_channel::<(ChildStdin, ChildStdout)>();

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

    // only one action can be processed at a time. this means that the two tasks (read + write)
    // are coupled by nature and should not be run in parallel. you cannot execute multiple things
    // at a time.
    // however, multiple inputs may be waiting in the game handle's queue at any given time.
    //
    // if receiving a new set of file descriptors while an input is in flight, and that input is an
    // engine level mechanism, it means that the input most likely triggered a crash, and so they should receive
    // an engine crashed response. if they didnt trigger a crash, it was likely some kind of freak
    // incident out of our control.
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
    // responses are not saved to command sequences. responses are meant only for specific connections.
    //
    // batches are sent on a best effort basis. if a client's outbox is full, the client is cut. the
    // alternatives are potentially unboundedly growing memory, or having the client lose mandatory
    // info.
    //
    // the coordinator is an asynchronous state machine
    // the coordinator takes a clone so the outer task still holds one to clean up with after aborting.
    let coordinator_cancel = cancel.clone();
    let coordinator_state = state.clone();
    let mut coordinator = tokio::spawn(async move {
        let state = coordinator_state;
        // by the time something is in flight it is necessarily an action -- controls never reach the
        // engine.
        let mut in_flight: Option<InFlight> = None;
        let mut stdin: Option<ChildStdin> = None;
        let mut stdout: Option<Lines<BufReader<ChildStdout>>> = None;

        // every action the engine accepted, in order. the source of truth for rebuilding a fresh
        // child, and what a persistent log would store.
        let mut accepted: Vec<ActionRequest> = vec![];
        // every command ever emitted, in emission order, plus its viewport index. "per-recipient
        // logs" are a FILTER over this, not separate storage -- one log is what keeps cross-recipient
        // order intact for free.
        let mut history = History::default();
        // replay responses still to be swallowed. a rebuilt child re-emits every command it already
        // emitted; `history` already holds them, so the echoes must not be logged or fanned out again.
        let mut to_discard: usize = 0;
        // the highest timestamp the engine has actually executed at.
        //
        // The engine's progress through its job queue is driven entirely by the timestamps of the
        // actions it receives -- execute() pops every job at or before the action's stamp. So
        // `accepted` alone does NOT describe how far the engine got: a null tick and a rejected
        // action both run the queue and have their commands logged, and neither is replayable.
        // Rebuilding from `accepted` leaves those jobs sitting in the fresh child's queue, and the
        // next tick pops them a second time -- re-emitting commands that are already in `history`,
        // after the discard count is spent, so they are appended and fanned out for real. That is
        // duplicated deaths and world events, permanently in the log, on every reconnect.
        //
        // Replaying one Null at this timestamp is what closes it: it says "the engine had reached
        // here", which is the fact `accepted` cannot carry.
        let mut clock: Time = 0;
        // watchdog: when the engine owes us a line, when we stop waiting for it. armed and disarmed
        // in one place, at the bottom of the loop.
        let mut deadline: Option<Instant> = None;
        // drawn once, here, rather than per boot: it rides in the InitializeEngine action, so a
        // rebuilt child replaying that action reproduces the same RNG stream. a fresh seed per boot
        // would make every crash silently fork the game's randomness.
        let seed = generate_seed();

        // Every player's current true name, mirrored from what the engine announced. The engine
        // stays the authority; this exists only so a reroll can be handed a name nobody holds,
        // which is a question the coordinator cannot ask the engine directly -- it lives in
        // another process.
        let names = NamePool::new();
        let mut true_names: HashMap<ActorKey, String> = HashMap::new();

        // Delay rather than the default Burst: if the coordinator was busy through several tick
        // periods, firing all of them back-to-back is pure waste. a tick catches the engine up to
        // NOW, so one late tick does everything the missed ones would have.
        let mut tick = interval(Duration::from_secs(NULL_TICK_INTERVAL));
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                // crash or initial boot — resaturate the new child with everything it accepted before
                Some(fds) = fd_out.recv() => {
                    // drain to the newest pair. a crash-loop can queue several, and every pair but
                    // the last belongs to a child that is already dead -- dropping them closes those
                    // pipes, which is exactly what we want.
                    let mut fds = fds;
                    while let Ok(newer) = fd_out.try_recv() {
                        fds = newer;
                    }
                    let (new_in, new_out) = fds;

                    // an action that was in flight when the pipe died is the prime suspect for having
                    // killed it, so it is NOT added to `accepted` and never replayed. its originator
                    // is told, rather than left waiting on a reply that will never come.
                    if let Some(InFlight { ticket, request, .. }) = in_flight.take() {
                        deliver_crash(&state, game_id, ticket, request);
                    }

                    stdin = Some(new_in);
                    stdout = Some(BufReader::new(new_out).lines());

                    // count only what actually made it down the pipe: if the write fails partway, the
                    // remaining actions never ran, so their echoes will never arrive to be discarded.
                    let mut written = 0;
                    for request in &accepted {
                        let line = to_line(request);
                        if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                            stdin = None;
                            break;
                        }
                        written += 1;
                    }

                    // Bring the fresh child up to the clock the old one had reached. Without this
                    // every job executed since the last ACCEPTED action is still queued here, and
                    // the next tick would emit its commands a second time. See `clock`.
                    //
                    // Collapsing however many ticks into one is safe: execute() pops jobs
                    // individually in timestamp order either way, so the same jobs run in the same
                    // order -- only their batching into contexts differs, and every one of these
                    // contexts is discarded below.
                    if clock > 0 && stdin.is_some() {
                        let line = to_line(&ActionRequest {
                            actor: ActionActor::System,
                            timestamp: clock,
                            payload: Action::Null(Null {}),
                        });
                        if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                            stdin = None;
                        } else {
                            written += 1;
                        }
                    }

                    to_discard = written;

                    // a game is initialized by the server, not by whoever connects first: an
                    // uninitialized engine rejects everything, so leaving it to an admin's first
                    // action would mean every game has a broken window before someone opens it.
                    //
                    // issued only when nothing has been accepted yet, which is exactly the first boot.
                    // once it succeeds it lives in `accepted` like any other action, so every
                    // subsequent rebuild replays it -- with its original seed, keeping the engine's RNG
                    // stream identical across a crash.
                    if accepted.is_empty() && stdin.is_some() {
                        let request = ActionRequest {
                            // the server's own voice. no key can ask for this (System is unreachable
                            // from a connection) and no connection is waiting on it.
                            actor: ActionActor::System,
                            timestamp: now(),
                            payload: Action::InitializeEngine(InitializeEngine { seed }),
                        };

                        let line = to_line(&request);
                        if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                            stdin = None;
                        } else {
                            in_flight = Some(InFlight { ticket: None, logged: true, request });
                        }
                    }
                }

                // a connection attaching, or an input from one. gated on the engine being reachable
                // with nothing in flight, so the channel itself is the queue -- and because it is ONE
                // channel, a connection's Attach is always handled before anything it goes on to send.
                Some(event) = events.recv(), if stdin.is_some() && in_flight.is_none() => {
                    let InputEnvelope { ticket, input } = match event {
                        GameEvent::Attach { ticket } => {
                            deliver_catchup(&state, game_id, &history, &ticket);
                            continue;
                        }
                        GameEvent::Widen { ticket, before } => {
                            deliver_widening(&state, game_id, &history, &ticket, &before);
                            continue;
                        }
                        GameEvent::Input(envelope) => envelope,
                    };

                    // controls are handled HERE and never forwarded: they act ON the timeline, not in
                    // the fiction, and the engine has no concept of them.
                    let mut request = match input {
                        ServerInput::Action(request) => request,
                        ServerInput::Control(control) => {
                            // controls carry no actor, so authority is a matter of capabilities and
                            // of the target key -- all of which lives in handle_control.
                            let outcome = handle_control(
                                &state,
                                game_id,
                                &ticket,
                                &control,
                                &coordinator_cancel,
                            );

                            // replied to even for EndGame, where the teardown races the send and will
                            // usually win. uniform beats special-casing a reply nobody is waiting on.
                            let pair = ResponsePair {
                                input: ServerInput::Control(control),
                                output: ExecOutcome::Control(outcome),
                            };
                            broadcast(&state, game_id, &history, history.head(), Some((ticket, pair)));
                            continue;
                        }
                    };

                    // auth: a connection may only act as an actor its key's privilege set permits.
                    // checked here rather than at the socket because the privilege set is resolved
                    // fresh every time, so a narrowed key takes effect on its live sockets at once.
                    let permitted = {
                        let server_state = lock_state(&state);
                        server_state
                            .games
                            .get(&game_id)
                            .and_then(|game| game.privileges(&ticket))
                            .is_some_and(|privileges| privileges.can_act_as(&request.actor))
                    };

                    if !permitted {
                        let pair = ResponsePair { input: ServerInput::Action(request), output: ExecOutcome::Action(ActionOutcome::Denied) };
                        broadcast(&state, game_id, &history, history.head(), Some((ticket, pair)));
                        continue;
                    }

                    // The server names people, not their clients: a true name a player picked for
                    // itself would be worth nothing to anybody. Same treatment as the timestamp,
                    // but done here rather than at the socket because only the coordinator knows
                    // which names are already in play.
                    //
                    // A reroll is ALWAYS replaced -- it is a player's own ability, and letting the
                    // user choose is the whole thing being prevented. AddPlayer and SetTrueName are
                    // admin actions, so a name they carry is deliberate and kept as sent; an empty
                    // one is how an admin asks for a drawn name instead.
                    //
                    // An exhausted reservoir leaves whatever arrived, and the engine refuses it as
                    // a duplicate -- a wrong name is worse than a failed action.
                    let unnamed = match &mut request.payload {
                        Action::UseAbility(use_ability) => match &mut use_ability.ability_args {
                            AbilityBehaviour::TrueNameReroll(reroll) => Some(&mut reroll.true_name),
                            _ => None,
                        },
                        Action::AddPlayer(add) if add.true_name.is_empty() => {
                            Some(&mut add.true_name)
                        }
                        Action::SetTrueName(set) if set.true_name.is_empty() => {
                            Some(&mut set.true_name)
                        }
                        _ => None,
                    };
                    if let Some(slot) = unnamed
                        && let Some(name) =
                            names.draw(|name| true_names.values().any(|held| held == name))
                    {
                        *slot = name;
                    }

                    let line = to_line(&request);
                    if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                        // the pipe died on the write, so this action never ran. same story as a
                        // crash; the fd arm will rebuild and resaturate.
                        //
                        // ask for the kill too. a failed write means the read end is gone, so the
                        // child is all but certainly dead already and the supervisor is on its way to
                        // replacing it -- but if it somehow is not, nothing else would ever notice:
                        // with stdin gone this arm is disabled and the watchdog is not armed, so the
                        // game would wedge in silence.
                        stdin = None;
                        let _ = kill_in.try_send(());
                        let pair = ResponsePair { input: ServerInput::Action(request), output: ExecOutcome::Action(ActionOutcome::Crashed) };
                        broadcast(&state, game_id, &history, history.head(), Some((ticket, pair)));
                    } else {
                        in_flight = Some(InFlight { ticket: Some(ticket), logged: true, request });
                    }
                }

                // response from child
                line = async { stdout.as_mut().unwrap().next_line().await }, if stdout.is_some() => {
                    let Ok(Some(text)) = line else {
                        stdout = None;
                        continue;
                    };

                    // the runtime is the other half of this protocol and we built both. an
                    // undeserializable line means the two binaries disagree about the wire format --
                    // a deploy mistake, not a runtime condition, and not something to limp along with.
                    let result: ExecutionResult = match serde_json::from_str(&text) {
                        Ok(result) => result,
                        Err(e) => {
                            eprintln!("engine output failed to deserialize: {e} -- aborting");
                            std::process::abort()
                        }
                    };

                    if to_discard > 0 {
                        to_discard -= 1; // resaturation echo; its commands are already in `log`
                        // the watchdog measures SILENCE, not how long the whole replay takes -- a
                        // long enough log would otherwise trip it and get a perfectly healthy child
                        // killed, then do it again on every rebuild. an echo is proof of progress, so
                        // the window restarts (and closes once nothing more is owed).
                        deadline = (to_discard > 0)
                            .then(|| Instant::now() + Duration::from_secs(ENGINE_TIMEOUT));
                        continue;
                    }

                    let Some(InFlight { ticket, logged, request }) = in_flight.take() else {
                        // the engine spoke with nothing owed. there is no good response available:
                        // aborting punishes every other game on the box for one child's weirdness,
                        // and rebooting would just reproduce whatever caused it.
                        // TODO: log this loudly (which engine, what it said) and carry on.
                        continue;
                    };

                    // The engine ran the job queue up to this stamp, whether or not the action
                    // itself was accepted, so this is what a rebuild has to reach. Recorded for
                    // BOTH arms below -- a rejected action and a null tick each advance the engine
                    // and neither goes into `accepted`.
                    clock = clock.max(request.timestamp);

                    let (output, commands) = match result {
                        Ok((response, context)) => {
                            // a null tick is not logged: it carries no intent of its own, only a
                            // clock, and `clock` above is how that clock survives a rebuild. its
                            // COMMANDS are logged and fanned out like anything else -- what the
                            // catchup actually did is real state, and a client reconnecting must
                            // still see it.
                            if logged {
                                accepted.push(request.clone());
                            }
                            (ActionOutcome::Ok(response), context.commands)
                        }
                        // a rejected action changed nothing of its own, so it is not replayed. it can
                        // still carry catchup commands: the job queue runs on the way in and its
                        // effects are real regardless of what the requested action did. those jobs
                        // are not lost on a rebuild either -- `clock` covers them.
                        Err((error, context)) => (ActionOutcome::Err(error), context.commands),
                    };

                    // Read before the commands are handed to the log, which takes them by value.
                    // TrueNameUpdate is emitted on every SetTrueName, including a player's first,
                    // so this mirror is complete without any special case for creation.
                    for payload in &commands {
                        if let Command::TrueNameUpdate {
                            target_id,
                            true_name,
                        } = &payload.cmd
                        {
                            true_names.insert(*target_id, true_name.clone());
                        }
                    }

                    // A new player slot gets a display name drawn for it, so nobody is ever left
                    // rendering as a raw slot. Cosmetic, like a colour in a lobby, and drawn
                    // independently of the true name. An admin replaces it with SetProfile.
                    //
                    // Written before the broadcast below, which is what makes it ride out with the
                    // MapPlayer that entitles a connection to it (see actors_introduced_by). The
                    // guard is for a rebuilt game: profiles are server state and outlive the child.
                    {
                        let mut server_state = lock_state(&state);
                        if let Some(game) = server_state.games.get_mut(&game_id) {
                            for payload in &commands {
                                let Command::MapPlayer { player_id } = &payload.cmd else {
                                    continue;
                                };
                                if game.profiles.contains_key(player_id) {
                                    continue;
                                }
                                let taken = |name: &str| {
                                    game.profiles.values().any(|profile| {
                                        profile.display_name.as_deref() == Some(name)
                                    })
                                };
                                let Some(display_name) = names.draw(taken) else {
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

                    let at = history.extend(commands);

                    // commands go to everyone entitled to them either way; only the reply needs an
                    // originating connection, and a server-issued action has none.
                    let reply = ticket.map(|ticket| {
                        let pair = ResponsePair {
                            input: ServerInput::Action(request),
                            output: ExecOutcome::Action(output),
                        };
                        (ticket, pair)
                    });
                    broadcast(&state, game_id, &history, at, reply);
                }

                // drive time forward. the engine only advances when an action arrives, so without
                // this nothing time-based ever happens: a scheduled kill, a poll timeout, a
                // prosecution phase or a release would all sit in the job queue until some player
                // happened to act. this is what makes them land on the clock instead.
                //
                // a Null action is the right instrument precisely because it does nothing itself.
                // execute() runs the job queue for everything up to its timestamp before the action,
                // and ActionExt::execute runs Update (polls, prosecutions, deferred-command flush)
                // after every action regardless -- so an empty action collects all of it. sending
                // Update explicitly would just run that sweep twice.
                _ = tick.tick(), if stdin.is_some() && in_flight.is_none() => {
                    // nobody is watching, so there is nothing to deliver. skipping keeps an idle game
                    // from waking its engine every few seconds forever; the jobs are not lost, they
                    // run on the catchup of whichever action or tick comes next.
                    let watched = {
                        let server_state = lock_state(&state);
                        server_state.games.get(&game_id).is_some_and(|game| {
                            game.connections.values().any(|conn| !conn.dropped)
                        })
                    };
                    if !watched {
                        continue;
                    }

                    let request = ActionRequest {
                        actor: ActionActor::System,
                        timestamp: now(),
                        payload: Action::Null(Null {}),
                    };

                    let line = to_line(&request);
                    if stdin.as_mut().unwrap().write_all(line.as_bytes()).await.is_err() {
                        stdin = None;
                        let _ = kill_in.try_send(());
                    } else {
                        in_flight = Some(InFlight { ticket: None, logged: false, request });
                    }
                }

                // the engine owes us a line and has not produced one. treat it exactly as a crash --
                // the only difference is that a hang never announces itself, so we have to notice.
                _ = async { sleep_until(deadline.unwrap()).await }, if deadline.is_some() => {
                    // reply here rather than leaving it to the fd arm: clearing in_flight now is what
                    // stops the watchdog re-arming and firing a second kill before the child is gone.
                    if let Some(InFlight { ticket, request, .. }) = in_flight.take() {
                        deliver_crash(&state, game_id, ticket, request);
                    }

                    // abandon the pipes: a late line from a child we have given up on must not be
                    // mistaken for a live response, and nothing more may be written to it.
                    stdin = None;
                    stdout = None;
                    to_discard = 0;

                    // full means a kill is already pending, which says everything this one would.
                    let _ = kill_in.try_send(());
                }

                // every branch disabled: no pipe, and no supervisor left to hand us a new one. select!
                // PANICS on that rather than parking, and a panic here dies inside a spawned task, so
                // exit deliberately instead -- nothing can make progress again either way.
                else => break,
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
            deadline = match (in_flight.is_some() || to_discard > 0, deadline) {
                (false, _) => None,
                (true, Some(running)) => Some(running),
                (true, None) => Some(Instant::now() + Duration::from_secs(ENGINE_TIMEOUT)),
            };
        }
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
