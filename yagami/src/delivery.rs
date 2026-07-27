// Who receives which command, and in what order.
//
// This module is the whole of the server-side access control on state. Everything the client does
// with visibility beyond what leaves here is UX, not security -- a client that ignores its own
// rules can only misrender what it was already entitled to.
//
// The core of it is `advance`: one walk of the log that serves both a live batch and a reconnect,
// which is what guarantees the two streams are identical rather than merely similar.

use std::collections::{HashMap, HashSet};

use lawliet_types::{
    command::{Command, CommandPayload, CommandRecipient},
    common::{ActorKey, ViewportKey},
};

use lawliet_types::action::ActionRequest;

use crate::{
    auth::{Capability, Privileges, Ticket},
    state::{ConnHandle, GameHandle, GameId, WrappedServerState, lock_state},
    wire::{
        ActionOutcome, Batch, ExecOutcome, OutputData, ResponsePair, ServerInput, ServerOutput,
    },
};

// viewport -> the log positions of the commands addressed to it, in order. built as the log grows
// and never persisted: it rebuilds from the log like everything else here.
//
// this exists so backfilling a viewport costs what it emits rather than the length of the log.
// entry events are frequent -- every actor state change resyncs every bug's membership -- and
// without it each one would scan the whole log to find the handful of commands it needs.
//
// it stores positions, not payloads: there is exactly one copy of every command.
pub type ViewportIndex = HashMap<ViewportKey, Vec<usize>>;

// What one walk of the log is entitled to see.
//
// `Privileges` is the obvious implementation -- "everything this key may read" -- and it is what
// live fan-out and attach walk with. The reason this is a trait at all is `Gained`: a widen has to
// walk the same log looking only for what a privilege change ADDED, and expressing that as a
// second delivery path is how the two quietly drift apart. One walk, two answers to "may I see
// this", is what keeps them identical by construction.
pub trait Reach {
    fn actor(&self, id: &ActorKey) -> bool;
    fn administers(&self) -> bool;
}

impl Reach for Privileges {
    fn actor(&self, id: &ActorKey) -> bool {
        self.actors.contains(id)
    }
    fn administers(&self) -> bool {
        self.capabilities.contains(Capability::Administer)
    }
}

// The reach a privilege change added: what a key can see now and could not before.
//
// Only ever widens. A narrowing is not expressed here because it needs no walk at all -- see
// `narrow`.
pub struct Gained<'a> {
    pub before: &'a Privileges,
    pub after: &'a Privileges,
}

impl Reach for Gained<'_> {
    fn actor(&self, id: &ActorKey) -> bool {
        self.after.actor(id) && !self.before.actor(id)
    }
    fn administers(&self) -> bool {
        self.after.administers() && !self.before.administers()
    }
}

// how much of each viewport one connection has been handed.
//
// the delivered set for a viewport is always a PREFIX of that viewport's commands, because gaining
// access always fills the entire gap. that is what lets this be one integer per viewport instead
// of a record of what was sent.
#[derive(Default)]
pub struct ViewportCursor {
    // which of this connection's actors currently have access. a SET rather than a refcount, for
    // two reasons: two actors on one connection entering the same viewport must not double-deliver
    // its history and one leaving must not cut the other off (a refcount gave that much), and a
    // narrowed key needs to drop exactly the actors it lost, which a bare count cannot answer.
    access: HashMap<ViewportKey, HashSet<ActorKey>>,
    // log position, exclusive, up to which this viewport has been delivered. left where it is on
    // exit, so a later re-entry resumes from exactly the gap.
    //
    // MONOTONIC. Every write below takes a max, so it can only ever mean "delivered up to here" --
    // never a position walked past without sending. That is what lets a widen pre-seed a probe
    // cursor with these and trust them to suppress re-delivery.
    watermark: HashMap<ViewportKey, usize>,
}

impl ViewportCursor {
    // Advance the delivered-prefix mark, never retreating it.
    fn deliver_to(&mut self, viewport: ViewportKey, position: usize) {
        let mark = self.watermark.entry(viewport).or_insert(0);
        *mark = (*mark).max(position);
    }

    fn delivered(&self, viewport: &ViewportKey) -> usize {
        self.watermark.get(viewport).copied().unwrap_or(0)
    }
}

// append everything addressed to `viewport` in [from, until) -- the gap an actor just gained access
// to. the index makes this proportional to what is actually emitted rather than to the log length.
pub fn backfill(
    out: &mut Vec<CommandPayload>,
    log: &[CommandPayload],
    index: &ViewportIndex,
    viewport: ViewportKey,
    from: usize,
    until: usize,
) {
    let Some(positions) = index.get(&viewport) else {
        return;
    };
    let start = positions.partition_point(|&pos| pos < from);
    for &pos in &positions[start..] {
        if pos >= until {
            break;
        }
        out.push(log[pos].clone());
    }
}

// walk the log from `from` and produce, in delivery order, everything this connection is entitled
// to -- advancing its cursor as it goes. this is the whole of the server-side access control on
// state: everything the client does with visibility beyond this is UX, not security.
//
// running the same walk for a live batch (from the tail) and for a reconnect (from 0, on a fresh
// cursor) is what makes the two streams identical. a reconnecting client is not replaying a
// different, filtered view of history -- it is receiving the same bytes in the same order it would
// have received them live, with each viewport's backfill spliced in at the point access was gained.
pub fn advance<R: Reach>(
    cursor: &mut ViewportCursor,
    reach: &R,
    log: &[CommandPayload],
    index: &ViewportIndex,
    from: usize,
) -> Vec<CommandPayload> {
    let mut out = Vec::new();

    for pos in from..log.len() {
        let payload = &log[pos];

        let administers = reach.administers();

        let visible = match &payload.recipient {
            CommandRecipient::System => administers,
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
                let held = administers
                    || cursor
                        .access
                        .get(viewport)
                        .is_some_and(|holders| !holders.is_empty());

                // ...and never re-send what this viewport's mark already accounts for. A no-op on
                // a live walk, where the mark trails the position by construction. It earns its
                // keep in a widen, whose probe cursor starts with the connection's real marks: it
                // is what turns "walk the whole log again" into "emit only the gaps".
                held && pos >= cursor.delivered(viewport)
            }
        };

        if visible {
            out.push(payload.clone());
            if let CommandRecipient::Viewport(viewport) = &payload.recipient {
                cursor.deliver_to(*viewport, pos + 1);
            }
        }

        // access changes are addressed to the actor they concern, so they ride the Actor arm above
        // like any other command. acting on them AFTERWARDS is what puts the Enter ahead of the
        // history it unlocks, which is the order a live client would have seen.
        match &payload.cmd {
            Command::EnterViewport {
                viewport, actor, ..
            } if reach.actor(actor) => {
                let holders = cursor.access.entry(*viewport).or_default();
                // only the FIRST holder backfills; a second actor arriving into a viewport this
                // connection is already reading has nothing to catch up on.
                let first = holders.is_empty();
                holders.insert(*actor);
                if first {
                    let resume = cursor.delivered(viewport);
                    // yields nothing when the mark is already at or past this point, which is the
                    // widen case: the connection read this viewport through another actor and the
                    // gap is empty.
                    backfill(&mut out, log, index, *viewport, resume, pos);
                    cursor.deliver_to(*viewport, pos);
                }
            }
            Command::ExitViewport { viewport, actor } if reach.actor(actor) => {
                if let Some(holders) = cursor.access.get_mut(viewport) {
                    holders.remove(actor);
                }
            }
            _ => {}
        }
    }

    out
}

// Apply a NARROWED privilege set to a cursor: drop the access held by actors the key no longer
// reaches.
//
// There is deliberately no replay here, and no batch. Narrowing is an exit at the meta level -- it
// says "no more of this from now on", exactly like an ExitViewport, and it says nothing about what
// was already delivered. Delivery cannot be undone and must not be: client state is monotonic, so
// the connection keeps the history it legitimately received from a viewport it has now left.
//
// The watermarks are left exactly where they are, and that is correct rather than merely harmless:
// they record what was delivered, which the narrowing does not change. If the key is later widened
// back into one of these viewports, resuming from them delivers precisely the gap.
//
// Capability narrowing needs no equivalent -- `advance` reads `administers` from the privilege set
// on every command, so dropping Administer takes effect on the very next one.
pub fn narrow(cursor: &mut ViewportCursor, privileges: &Privileges) {
    for holders in cursor.access.values_mut() {
        holders.retain(|actor| privileges.actors.contains(actor));
    }
}

// Deliver the history a WIDENED privilege set is now owed, and fold the new access into the cursor.
//
// This is the only privilege change that owes anything, because it is the only one where the
// connection is missing state it is now entitled to. It walks the whole log, but it is not a
// replay: the probe carries the connection's real watermarks, so every viewport it already reads
// is suppressed and only genuine gaps come back. Nothing already delivered is re-sent, which is
// what makes this safe to apply on top of live client state -- an actual replay would duplicate
// every appended message and news event on the client.
//
// `Gained` is what restricts the walk to the delta. Without it the Actor-addressed commands for
// actors the key ALREADY held would all come back too: those have no watermark to suppress them.
pub fn widen(
    cursor: &mut ViewportCursor,
    before: &Privileges,
    after: &Privileges,
    log: &[CommandPayload],
    index: &ViewportIndex,
) -> Vec<CommandPayload> {
    // access starts empty so the walk builds only what the newly-reached actors unlock; the
    // watermarks come along so it can tell an unseen gap from history already in hand.
    let mut probe = ViewportCursor {
        access: HashMap::new(),
        watermark: cursor.watermark.clone(),
    };

    let commands = advance(&mut probe, &Gained { before, after }, log, index, 0);

    // marks are monotonic, so the probe's are the union already.
    cursor.watermark = probe.watermark;
    for (viewport, holders) in probe.access {
        cursor.access.entry(viewport).or_default().extend(holders);
    }

    commands
}

// hand one batch to one connection, stamped with that connection's next sequence number.
//
// best effort by design: a client whose outbox is full is CUT, not waited on. the alternatives are
// unbounded memory growth or letting it silently miss mandatory state, and a client missing state is
// worse than a client that has to reconnect.
pub fn push_batch(conn: &mut ConnHandle, batch: Batch) {
    conn.seq_num += 1;
    let output = ServerOutput {
        seq_num: conn.seq_num,
        data: OutputData::Batch(batch),
    };

    if conn.outbox.try_send(output).is_err() {
        conn.cancel.cancel();
        conn.dropped = true;
    }
}

// fan the result of one execution out to every live connection: recipient-filtered commands for
// everyone who can see any of them, plus the request/response pair for whoever asked.
//
// takes the whole `log` and the position the new tail starts at, rather than just the tail: an
// actor gaining access to a viewport in this batch has to be handed that viewport's history, which
// is behind `at`.
//
// a connection that would see nothing and asked for nothing gets no batch and consumes no sequence
// number: seq counts what a socket was actually sent, so it must not tick for a no-op.
pub fn dispatch(
    state: &WrappedServerState,
    game_id: GameId,
    log: &[CommandPayload],
    index: &ViewportIndex,
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

        // the ledger and the connection map are written together under one lock, so a live
        // connection that resolves to nothing means our own bookkeeping is inconsistent -- there is
        // no privilege set to filter against and guessing one is how you leak state.
        let Some(privileges) = tickets
            .get(ticket)
            .and_then(|key| keys.get(key))
            .map(|key_data| &key_data.privileges)
        else {
            eprintln!("connection {ticket:?} has no ledger entry -- aborting");
            std::process::abort()
        };

        // only the originating connection gets the response, and only once.
        let response = if Some(ticket) == reply_ticket.as_ref() {
            reply.take()
        } else {
            None
        };

        // no cursor means the connection has not been attached yet; its replay will cover this
        // batch. the response still goes out -- it answers something this socket asked for, and
        // holding it back would leave the caller waiting on a reply that never comes.
        let visible = match conn.cursor.as_mut() {
            Some(cursor) => advance(cursor, privileges, log, index, at),
            None => Vec::new(),
        };

        if visible.is_empty() && response.is_none() {
            continue;
        }

        push_batch(
            conn,
            Batch {
                commands: visible,
                response,
            },
        );
    }
}

// tell whoever submitted an action that the engine died holding it. a server-issued action has no
// originating connection, so there is simply nobody to tell.
pub fn crashed(
    state: &WrappedServerState,
    game_id: GameId,
    log: &[CommandPayload],
    index: &ViewportIndex,
    ticket: Option<Ticket>,
    request: ActionRequest,
) {
    let Some(ticket) = ticket else {
        return;
    };

    let pair = ResponsePair {
        input: ServerInput::Action(request),
        output: ExecOutcome::Action(ActionOutcome::Crashed),
    };
    // nothing new was logged, so the walk starts at the head and produces no commands.
    dispatch(state, game_id, log, index, log.len(), Some((ticket, pair)));
}

// replay everything a freshly attached connection is entitled to, as its first batch, and install
// the cursor that live fan-out will advance from there.
//
// a single global log is what makes this correct in one pass: it is walked in emission order, so
// there is no way to hand a connection a command that references something an earlier one was
// supposed to create.
//
// sent even when the filtered result is empty -- it is the client's "you are caught up" marker, and
// without it a client cannot tell being up to date from not being attached yet.
pub fn attach(
    state: &WrappedServerState,
    game_id: GameId,
    log: &[CommandPayload],
    index: &ViewportIndex,
    ticket: &Ticket,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return;
    };

    // both of these absences are ordinary and must be ruled out BEFORE the ledger lookup, because
    // only a missing ledger entry under a connection that is still LIVE is a broken invariant:
    //   - gone entirely: the connection died before its attach was handled, taking its entry with it.
    //   - dropped: its key was revoked between the upgrade and now, which removes the ledger entry
    //     while the ConnHandle waits for its guard to run.
    match game.connections.get(ticket) {
        None => return,
        Some(conn) if conn.dropped => return,
        Some(_) => {}
    }

    let Some(privileges) = game.privileges(ticket) else {
        eprintln!("connection {ticket:?} has no ledger entry -- aborting");
        std::process::abort()
    };

    // a fresh cursor walked over the whole log: attaching is re-living the stream from the start,
    // not reconstructing a snapshot of it.
    let mut cursor = ViewportCursor::default();
    let commands = advance(&mut cursor, privileges, log, index, 0);

    let Some(conn) = game.connections.get_mut(ticket) else {
        return; // connection died between sending the attach event and it being handled
    };

    // installed only now that the replay is going out, so a fan-out racing this either skips the
    // connection (no cursor yet, covered by the replay) or advances the cursor the replay left.
    conn.cursor = Some(cursor);

    push_batch(
        conn,
        Batch {
            commands,
            response: None,
        },
    );
}

// hand one connection the history its key was just widened into.
//
// the lock-and-fan-out shell around `widen`, exactly as `dispatch` is around `advance`. no batch
// goes out when the walk produces nothing: a widen that reaches no new history is a no-op, and seq
// counts what a socket was actually sent.
pub fn deliver_widening(
    state: &WrappedServerState,
    game_id: GameId,
    log: &[CommandPayload],
    index: &ViewportIndex,
    ticket: &Ticket,
    before: &Privileges,
) {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return; // game is gone, and so are its connections
    };

    // split the borrow by field, same as dispatch: the cursor is edited through `connections`
    // while the ledger is still read to resolve the key's new privilege set.
    let GameHandle {
        tickets,
        connections,
        keys,
        ..
    } = game;

    // gone, or cut between the control and now. both ordinary; see attach.
    let Some(conn) = connections.get_mut(ticket) else {
        return;
    };
    if conn.dropped {
        return;
    }

    let Some(after) = tickets
        .get(ticket)
        .and_then(|key| keys.get(key))
        .map(|key_data| &key_data.privileges)
    else {
        eprintln!("connection {ticket:?} has no ledger entry -- aborting");
        std::process::abort()
    };

    // no cursor means the connection has not attached yet, so its replay walks the whole log under
    // the new privileges and already covers everything this would have sent.
    let Some(cursor) = conn.cursor.as_mut() else {
        return;
    };

    let commands = widen(cursor, before, after, log, index);
    if commands.is_empty() {
        return;
    }

    push_batch(
        conn,
        Batch {
            commands,
            response: None,
        },
    );
}

#[cfg(test)]
mod delivery_tests {
    use std::collections::HashSet;

    use enumflags2::BitFlags;
    use lawliet_types::common::ActorKey;
    use slotmap::KeyData;

    use super::*;
    use crate::auth::ActorScope;

    // The concurrency in this file lives in dispatch/attach, which are lock-and-fan-out around the
    // real logic. `advance` and `backfill` are pure functions of (cursor, privileges, log, index),
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

    fn index_of(log: &[CommandPayload]) -> ViewportIndex {
        let mut index = ViewportIndex::new();
        for (pos, payload) in log.iter().enumerate() {
            if let CommandRecipient::Viewport(vp) = &payload.recipient {
                index.entry(*vp).or_default().push(pos);
            }
        }
        index
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

    // Replay the whole log onto a fresh cursor, as attach does.
    fn replay(privileges: &Privileges, log: &[CommandPayload]) -> Vec<CommandPayload> {
        let mut cursor = ViewportCursor::default();
        advance(&mut cursor, privileges, log, &index_of(log), 0)
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
        let index = index_of(&log);
        let privileges = privileges(&[a, b], false);

        // Live: one batch per command, each advancing the same cursor.
        let mut cursor = ViewportCursor::default();
        let mut live = Vec::new();
        for at in 0..log.len() {
            live.extend(advance(&mut cursor, &privileges, &log[..=at], &index, at));
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
        let out = advance(&mut cursor, privileges, log, &index_of(log), 0);
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
        narrow(&mut cursor, &stripped);

        log.push(content(vp, "not-theirs"));
        let index = index_of(&log);
        assert!(tags(&advance(&mut cursor, &stripped, &log, &index, 2)).is_empty());
    }

    #[test]
    fn narrowing_keeps_access_a_remaining_actor_holds() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let mut log = vec![enter(vp, a), enter(vp, b), content(vp, "shared")];
        let (mut cursor, _) = connect(&privileges(&[a, b], false), &log);

        let only_b = privileges(&[b], false);
        narrow(&mut cursor, &only_b);

        log.push(content(vp, "still-b"));
        let index = index_of(&log);
        assert_eq!(
            tags(&advance(&mut cursor, &only_b, &log, &index, 3)),
            ["still-b"]
        );
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
        let commands = widen(&mut cursor, &before, &after, &log, &index_of(&log));
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
        let commands = widen(&mut cursor, &before, &after, &log, &index_of(&log));
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
        let commands = widen(&mut cursor, &before, &after, &log, &index_of(&log));
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

        assert!(widen(&mut cursor, &held, &held, &log, &index_of(&log)).is_empty());
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
        widen(&mut cursor, &before, &after, &log, &index_of(&log));

        // both viewports must now feed this connection on the LIVE path -- the pre-existing
        // access through A included, which the fold must not have clobbered.
        log.push(content(vp1, "from-a"));
        log.push(content(vp2, "from-b"));
        let index = index_of(&log);
        assert_eq!(
            tags(&advance(&mut cursor, &after, &log, &index, 2)),
            ["from-a", "from-b"]
        );
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
        narrow(&mut cursor, &stripped);
        log.push(content(vp, "missed"));
        let index = index_of(&log);
        assert!(tags(&advance(&mut cursor, &stripped, &log, &index, 2)).is_empty());

        // ...and back. "seen" is not re-sent; "missed" is.
        let commands = widen(&mut cursor, &stripped, &held, &log, &index);
        assert_eq!(tags(&commands), ["missed"]);
    }

    #[test]
    fn backfill_bounds_are_half_open() {
        let vp = viewport(1);
        let log = vec![
            content(vp, "0"),
            content(vp, "1"),
            content(vp, "2"),
            content(vp, "3"),
        ];
        let mut out = Vec::new();
        backfill(&mut out, &log, &index_of(&log), vp, 1, 3);

        assert_eq!(tags(&out), ["1", "2"]);
    }
}
