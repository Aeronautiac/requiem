// Who receives which command, and in what order.
//
// This module is the whole of the server-side access control on state. Everything the client does
// with visibility beyond what leaves here is UX, not security -- a client that ignores its own rules
// can only misrender what it was already entitled to.
//
// Three types and four entrypoints:
//
//   History        the single command log, plus the index over it
//   Reach          what one walk of that log may see
//   ViewportCursor how far one connection has got, and every walk that moves it
//
//   broadcast          -> every live connection in the game
//   deliver_catchup    -> one connection, its whole entitled history, on attach
//   deliver_widening   -> one connection, the history a privilege change just owed it
//   deliver_crash      -> one connection, the news that the engine died holding its action
//
// Every one of those walks goes through `ViewportCursor::advance`. That is deliberate and it is the
// load-bearing property of the module: a reconnecting client is handed the same commands in the same
// order as one that was connected throughout, because there is only one routine that can hand out a
// command at all.

use std::collections::{HashMap, HashSet};

use lawliet_types::{
    action::ActionRequest,
    command::{Command, CommandPayload, CommandRecipient},
    common::{ActorKey, ViewportKey},
};

use crate::{
    auth::{Key, KeyData, Privileges, Ticket},
    state::{ConnHandle, GameHandle, GameId, WrappedServerState, lock_state},
    wire::{
        ActionOutcome, Batch, ExecOutcome, OutputData, Profile, ProfileUpdate, ResponsePair,
        ServerInput, ServerOutput,
    },
};

// Every command ever emitted, in emission order, together with the index over it.
//
// One log is what keeps cross-recipient order intact: a recipient's "own log" is a FILTER over this,
// never separate storage, so a replay can never hand out a command before the one that created what
// it refers to.
//
// The index is `viewport -> the positions addressed to it`, and it is here rather than beside the log
// because it is only meaningful against this exact log -- passing the two separately is an invitation
// to let them get out of step. It stores positions, not payloads: there is exactly one copy of every
// command. It is purely an accelerator, and nothing is persisted: both rebuild from the action log.
#[derive(Default)]
pub struct History {
    log: Vec<CommandPayload>,
    index: HashMap<ViewportKey, Vec<usize>>,
}

impl History {
    // Append one execution's commands. Returns the position the new tail starts at, which is where
    // fan-out resumes from.
    pub fn extend(&mut self, commands: Vec<CommandPayload>) -> usize {
        let at = self.log.len();
        for (offset, payload) in commands.iter().enumerate() {
            if let CommandRecipient::Viewport(viewport) = &payload.recipient {
                self.index.entry(*viewport).or_default().push(at + offset);
            }
        }
        self.log.extend(commands);
        at
    }

    // One past the last command: where a batch that added nothing both begins and ends.
    pub fn head(&self) -> usize {
        self.log.len()
    }

    // Everything addressed to `viewport` in [from, until), in order -- the gap an actor just gained
    // access to. The index is what makes this cost what it yields rather than the length of the log,
    // which matters because entry events are frequent: every actor state change resyncs every bug's
    // membership.
    fn viewport_range(
        &self,
        viewport: ViewportKey,
        from: usize,
        until: usize,
    ) -> impl Iterator<Item = &CommandPayload> {
        let positions = self.index.get(&viewport).map(Vec::as_slice).unwrap_or(&[]);
        let start = positions.partition_point(|&pos| pos < from);

        positions[start..]
            .iter()
            .copied()
            .take_while(move |&pos| pos < until)
            .map(move |pos| &self.log[pos])
    }
}

// What one walk of the log may see.
//
// `full` is the ordinary case, and both live fan-out and a catch-up replay walk with it. `gained`
// exists so a widen can walk the SAME log looking only for what a privilege change added: giving
// that its own delivery routine is how two streams that must be identical quietly drift apart.
struct Reach<'a> {
    granted: &'a Privileges,
    // subtracted from `granted`, leaving only what the key could not see before. `None` means
    // subtract nothing.
    already_reached: Option<&'a Privileges>,
}

impl<'a> Reach<'a> {
    fn full(granted: &'a Privileges) -> Self {
        Self {
            granted,
            already_reached: None,
        }
    }

    fn gained(before: &'a Privileges, after: &'a Privileges) -> Self {
        Self {
            granted: after,
            already_reached: Some(before),
        }
    }

    fn actor(&self, id: &ActorKey) -> bool {
        self.granted.actors.contains(id)
            && !self
                .already_reached
                .is_some_and(|had| had.actors.contains(id))
    }

    fn administers(&self) -> bool {
        self.granted.administers() && !self.already_reached.is_some_and(Privileges::administers)
    }
}

// How much of each viewport one connection has been handed, and the walk that hands it more.
//
// The delivered set for a viewport is always a PREFIX of that viewport's commands, because gaining
// access always fills the entire gap. That is what lets this be one integer per viewport instead of a
// record of what was sent.
#[derive(Default)]
pub struct ViewportCursor {
    // which of this connection's actors currently have access. a SET rather than a refcount, for two
    // reasons: two actors on one connection entering the same viewport must not double-deliver its
    // history and one leaving must not cut the other off (a refcount gave that much), and a narrowed
    // key needs to drop exactly the actors it lost, which a bare count cannot answer.
    access: HashMap<ViewportKey, HashSet<ActorKey>>,
    // log position, exclusive, up to which each viewport has been delivered. left where it is on
    // exit, so a later re-entry resumes from exactly the gap.
    //
    // MONOTONIC. Every write goes through `deliver_to`, which takes a max, so it can only ever mean
    // "delivered up to here" -- never a position walked past without sending. That is what lets
    // `widen` seed a probe with these and trust them to suppress re-delivery.
    watermark: HashMap<ViewportKey, usize>,
    // every actor whose MapPlayer this connection has been delivered -- i.e. every player it has
    // been told exists.
    //
    // This gates the profile channel. Without it, sending a display name would be a second way to
    // learn of a player, ungated by anything the command stream decided: a viewer who was never
    // sent someone's MapPlayer would be handed their name anyway. Grows only here, and only from
    // a command that actually went out.
    known_actors: HashSet<ActorKey>,
}

impl ViewportCursor {
    fn delivered(&self, viewport: &ViewportKey) -> usize {
        self.watermark.get(viewport).copied().unwrap_or(0)
    }

    // Advance the delivered-prefix mark, never retreating it.
    fn deliver_to(&mut self, viewport: ViewportKey, position: usize) {
        let mark = self.watermark.entry(viewport).or_insert(0);
        *mark = (*mark).max(position);
    }

    // Walk the history from `from` and produce, in delivery order, everything this connection is
    // entitled to -- advancing the cursor as it goes. This is the whole of the server-side access
    // control on state: everything the client does with visibility beyond this is UX, not security.
    //
    // Running the same walk for a live batch (from the tail) and for a reconnect (from 0, on a fresh
    // cursor) is what makes the two streams identical. A reconnecting client is not replaying a
    // different, filtered view of history -- it is receiving the same bytes in the same order it
    // would have received them live, with each viewport's backfill spliced in at the point access
    // was gained.
    fn advance(&mut self, reach: &Reach, history: &History, from: usize) -> Vec<CommandPayload> {
        let mut out = Vec::new();

        for pos in from..history.log.len() {
            let payload = &history.log[pos];
            // where this command's output begins, so the tail can be inspected below.
            let handed_over = out.len();

            let visible = match &payload.recipient {
                CommandRecipient::System => reach.administers(),
                CommandRecipient::Actor(id) => reach.actor(id),
                // Administer reads every viewport, including ones nobody has ever entered. That is
                // what the capability means -- it already grants the System stream -- and it is what
                // lets admin watch a deception unfold: they receive the fiction through the presence
                // viewport exactly as the players do, and any truth the engine chooses to expose
                // arrives separately, addressed to System, for them to compose against it.
                //
                // No double-delivery results. A key reading a viewport unconditionally advances that
                // viewport's watermark on every command, so a later EnterViewport finds the gap
                // already closed and backfills nothing.
                CommandRecipient::Viewport(viewport) => {
                    let held = reach.administers()
                        || self
                            .access
                            .get(viewport)
                            .is_some_and(|holders| !holders.is_empty());

                    // ...and never re-send what this viewport's mark already accounts for. A no-op
                    // on a live walk, where the mark trails the position by construction. It earns
                    // its keep in a widen, whose probe starts with the connection's real marks: it
                    // is what turns "walk the whole log again" into "emit only the gaps".
                    held && pos >= self.delivered(viewport)
                }
            };

            if visible {
                out.push(payload.clone());
                if let CommandRecipient::Viewport(viewport) = &payload.recipient {
                    self.deliver_to(*viewport, pos + 1);
                }
            }

            // access changes are addressed to the actor they concern, so they ride the Actor arm
            // above like any other command. acting on them AFTERWARDS is what puts the Enter ahead
            // of the history it unlocks, which is the order a live client would have seen.
            match &payload.cmd {
                Command::EnterViewport {
                    viewport, actor, ..
                } if reach.actor(actor) => {
                    let holders = self.access.entry(*viewport).or_default();
                    // only the FIRST holder backfills; a second actor arriving into a viewport this
                    // connection is already reading has nothing to catch up on.
                    let first = holders.is_empty();
                    holders.insert(*actor);

                    if first {
                        // yields nothing when the mark is already at or past this point, which is
                        // the widen case: the connection read this viewport through another actor
                        // and the gap is empty.
                        let gap = history.viewport_range(*viewport, self.delivered(viewport), pos);
                        out.extend(gap.cloned());
                        self.deliver_to(*viewport, pos);
                    }
                }
                Command::ExitViewport { viewport, actor } if reach.actor(actor) => {
                    if let Some(holders) = self.access.get_mut(viewport) {
                        holders.remove(actor);
                    }
                }
                _ => {}
            }

            // One place records what this connection has been TOLD, covering both routes a command
            // reaches it by -- the live push above and the backfill spliced in by an entry. Doing it
            // per-route is how the backfilled half gets forgotten, which is exactly what happened.
            for delivered in &out[handed_over..] {
                if let Command::MapPlayer { player_id } = &delivered.cmd {
                    self.known_actors.insert(*player_id);
                }
            }
        }

        out
    }

    // Apply a NARROWED privilege set: drop the access held by actors the key no longer reaches.
    //
    // There is deliberately no replay here, and no batch. Narrowing is an exit at the meta level --
    // it says "no more of this from now on", exactly like an ExitViewport, and it says nothing about
    // what was already delivered. Delivery cannot be undone and must not be: client state is
    // monotonic, so the connection keeps the history it legitimately received from a viewport it has
    // now left.
    //
    // The watermarks are left exactly where they are, and that is correct rather than merely
    // harmless: they record what was delivered, which the narrowing does not change. If the key is
    // later widened back into one of these viewports, resuming from them delivers precisely the gap.
    //
    // Capability narrowing needs no equivalent -- `advance` reads `administers` fresh on every
    // command, so dropping Administer takes effect on the very next one.
    pub fn narrow(&mut self, privileges: &Privileges) {
        for holders in self.access.values_mut() {
            holders.retain(|actor| privileges.actors.contains(actor));
        }
    }

    // Produce the history a WIDENED privilege set is now owed, and fold the new access in.
    //
    // This is the only privilege change that owes anything, because it is the only one where the
    // connection is missing state it is now entitled to. It walks the whole log, but it is not a
    // replay: the probe carries this cursor's real watermarks, so every viewport already being read
    // is suppressed and only genuine gaps come back. Nothing already delivered is re-sent, which is
    // what makes the result safe to apply on top of live client state -- an actual replay would
    // duplicate every appended message and news event on the client.
    //
    // `Reach::gained` is what restricts the walk to the delta. Without it the Actor-addressed
    // commands for actors the key ALREADY held would all come back too: those have no watermark to
    // suppress them.
    fn widen(
        &mut self,
        before: &Privileges,
        after: &Privileges,
        history: &History,
    ) -> Vec<CommandPayload> {
        // access starts empty so the walk builds only what the newly-reached actors unlock; the
        // watermarks come along so it can tell an unseen gap from history already in hand.
        let mut probe = ViewportCursor {
            access: HashMap::new(),
            watermark: self.watermark.clone(),
            known_actors: HashSet::new(),
        };

        let commands = probe.advance(&Reach::gained(before, after), history, 0);

        // marks are monotonic, so the probe's are the union already.
        self.watermark = probe.watermark;
        for (viewport, holders) in probe.access {
            self.access.entry(viewport).or_default().extend(holders);
        }
        self.known_actors.extend(probe.known_actors);

        commands
    }

    // The profiles out of `wanted` this connection is entitled to: those, and only those, whose
    // MapPlayer it has already been delivered. Anything else would make this channel a second,
    // ungated way to learn that a player exists.
    //
    // `None` when nothing is owed, and that has to stay silent: seq counts what a socket was
    // actually sent, so it must not tick for an empty update.
    pub fn profiles_for(
        &self,
        profiles: &HashMap<ActorKey, Profile>,
        wanted: impl Iterator<Item = ActorKey>,
    ) -> Option<ProfileUpdate> {
        let entitled: Vec<(ActorKey, Profile)> = wanted
            .filter(|actor| self.known_actors.contains(actor))
            .filter_map(|actor| profiles.get(&actor).map(|profile| (actor, profile.clone())))
            .collect();

        (!entitled.is_empty()).then_some(ProfileUpdate { profiles: entitled })
    }
}

// ---- sending -----------------------------------------------------------------------------------
//
// Four entrypoints, named on the only axis that distinguishes them: `broadcast` reaches every live
// connection in the game, and each `deliver_*` reaches exactly the one named by a ticket. All four
// are the same shape -- take the lock, walk, push a batch -- and none of them decides anything about
// visibility, which lives entirely in `ViewportCursor::advance`.

// hand one thing to one connection, stamped with that connection's next sequence number.
//
// Both channels go through here, which is what puts them in ONE order: a profile can never arrive
// ahead of the MapPlayer that entitles the connection to it, because both are numbered from the
// same counter as they are sent.
//
// best effort by design: a client whose outbox is full is CUT, not waited on. the alternatives are
// unbounded memory growth or letting it silently miss mandatory state, and a client missing state is
// worse than a client that has to reconnect.
fn push(conn: &mut ConnHandle, data: OutputData) {
    conn.seq_num += 1;
    let output = ServerOutput {
        seq_num: conn.seq_num,
        data,
    };

    if conn.outbox.try_send(output).is_err() {
        conn.cancel.cancel();
        conn.dropped = true;
    }
}

fn push_batch(conn: &mut ConnHandle, batch: Batch) {
    push(conn, OutputData::Batch(batch));
}

// For a profile change, which needs no log and so is sent from the control that made it.
pub fn push_profiles(conn: &mut ConnHandle, update: ProfileUpdate) {
    push(conn, OutputData::Profiles(update));
}

// The actors a just-delivered command run introduced this connection to. Their profiles are owed
// immediately: the connection has just been told they exist, so withholding the name until some
// later change would leave every existing player nameless on arrival.
fn actors_introduced_by(commands: &[CommandPayload]) -> impl Iterator<Item = ActorKey> + '_ {
    commands.iter().filter_map(|payload| match &payload.cmd {
        Command::MapPlayer { player_id } => Some(*player_id),
        _ => None,
    })
}

// The privilege set a LIVE connection's key grants.
//
// Fatal rather than skippable: the ledger and the connection map are written together under one lock,
// so a live connection that resolves to nothing means our own bookkeeping is inconsistent. There is
// no privilege set to filter against, and guessing one is how you leak state.
//
// Every caller must therefore rule out the two ORDINARY absences first -- a connection that is gone
// entirely (it died before this was handled, taking its ledger entry with it) and one that is
// `dropped` (its key was revoked between the upgrade and now, which removes the ledger entry while
// the ConnHandle waits for its guard to run).
fn live_privileges<'a>(
    tickets: &HashMap<Ticket, Key>,
    keys: &'a HashMap<Key, KeyData>,
    ticket: &Ticket,
) -> &'a Privileges {
    let Some(key_data) = tickets.get(ticket).and_then(|key| keys.get(key)) else {
        eprintln!("connection {ticket:?} has no ledger entry -- aborting");
        std::process::abort()
    };

    &key_data.privileges
}

// Fan the result of one execution out to EVERY live connection: recipient-filtered commands for
// everyone who can see any of them, plus the request/response pair for whoever asked.
//
// Takes the whole history and the position the new tail starts at, rather than just the tail: an
// actor gaining access to a viewport in this batch has to be handed that viewport's history, which
// is behind `at`.
//
// A connection that would see nothing and asked for nothing gets no batch and consumes no sequence
// number: seq counts what a socket was actually sent, so it must not tick for a no-op.
pub fn broadcast(
    state: &WrappedServerState,
    game_id: GameId,
    history: &History,
    at: usize,
    reply: Option<(Ticket, ResponsePair)>,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return; // game is gone, and so are its connections
    };

    // split the borrow by field: the loop holds `connections` mutably while still reading the
    // ticket/key ledger to resolve each connection's privileges.
    let GameHandle {
        tickets,
        connections,
        keys,
        profiles,
        ..
    } = game;

    let (reply_ticket, mut reply) = match reply {
        Some((ticket, pair)) => (Some(ticket), Some(pair)),
        None => (None, None),
    };

    for (ticket, conn) in connections.iter_mut() {
        if conn.dropped {
            continue;
        }
        let privileges = live_privileges(tickets, keys, ticket);

        // only the originating connection gets the response, and only once.
        let response = if Some(ticket) == reply_ticket.as_ref() {
            reply.take()
        } else {
            None
        };

        // no cursor means the connection has not attached yet; its catch-up will cover this batch.
        // the response still goes out -- it answers something this socket asked for, and holding it
        // back would leave the caller waiting on a reply that never comes.
        let (commands, introduced) = match conn.cursor.as_mut() {
            Some(cursor) => {
                let commands = cursor.advance(&Reach::full(privileges), history, at);
                let introduced = cursor.profiles_for(profiles, actors_introduced_by(&commands));
                (commands, introduced)
            }
            None => (Vec::new(), None),
        };

        if commands.is_empty() && response.is_none() {
            continue;
        }

        push_batch(conn, Batch { commands, response });
        // after the batch, never before: the MapPlayer that entitles this connection to the profile
        // is in the batch we just sent.
        if let Some(update) = introduced {
            push(conn, OutputData::Profiles(update));
        }
    }
}

// The shell every ONE-connection delivery shares: take the lock, find the connection, resolve what
// its key currently grants, and stamp out whatever `outputs` produced, in order. An empty result
// sends nothing and consumes no sequence number.
//
// A list rather than one output because a delivery can owe both channels at once -- commands, then
// the profiles those commands just entitled the connection to. They must go in that order and share
// one counter, which is exactly what returning them together guarantees.
fn deliver(
    state: &WrappedServerState,
    game_id: GameId,
    ticket: &Ticket,
    outputs: impl FnOnce(&mut ConnHandle, &Privileges, &HashMap<ActorKey, Profile>) -> Vec<OutputData>,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return; // game is gone, and so are its connections
    };

    // split the borrow by field: the connection is edited while the ticket/key ledger is still read.
    let GameHandle {
        tickets,
        connections,
        keys,
        profiles,
        ..
    } = game;

    // the two ordinary absences, ruled out before the ledger lookup. see live_privileges.
    let Some(conn) = connections.get_mut(ticket) else {
        return;
    };
    if conn.dropped {
        return;
    }

    let privileges = live_privileges(tickets, keys, ticket);
    for output in outputs(conn, privileges, profiles) {
        push(conn, output);
    }
}

// Tell whoever submitted an action that the engine died holding it. A server-issued action has no
// originating connection, so there is simply nobody to tell.
pub fn deliver_crash(
    state: &WrappedServerState,
    game_id: GameId,
    ticket: Option<Ticket>,
    request: ActionRequest,
) {
    let Some(ticket) = ticket else {
        return;
    };

    // nothing new was logged, so there are no commands to go with it.
    deliver(state, game_id, &ticket, |_, _, _| {
        vec![OutputData::Batch(Batch {
            commands: Vec::new(),
            response: Some(ResponsePair {
                input: ServerInput::Action(request),
                output: ExecOutcome::Action(ActionOutcome::Crashed),
            }),
        })]
    });
}

// Replay everything a freshly attached connection is entitled to, as its first batch, and install the
// cursor that `broadcast` will advance from there.
//
// A single global log is what makes this correct in one pass: it is walked in emission order, so
// there is no way to hand a connection a command that references something an earlier one was
// supposed to create.
//
// Sent even when the filtered result is empty -- it is the client's "you are caught up" marker, and
// without it a client cannot tell being up to date from not being attached yet.
pub fn deliver_catchup(
    state: &WrappedServerState,
    game_id: GameId,
    history: &History,
    ticket: &Ticket,
) {
    deliver(state, game_id, ticket, |conn, privileges, profiles| {
        // a fresh cursor walked over the whole log: attaching is re-living the stream from the
        // start, not reconstructing a snapshot of it.
        let mut cursor = ViewportCursor::default();
        let commands = cursor.advance(&Reach::full(privileges), history, 0);

        // the roster as it stands, for exactly the players the replay just introduced. derived from
        // the replay rather than from the profile map, so a connection is never handed a name for
        // someone its own catch-up did not mention.
        let roster = cursor.profiles_for(profiles, actors_introduced_by(&commands));

        // installed only now that the replay is going out, so a fan-out racing this either skips the
        // connection (no cursor yet, covered by the replay) or advances the cursor the replay left.
        conn.cursor = Some(cursor);

        let mut outputs = vec![OutputData::Batch(Batch {
            commands,
            response: None,
        })];
        outputs.extend(roster.map(OutputData::Profiles));
        outputs
    });
}

// Hand one connection the history its key was just widened into. Nothing goes out when the walk
// produces nothing: a widen that reaches no new history is a no-op.
pub fn deliver_widening(
    state: &WrappedServerState,
    game_id: GameId,
    history: &History,
    ticket: &Ticket,
    before: &Privileges,
) {
    deliver(state, game_id, ticket, |conn, after, profiles| {
        // an unattached connection is never widened -- `apply_privilege_change` declines to send the
        // event for one, because the catch-up it is still owed walks the whole log under the new
        // privileges anyway. a cursor is also only ever installed, never taken away, so a connection
        // that had one when the event was sent still has it now. this is the same class of
        // contradiction as a missing ledger entry: our own bookkeeping disagreeing with itself.
        let Some(cursor) = conn.cursor.as_mut() else {
            eprintln!("widening a connection that never attached -- aborting");
            std::process::abort()
        };

        let commands = cursor.widen(before, after, history);
        if commands.is_empty() {
            return Vec::new();
        }

        // a widen can reach players this connection had never been told about, so it owes their
        // profiles too.
        let introduced = cursor.profiles_for(profiles, actors_introduced_by(&commands));

        let mut outputs = vec![OutputData::Batch(Batch {
            commands,
            response: None,
        })];
        outputs.extend(introduced.map(OutputData::Profiles));
        outputs
    });
}

#[cfg(test)]
mod delivery_tests {
    use std::collections::HashSet;

    use enumflags2::BitFlags;
    use lawliet_types::common::ActorKey;
    use slotmap::KeyData;

    use super::*;
    use crate::auth::{ActorScope, Capability};

    // The concurrency in this file lives in the four sending entrypoints, which are lock-and-push
    // shells around the real logic. `ViewportCursor` is a pure function of (itself, reach, history),
    // so everything worth asserting is reachable without a runtime, a socket or a game.

    fn actor(n: u64) -> ActorKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn viewport(n: u64) -> ViewportKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn privileges(actors: &[ActorKey], administer: bool) -> Privileges {
        Privileges {
            actors: ActorScope::Only(actors.iter().copied().collect::<HashSet<_>>()),
            capabilities: if administer {
                Capability::Administer.into()
            } else {
                BitFlags::empty()
            },
        }
    }

    fn payload(recipient: CommandRecipient, cmd: Command) -> CommandPayload {
        CommandPayload {
            timestamp: 0,
            recipient,
            cmd,
        }
    }

    // Viewport-addressed filler, tagged so assertions can identify which one came back.
    fn content(vp: ViewportKey, tag: &str) -> CommandPayload {
        payload(
            CommandRecipient::Viewport(vp),
            Command::AnonymousAnnouncement {
                content: tag.to_string(),
            },
        )
    }

    fn enter(vp: ViewportKey, who: ActorKey) -> CommandPayload {
        payload(
            CommandRecipient::Actor(who),
            Command::EnterViewport {
                viewport: vp,
                actor: who,
                kind: lawliet_types::viewport::ViewportKind::Channel,
            },
        )
    }

    fn exit(vp: ViewportKey, who: ActorKey) -> CommandPayload {
        payload(
            CommandRecipient::Actor(who),
            Command::ExitViewport {
                viewport: vp,
                actor: who,
            },
        )
    }

    // Built through the real `extend`, so the tests exercise the same indexing the coordinator does.
    fn history(log: &[CommandPayload]) -> History {
        let mut history = History::default();
        history.extend(log.to_vec());
        history
    }

    // The tags of the AnonymousAnnouncements in a delivered stream, in order.
    fn tags(out: &[CommandPayload]) -> Vec<String> {
        out.iter()
            .filter_map(|p| match &p.cmd {
                Command::AnonymousAnnouncement { content } => Some(content.clone()),
                _ => None,
            })
            .collect()
    }

    // Replay the whole log onto a fresh cursor, as deliver_catchup does.
    fn replay(privileges: &Privileges, log: &[CommandPayload]) -> Vec<CommandPayload> {
        ViewportCursor::default().advance(&Reach::full(privileges), &history(log), 0)
    }

    #[test]
    fn system_is_admin_only() {
        let log = vec![payload(
            CommandRecipient::System,
            Command::AnonymousAnnouncement {
                content: "mirror".into(),
            },
        )];
        assert_eq!(tags(&replay(&privileges(&[], true), &log)), ["mirror"]);
        assert!(tags(&replay(&privileges(&[], false), &log)).is_empty());
    }

    #[test]
    fn actor_commands_follow_scope() {
        let (a, b) = (actor(1), actor(2));
        let log = vec![payload(
            CommandRecipient::Actor(a),
            Command::AnonymousAnnouncement {
                content: "for-a".into(),
            },
        )];
        assert_eq!(tags(&replay(&privileges(&[a], false), &log)), ["for-a"]);
        assert!(tags(&replay(&privileges(&[b], false), &log)).is_empty());
    }

    #[test]
    fn viewport_content_needs_access() {
        let vp = viewport(1);
        let log = vec![content(vp, "secret")];
        assert!(tags(&replay(&privileges(&[actor(1)], false), &log)).is_empty());
    }

    // The central property: entering delivers what was said before you arrived.
    #[test]
    fn entry_backfills_history() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a), content(vp, "after")];

        assert_eq!(
            tags(&replay(&privileges(&[a], false), &log)),
            ["before", "after"]
        );
    }

    // ...and the Enter itself must precede the history it unlocks, or a client is handed content
    // for a viewport it has not been introduced to.
    #[test]
    fn enter_precedes_its_own_backfill() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a)];
        let out = replay(&privileges(&[a], false), &log);

        let enter_pos = out
            .iter()
            .position(|p| matches!(&p.cmd, Command::EnterViewport { .. }))
            .expect("the enter is addressed to this actor");
        let content_pos = out
            .iter()
            .position(|p| matches!(&p.cmd, Command::AnonymousAnnouncement { .. }))
            .expect("backfilled content");
        assert!(enter_pos < content_pos);
    }

    #[test]
    fn exit_stops_delivery_without_retracting() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![
            enter(vp, a),
            content(vp, "seen"),
            exit(vp, a),
            content(vp, "missed"),
        ];

        assert_eq!(tags(&replay(&privileges(&[a], false), &log)), ["seen"]);
    }

    // The prosecution guarantee: an absent player receives exactly the gap, in order, on return —
    // not a snapshot, and not the whole history again.
    #[test]
    fn re_entry_delivers_only_the_gap() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![
            enter(vp, a),
            content(vp, "first"),
            exit(vp, a),
            content(vp, "gap-1"),
            content(vp, "gap-2"),
            enter(vp, a),
            content(vp, "live"),
        ];

        assert_eq!(
            tags(&replay(&privileges(&[a], false), &log)),
            ["first", "gap-1", "gap-2", "live"]
        );
    }

    // Two actors on one connection. The second entry must not re-deliver the history the first
    // already brought in, and one of them leaving must not cut the other off.
    #[test]
    fn two_actors_share_access_without_double_delivery() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let log = vec![
            content(vp, "history"),
            enter(vp, a),
            enter(vp, b),
            content(vp, "shared"),
            exit(vp, a),
            content(vp, "still-b"),
            exit(vp, b),
            content(vp, "gone"),
        ];

        assert_eq!(
            tags(&replay(&privileges(&[a, b], false), &log)),
            ["history", "shared", "still-b"]
        );
    }

    // Access is per-key, so an Enter for an actor this key does not hold grants it nothing.
    #[test]
    fn another_actors_entry_grants_nothing() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let log = vec![enter(vp, b), content(vp, "theirs")];

        assert!(tags(&replay(&privileges(&[a], false), &log)).is_empty());
    }

    // Backfill must not leak a sibling viewport's traffic interleaved in the same log.
    #[test]
    fn backfill_is_scoped_to_one_viewport() {
        let (a, vp1, vp2) = (actor(1), viewport(1), viewport(2));
        let log = vec![
            content(vp1, "mine-1"),
            content(vp2, "theirs"),
            content(vp1, "mine-2"),
            enter(vp1, a),
        ];

        assert_eq!(
            tags(&replay(&privileges(&[a], false), &log)),
            ["mine-1", "mine-2"]
        );
    }

    // THE invariant the whole design rests on: a reconnecting client receives the same commands in
    // the same order as one that was connected throughout. If these ever diverge, reconnect stops
    // being a replay and becomes a second, subtly different protocol.
    #[test]
    fn live_delivery_and_reconnect_replay_are_identical() {
        let (a, b, vp1, vp2) = (actor(1), actor(2), viewport(1), viewport(2));
        let log = vec![
            content(vp1, "c1"),
            enter(vp1, a),
            content(vp1, "c2"),
            content(vp2, "d1"),
            enter(vp2, b),
            exit(vp1, a),
            content(vp1, "c3"),
            content(vp2, "d2"),
            enter(vp1, a),
            content(vp1, "c4"),
        ];
        let privileges = privileges(&[a, b], false);

        // Live: one batch per command, each advancing the same cursor over a history that grows by
        // one, exactly as the coordinator feeds it.
        let mut cursor = ViewportCursor::default();
        let mut growing = History::default();
        let mut live = Vec::new();
        for payload in &log {
            let at = growing.extend(vec![payload.clone()]);
            live.extend(cursor.advance(&Reach::full(&privileges), &growing, at));
        }

        assert_eq!(tags(&live), tags(&replay(&privileges, &log)));
    }

    // An admin key holds every actor, so it accumulates access from everyone else's entries.
    #[test]
    fn actor_scope_all_sees_every_viewport() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a), content(vp, "after")];
        let omniscient = Privileges {
            actors: ActorScope::All,
            capabilities: Capability::Administer.into(),
        };

        assert_eq!(tags(&replay(&omniscient, &log)), ["before", "after"]);
    }

    // ...including viewports nobody has ever entered, which a scoped key can never reach.
    #[test]
    fn admin_sees_a_viewport_nobody_entered() {
        let log = vec![content(viewport(1), "orphaned")];
        let omniscient = Privileges {
            actors: ActorScope::All,
            capabilities: Capability::Administer.into(),
        };

        assert_eq!(tags(&replay(&omniscient, &log)), ["orphaned"]);
        assert!(tags(&replay(&privileges(&[actor(1)], false), &log)).is_empty());
    }

    // Reading a viewport unconditionally must not cause an entry to re-deliver what admin has
    // already been sent: the watermark advances as content arrives, so the backfill finds no gap.
    #[test]
    fn admin_entry_does_not_redeliver() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a), content(vp, "after")];
        let omniscient = Privileges {
            actors: ActorScope::All,
            capabilities: Capability::Administer.into(),
        };

        assert_eq!(tags(&replay(&omniscient, &log)), ["before", "after"]);
    }

    // Walk the whole log onto a cursor and keep the cursor, as a live connection has one.
    fn connect(privileges: &Privileges, log: &[CommandPayload]) -> (ViewportCursor, Vec<String>) {
        let mut cursor = ViewportCursor::default();
        let out = cursor.advance(&Reach::full(privileges), &history(log), 0);
        (cursor, tags(&out))
    }

    // ---- narrowing ------------------------------------------------------------------------
    //
    // A narrowing is an exit at the meta level: it ends future delivery and touches nothing that
    // already arrived. Without `narrow` the cursor keeps the lost actors' access forever, because
    // the Exit that would close it is filtered out by the very change that should have ended it.

    #[test]
    fn narrowing_stops_delivery() {
        let (a, vp) = (actor(1), viewport(1));
        let mut log = vec![enter(vp, a), content(vp, "seen")];
        let (mut cursor, delivered) = connect(&privileges(&[a], false), &log);
        assert_eq!(delivered, ["seen"]);

        let stripped = privileges(&[], false);
        cursor.narrow(&stripped);

        log.push(content(vp, "not-theirs"));
        let out = cursor.advance(&Reach::full(&stripped), &history(&log), 2);
        assert!(tags(&out).is_empty());
    }

    #[test]
    fn narrowing_keeps_access_a_remaining_actor_holds() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let mut log = vec![enter(vp, a), enter(vp, b), content(vp, "shared")];
        let (mut cursor, _) = connect(&privileges(&[a, b], false), &log);

        let only_b = privileges(&[b], false);
        cursor.narrow(&only_b);

        log.push(content(vp, "still-b"));
        let out = cursor.advance(&Reach::full(&only_b), &history(&log), 3);
        assert_eq!(tags(&out), ["still-b"]);
    }

    // ---- widening -------------------------------------------------------------------------

    // The central widening property: history the connection never had, and nothing else.
    #[test]
    fn widening_delivers_the_new_actors_history() {
        let (a, b, vp1, vp2) = (actor(1), actor(2), viewport(1), viewport(2));
        let log = vec![
            enter(vp1, a),
            content(vp1, "a-stuff"),
            content(vp2, "b-history"),
            enter(vp2, b),
            content(vp2, "b-more"),
        ];
        let before = privileges(&[a], false);
        let (mut cursor, delivered) = connect(&before, &log);
        assert_eq!(delivered, ["a-stuff"]);

        let after = privileges(&[a, b], false);
        let commands = cursor.widen(&before, &after, &history(&log));
        assert_eq!(tags(&commands), ["b-history", "b-more"]);
    }

    // ...and specifically NOT history it already has. This is what makes a widen safe to apply on
    // top of live client state: a replay would duplicate every appended message on the client.
    #[test]
    fn widening_does_not_redeliver_a_shared_viewport() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let log = vec![enter(vp, a), enter(vp, b), content(vp, "shared")];
        let before = privileges(&[a], false);
        let (mut cursor, delivered) = connect(&before, &log);
        assert_eq!(delivered, ["shared"]);

        let after = privileges(&[a, b], false);
        let commands = cursor.widen(&before, &after, &history(&log));
        assert!(tags(&commands).is_empty());
    }

    #[test]
    fn widening_to_administer_delivers_system_and_unentered_viewports() {
        let log = vec![
            content(viewport(1), "nobody-entered"),
            payload(
                CommandRecipient::System,
                Command::AnonymousAnnouncement {
                    content: "mirror".into(),
                },
            ),
        ];
        let before = privileges(&[], false);
        let (mut cursor, delivered) = connect(&before, &log);
        assert!(delivered.is_empty());

        let after = privileges(&[], true);
        let commands = cursor.widen(&before, &after, &history(&log));
        assert_eq!(tags(&commands), ["nobody-entered", "mirror"]);
    }

    // A control that changed nothing must not resend anything -- both mutators REPLACE rather than
    // delta, so an admin restating the same privilege set is an ordinary thing to do.
    #[test]
    fn a_widen_that_gains_nothing_delivers_nothing() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![enter(vp, a), content(vp, "seen")];
        let held = privileges(&[a], false);
        let (mut cursor, _) = connect(&held, &log);

        assert!(cursor.widen(&held, &held, &history(&log)).is_empty());
    }

    // The widen walks on a throwaway probe, so the access it discovers has to be folded back or
    // the connection is handed the history and then never sees the viewport live.
    #[test]
    fn widening_folds_new_access_into_the_live_cursor() {
        let (a, b, vp1, vp2) = (actor(1), actor(2), viewport(1), viewport(2));
        let mut log = vec![enter(vp1, a), enter(vp2, b)];
        let before = privileges(&[a], false);
        let (mut cursor, _) = connect(&before, &log);

        let after = privileges(&[a, b], false);
        cursor.widen(&before, &after, &history(&log));

        // both viewports must now feed this connection on the LIVE path -- the pre-existing
        // access through A included, which the fold must not have clobbered.
        log.push(content(vp1, "from-a"));
        log.push(content(vp2, "from-b"));
        let out = cursor.advance(&Reach::full(&after), &history(&log), 2);
        assert_eq!(tags(&out), ["from-a", "from-b"]);
    }

    // The round trip, and the reason narrowing must not touch the watermarks: they record what was
    // DELIVERED, which losing access does not change. Leaving them alone is what lets a later
    // widen resume from exactly the gap instead of resending the lot.
    #[test]
    fn narrow_then_widen_delivers_exactly_the_gap() {
        let (a, vp) = (actor(1), viewport(1));
        let mut log = vec![enter(vp, a), content(vp, "seen")];
        let held = privileges(&[a], false);
        let (mut cursor, delivered) = connect(&held, &log);
        assert_eq!(delivered, ["seen"]);

        // scope narrowed away, and the world moves on without them.
        let stripped = privileges(&[], false);
        cursor.narrow(&stripped);
        log.push(content(vp, "missed"));
        let history = history(&log);
        assert!(tags(&cursor.advance(&Reach::full(&stripped), &history, 2)).is_empty());

        // ...and back. "seen" is not re-sent; "missed" is.
        let commands = cursor.widen(&stripped, &held, &history);
        assert_eq!(tags(&commands), ["missed"]);
    }

    // ---- the profile gate --------------------------------------------------------------------
    //
    // A profile may only follow a MapPlayer the connection was actually delivered. Otherwise naming
    // someone announces their existence to viewers the command stream deliberately kept them from.

    fn map_player(who: ActorKey) -> CommandPayload {
        payload(
            CommandRecipient::Viewport(viewport(9)),
            Command::MapPlayer { player_id: who },
        )
    }

    fn named(who: ActorKey, name: &str) -> HashMap<ActorKey, Profile> {
        HashMap::from([(
            who,
            Profile {
                display_name: Some(name.to_string()),
            },
        )])
    }

    #[test]
    fn a_profile_needs_its_map_player() {
        let (a, watcher, vp) = (actor(1), actor(2), viewport(9));
        let log = vec![map_player(a)];

        // never entered the viewport the roster rides, so it was never told `a` exists.
        let (blind, _) = connect(&privileges(&[watcher], false), &log);
        assert!(
            blind
                .profiles_for(&named(a, "Light"), [a].into_iter())
                .is_none()
        );

        // entered, so it was.
        let log = vec![enter(vp, watcher), map_player(a)];
        let (informed, _) = connect(&privileges(&[watcher], false), &log);
        let update = informed
            .profiles_for(&named(a, "Light"), [a].into_iter())
            .expect("told of a, so entitled to a's profile");
        assert_eq!(update.profiles[0].1.display_name.as_deref(), Some("Light"));
    }

    // Entering late must hand over the roster, not just future arrivals — the backfill delivers the
    // earlier MapPlayers, so the gate has to open for them too.
    #[test]
    fn backfilled_map_players_open_the_gate() {
        let (a, watcher, vp) = (actor(1), actor(2), viewport(9));
        let log = vec![map_player(a), enter(vp, watcher)];

        let (cursor, _) = connect(&privileges(&[watcher], false), &log);
        assert!(
            cursor
                .profiles_for(&named(a, "Light"), [a].into_iter())
                .is_some()
        );
    }

    // An unnamed slot yields nothing rather than an empty entry: seq counts what a socket was
    // actually sent, so an update with no content must not be pushed at all.
    #[test]
    fn an_unnamed_actor_yields_no_update() {
        let (a, watcher, vp) = (actor(1), actor(2), viewport(9));
        let log = vec![enter(vp, watcher), map_player(a)];

        let (cursor, _) = connect(&privileges(&[watcher], false), &log);
        assert!(
            cursor
                .profiles_for(&HashMap::new(), [a].into_iter())
                .is_none()
        );
    }

    #[test]
    fn viewport_range_bounds_are_half_open() {
        let vp = viewport(1);
        let log = vec![
            content(vp, "0"),
            content(vp, "1"),
            content(vp, "2"),
            content(vp, "3"),
        ];
        let gap: Vec<_> = history(&log).viewport_range(vp, 1, 3).cloned().collect();

        assert_eq!(tags(&gap), ["1", "2"]);
    }
}
