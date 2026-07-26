// Who receives which command, and in what order.
//
// This module is the whole of the server-side access control on state. Everything the client does
// with visibility beyond what leaves here is UX, not security -- a client that ignores its own
// rules can only misrender what it was already entitled to.
//
// The core of it is `advance`: one walk of the log that serves both a live batch and a reconnect,
// which is what guarantees the two streams are identical rather than merely similar.

use std::collections::HashMap;

use lawliet_types::{
    command::{Command, CommandPayload, CommandRecipient},
    common::ViewportKey,
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

// how much of each viewport one connection has been handed.
//
// the delivered set for a viewport is always a PREFIX of that viewport's commands, because gaining
// access always fills the entire gap. that is what lets this be two integers per viewport instead
// of a record of what was sent.
#[derive(Default)]
pub struct ViewportCursor {
    // how many of this connection's actors currently have access. refcounted rather than a set
    // because two actors on one connection entering the same viewport must not double-deliver its
    // history, and one of them leaving must not cut the other off.
    access: HashMap<ViewportKey, usize>,
    // log position, exclusive, up to which this viewport has been delivered. left where it is on
    // exit, so a later re-entry resumes from exactly the gap.
    watermark: HashMap<ViewportKey, usize>,
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
pub fn advance(
    cursor: &mut ViewportCursor,
    privileges: &Privileges,
    log: &[CommandPayload],
    index: &ViewportIndex,
    from: usize,
) -> Vec<CommandPayload> {
    let mut out = Vec::new();

    for pos in from..log.len() {
        let payload = &log[pos];

        let administers = privileges.capabilities.contains(Capability::Administer);

        let visible = match &payload.recipient {
            CommandRecipient::System => administers,
            CommandRecipient::Actor(id) => privileges.actors.contains(id),
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
                administers || cursor.access.get(viewport).is_some_and(|held| *held > 0)
            }
        };

        if visible {
            out.push(payload.clone());
            if let CommandRecipient::Viewport(viewport) = &payload.recipient {
                cursor.watermark.insert(*viewport, pos + 1);
            }
        }

        // access changes are addressed to the actor they concern, so they ride the Actor arm above
        // like any other command. acting on them AFTERWARDS is what puts the Enter ahead of the
        // history it unlocks, which is the order a live client would have seen.
        match &payload.cmd {
            Command::EnterViewport {
                viewport, actor, ..
            } if privileges.actors.contains(actor) => {
                let held = cursor.access.entry(*viewport).or_insert(0);
                *held += 1;
                if *held == 1 {
                    let resume = cursor.watermark.get(viewport).copied().unwrap_or(0);
                    backfill(&mut out, log, index, *viewport, resume, pos);
                    cursor.watermark.insert(*viewport, pos);
                }
            }
            Command::ExitViewport { viewport, actor } if privileges.actors.contains(actor) => {
                if let Some(held) = cursor.access.get_mut(viewport) {
                    *held = held.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    out
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
