use std::collections::{HashMap, HashSet};

use lawliet_types::{
    command::{Command, CommandPayload, CommandRecipient, TapInOutcome},
    common::{ActorKey, LogID, Time, ViewportKey},
};

use crate::{
    auth::{Capability, Key, KeyData, Privileges, Ticket},
    state::{ConnHandle, GameHandle, GameId, WrappedServerState, lock_state},
    wire::{
        Batch, BatchKind, LogCommand, LogType, OutputData, Profile, ResponsePair, ServerCmd,
        ServerOutput, ViewGate,
    },
};

// convert an engine payload to its stored server output: derive the view gates from the recipient,
// keep the engine command as the payload. nothing server-computed is done here -- a log dump is a
// property of CONSUMPTION (see History::transform), because it depends on how much of the log has
// been written by that point.
pub fn engine_to_server_cmd(cmd: &CommandPayload) -> ServerOutput {
    let view_gates = match &cmd.recipient {
        CommandRecipient::System => vec![ViewGate::Admin],
        CommandRecipient::Viewport(viewport) => {
            vec![ViewGate::Viewport(*viewport), ViewGate::Admin]
        }
        CommandRecipient::Actor(id) => vec![ViewGate::Player(*id)],
        // a record is nobody: kept for later queries, delivered to no client.
        CommandRecipient::Log(_) => Vec::new(),
    };

    ServerOutput {
        time: cmd.timestamp,
        view_gates,
        data: OutputData::Engine(cmd.cmd.clone()),
    }
}

// The connection's own privilege set, sent DIRECTLY to it as the first output of every sync (a
// fresh attach or a re-sync after a privilege change). This is connection-level context -- owned by
// the specific connection it is pushed to, never stored in the shared log and never routed to a
// view -- so it carries an EMPTY gate list: no actor, no viewport, no admin reach. "empty" here
// means "this connection's concern", which is a different thing from an empty gate in the log
// (that one delivers to nobody).
fn privilege_output(privileges: &Privileges, time: Time) -> ServerOutput {
    ServerOutput {
        time,
        view_gates: vec![],
        data: OutputData::Server(ServerCmd::Privileges(crate::wire::privileges_to_wire(
            privileges,
        ))),
    }
}

// the game's clock anchor, riding the world-data viewport like every other piece of server-computed
// world state (the ProfileRoster). It is game-wide, not connection-specific, so it lives in the log
// and reaches anyone who can read the world: it anchors the game's virtual time to `sent_at` real
// wall time so a client can derive the current game time.
pub fn game_clock_output(data_viewport: ViewportKey, time: Time, sent_at: u128) -> ServerOutput {
    ServerOutput {
        time,
        view_gates: vec![ViewGate::Viewport(data_viewport), ViewGate::Admin],
        data: OutputData::Server(ServerCmd::GameClock { sent_at }),
    }
}

// The whole key ledger, delivered whole: every key and the privilege set it currently allows. Gated
// Admin like the rest of the management surface, and replaced wholesale by the client, never diffed.
pub fn key_roster_output(keys: &HashMap<Key, KeyData>, time: Time) -> ServerOutput {
    let keys: Vec<(Key, crate::wire::PrivilegeSet)> = keys
        .iter()
        .map(|(key, data)| {
            (
                key.clone(),
                crate::wire::privileges_to_wire(&data.privileges),
            )
        })
        .collect();
    ServerOutput {
        view_gates: vec![ViewGate::Admin],
        data: OutputData::Server(ServerCmd::KeyRoster { keys }),
        time,
    }
}

// What the SERVER knows about who occupies the slots, aimed at the DATA viewport -- the same one
// actor existence (MapActor) rides. The gate pair matches the engine's own for viewport-addressed
// commands (Viewport + Admin), and the shared viewport is the whole correctness argument: anyone
// who can read this roster has already been walked the actor mappings it names, by design, so a
// client never learns a name for a slot it does not hold. Delivered whole on every change.
pub fn profile_roster_output(
    data_viewport: ViewportKey,
    profiles: &HashMap<ActorKey, Profile>,
    time: Time,
) -> ServerOutput {
    let profiles: Vec<(ActorKey, Profile)> =
        profiles.iter().map(|(k, v)| (*k, v.clone())).collect();
    ServerOutput {
        view_gates: vec![ViewGate::Viewport(data_viewport), ViewGate::Admin],
        data: OutputData::Server(ServerCmd::ProfileRoster { profiles }),
        time,
    }
}

// raw per-connection info used by History's delivery: which viewports the connection has seen, up
// to what point, and which of its actors currently grant each.
#[derive(Default)]
pub struct ViewportData {
    pub delivered_to: usize, // how much has this connection seen of this viewport? (prevent
    // repeated data on unrelated viewport entries)
    pub players: HashSet<ActorKey>, // which players in this connection are granting access to this viewport?
}

impl ViewportData {
    pub fn player_may_view(&self) -> bool {
        !self.players.is_empty()
    }
}

// held per connection.
// what has this connection seen within a viewport?
#[derive(Default)]
pub struct DeliveryData {
    // which viewports has this connection seen,
    // and up to which point? do they still have access?
    pub viewports: HashMap<ViewportKey, ViewportData>,
}

impl DeliveryData {
    // does one gate pass for this connection? reach is the key's privileges plus the viewport
    // membership held here.
    fn gate_passes(&self, privileges: &Privileges, gate: &ViewGate) -> bool {
        match gate {
            ViewGate::Admin => privileges.capabilities.contains(Capability::Administer),
            ViewGate::Viewport(viewport) => self
                .viewports
                .get(viewport)
                .is_some_and(ViewportData::player_may_view),
            ViewGate::Player(actor) => privileges.actors.contains(actor),
        }
    }
}

// history is walked and delivered PER connection.
// delivery flow:
// - ready an empty batch
// - walk some section of history (walks from a start index)
// - as you go, keep constructing the batch
// - attempt to send the batch to the connection. if full, cut the connection.

// a log of server outputs/individual packets, viewport indices, and log indices
pub struct History {
    // the game this history belongs to
    pub game_id: GameId,
    pub cmds: Vec<ServerOutput>,
    // indices into cmds
    pub viewports: HashMap<ViewportKey, Vec<usize>>,
    pub logs: HashMap<LogID, Vec<usize>>,
}

impl History {
    pub fn new(game_id: GameId) -> Self {
        Self {
            game_id,
            viewports: HashMap::default(),
            logs: HashMap::default(),
            cmds: vec![],
        }
    }

    // append to history and return the next start position (inclusive). records the viewport/log
    // each output belongs to (from its engine recipient) so a later entry, or a log dump, can pull
    // exactly what it is owed without scanning the whole log.
    pub fn append_engine(&mut self, cmds: Vec<CommandPayload>) -> usize {
        let start = self.cmds.len();
        for (offset, payload) in cmds.iter().enumerate() {
            match &payload.recipient {
                CommandRecipient::Viewport(viewport) => self
                    .viewports
                    .entry(*viewport)
                    .or_default()
                    .push(start + offset),
                CommandRecipient::Log(log) => {
                    self.logs.entry(*log).or_default().push(start + offset)
                }
                _ => {}
            }
            self.cmds.push(engine_to_server_cmd(payload));
        }
        start
    }

    // the current log length: where a deliver that added nothing must begin and end.
    pub fn head(&self) -> usize {
        self.cmds.len()
    }

    // append server outputs that did not come from the engine -- snapshots like rosters -- to the
    // log. they already carry their gates and need no viewport/log indexing (there is nothing to
    // backfill or dump): a later replay reconstructs the exact state each change was made in.
    pub fn append_server(&mut self, outputs: Vec<ServerOutput>) -> usize {
        let at = self.cmds.len();
        self.cmds.extend(outputs);
        at
    }

    // the `data` for one filtered log, as of `pos` -- everything logged to it up to and including that
    // point. a dump is its own distinct instance, so repeats across dumps are expected and fine.
    fn log_commands<F>(&self, log: LogID, pos: usize, filter: F) -> Vec<LogCommand>
    where
        F: Fn(&LogCommand) -> bool,
    {
        let positions = self.logs.get(&log).map(Vec::as_slice).unwrap_or(&[]);
        let cut = positions.partition_point(|&i| i <= pos);
        positions[..cut]
            .iter()
            .map(|&i| match &self.cmds[i].data {
                OutputData::Engine(cmd) => LogCommand {
                    time: self.cmds[i].time,
                    data: cmd.clone(),
                },
                _ => unreachable!("a logged record is always an engine command"),
            })
            .filter(|cmd| filter(cmd))
            .collect()
    }

    // a stored output, shaped for THIS connection at THIS point: a log dump is filled with
    // everything its log held up to here. the raw reveal command is never handed to a client.
    fn transform(&self, output: &ServerOutput, pos: usize) -> ServerOutput {
        let t = output.time;
        match &output.data {
            OutputData::Engine(Command::RevealAutopsyMessages {
                log,
                range,
                // TODO:
                // ai integration
                redact_names,
            }) => {
                let lowest = t.saturating_sub(*range);
                let actor = output
                    .view_gates
                    .iter()
                    .find_map(|g| match g {
                        ViewGate::Player(actor) => Some(*actor),
                        _ => None,
                    })
                    .expect("autopsy is always addressed to a player");
                ServerOutput {
                    time: output.time,
                    view_gates: output.view_gates.clone(),
                    data: OutputData::Server(ServerCmd::LogDump {
                        log_type: LogType::Autopsy(actor),
                        data: self.log_commands(*log, pos, |cmd| cmd.time >= lowest),
                    }),
                }
            }
            OutputData::Engine(Command::TapInResult {
                contact_id,
                outcome: TapInOutcome::Found { log, range },
            }) => {
                let lowest = if let Some(range_val) = range {
                    t.saturating_sub(*range_val)
                } else {
                    0
                };
                ServerOutput {
                    time: output.time,
                    view_gates: output.view_gates.clone(),
                    data: OutputData::Server(ServerCmd::LogDump {
                        log_type: LogType::TapIn(*contact_id),
                        data: self.log_commands(*log, pos, |cmd| cmd.time >= lowest),
                    }),
                }
            }
            // a miss/non-loggable tap-in, and everything else, passes through as it was stored.
            _ => output.clone(),
        }
    }

    // walk history from `start`, building the batch of everything this connection is entitled to
    // (any gate passing), advancing its delivery data as it goes. access changes (enter/exit) are
    // seen by exactly the connection they concern and are applied here, splicing a late entry's
    // backfilled history in at the right place.
    fn build_batch(
        &self,
        data: &mut DeliveryData,
        privileges: &Privileges,
        start: usize,
    ) -> Vec<ServerOutput> {
        let mut out = Vec::new();

        for pos in start..self.cmds.len() {
            let output = &self.cmds[pos];

            if output
                .view_gates
                .iter()
                .any(|gate| data.gate_passes(privileges, gate))
            {
                // reading a viewport (by either gate) advances its watermark, so a later entry of
                // this connection's own actor redelivers nothing.
                for gate in &output.view_gates {
                    if let ViewGate::Viewport(viewport) = gate {
                        let entry = data.viewports.entry(*viewport).or_default();
                        entry.delivered_to = entry.delivered_to.max(pos + 1);
                    }
                }
                out.push(self.transform(output, pos));
            }

            match &output.data {
                OutputData::Engine(Command::EnterViewport { viewport, actor })
                    if privileges.actors.contains(actor) =>
                {
                    let entry = data.viewports.entry(*viewport).or_default();
                    // only the first holder backfills; a second actor arriving into a viewport this
                    // connection already reads has nothing to catch up on.
                    let first = entry.players.is_empty();
                    entry.players.insert(*actor);

                    if first {
                        // the enter is already in `out`, so the backfill lands after it: a client is
                        // never handed content for a viewport it has not been told it can read.
                        let from = entry.delivered_to;
                        let positions = self
                            .viewports
                            .get(viewport)
                            .map(Vec::as_slice)
                            .unwrap_or(&[]);
                        let cut = positions.partition_point(|&i| i < from);
                        out.extend(
                            positions[cut..]
                                .iter()
                                .copied()
                                .take_while(|&i| i < pos)
                                .map(|i| self.transform(&self.cmds[i], i)),
                        );
                        entry.delivered_to = entry.delivered_to.max(pos);
                    }
                }
                OutputData::Engine(Command::ExitViewport { viewport, actor })
                    if privileges.actors.contains(actor) =>
                {
                    if let Some(entry) = data.viewports.get_mut(viewport) {
                        entry.players.remove(actor);
                    }
                }
                _ => {}
            }
        }

        out
    }

    // walk history from start point, construct a batch, and deliver to a connection. `now` is the
    // connection's current game time, stamped onto the connection-level outputs (its privileges)
    // that lead the batch -- those are not moments on the timeline, but they still want to be
    // correct if a client ever reads an "as of" off them.
    pub fn deliver(
        &self,
        state: &WrappedServerState,
        start: usize,
        ticket: Ticket,
        kind: BatchKind,
        now: Time,
    ) {
        let mut server_state = lock_state(state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return; // game is gone
        };

        // split by field so the connection is edited while the key ledger is still read.
        let GameHandle {
            tickets,
            connections,
            keys,
            ..
        } = game;

        let Some(key) = tickets.get(&ticket) else {
            return;
        };
        let Some(privileges) = keys.get(key).map(|kd| &kd.privileges) else {
            return;
        };
        let Some(conn) = connections.get_mut(&ticket) else {
            return;
        };
        if conn.dropped {
            return;
        }

        let mut outputs = self.build_batch(&mut conn.delivery, privileges, start);

        // an initialize is the "you are caught up" marker and goes out even when empty; anything
        // else that produced nothing sends nothing.
        if outputs.is_empty() && !matches!(kind, BatchKind::Initialize) {
            return;
        }

        // every sync leads with the connection's own privileges, ahead of the replay it concerns:
        // by the time gated output arrives the client already knows what it can see. Deliveries for
        // a changed key arrive by this same route (see game::resync_key), so a rebuild always
        // carries the current set.
        if matches!(kind, BatchKind::Initialize) {
            outputs.insert(0, privilege_output(privileges, now));
        }

        push_batch(conn, Batch { kind, outputs });
    }

    // re-sync one connection from the start of the log. drops the connection's delivery data first:
    // a sync is "rebuild everything from zero" (a fresh attach, or a client that must be reset because
    // the timeline it was on no longer exists), so stale watermarks must not suppress a second delivery.
    pub fn deliver_sync(&self, state: &WrappedServerState, ticket: Ticket, now: Time) {
        // watermark reset is part of the sync contract, so a caller cannot forget it: drip with an
        // Initialize batch but keep old watermarks would silently skip history the client must rebuild.
        {
            let mut server_state = lock_state(state);
            if let Some(game) = server_state.games.get_mut(&self.game_id)
                && let Some(conn) = game.connections.get_mut(&ticket)
            {
                conn.delivery = DeliveryData::default();
            }
        }
        self.deliver(state, 0, ticket, BatchKind::Initialize, now)
    }

    // re-sync every connection. used after a backward time jump, when the engine has been rebuilt onto
    // an earlier base and every client's view of the world no longer describes the timeline it is now on.
    pub fn resync_all(&self, state: &WrappedServerState, now: Time) {
        let mut server_state = lock_state(state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return;
        };
        let tickets: Vec<Ticket> = game.connections.keys().cloned().collect();
        drop(server_state);

        for ticket in tickets {
            self.deliver_sync(state, ticket, now);
        }
    }

    // deliver to all connections
    pub fn broadcast(
        &self,
        state: &WrappedServerState,
        start: usize,
        // only for the specific connection that sent in the request
        reply: Option<(Ticket, ResponsePair)>,
    ) {
        let mut server_state = lock_state(state);
        let Some(game) = server_state.games.get_mut(&self.game_id) else {
            return; // game is gone, and so are its connections
        };

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
            let Some(key) = tickets.get(ticket) else {
                continue;
            };
            let Some(privileges) = keys.get(key).map(|kd| &kd.privileges) else {
                continue;
            };

            // the response goes only to the connection that asked.
            let response = if Some(ticket) == reply_ticket.as_ref() {
                reply.take()
            } else {
                None
            };

            let outputs = self.build_batch(&mut conn.delivery, privileges, start);

            if outputs.is_empty() && response.is_none() {
                continue;
            }

            push_batch(
                conn,
                Batch {
                    kind: BatchKind::Live(response),
                    outputs,
                },
            );
        }
    }
}

// best effort by design: a connection whose outbox is full is cut, not waited on -- a client
// missing state is worse than one that has to reconnect.
fn push_batch(conn: &mut ConnHandle, batch: Batch) {
    if conn.outbox.try_send(batch).is_err() {
        conn.cancel.cancel();
        conn.dropped = true;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use enumflags2::BitFlags;
    use lawliet_types::common::Time;
    use slotmap::KeyData;

    use super::*;
    use crate::auth::ActorScope as Scope;

    fn actor(n: u64) -> ActorKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn viewport(n: u64) -> ViewportKey {
        KeyData::from_ffi(n | (1 << 32)).into()
    }

    fn privileges(actors: &[ActorKey], administer: bool) -> Privileges {
        Privileges {
            actors: if administer {
                Scope::All
            } else {
                Scope::Only(actors.iter().copied().collect::<HashSet<_>>())
            },
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

    fn payload_at(time: Time, recipient: CommandRecipient, cmd: Command) -> CommandPayload {
        CommandPayload {
            timestamp: time,
            recipient,
            cmd,
        }
    }

    // viewport-addressed filler, tagged so assertions can identify which one came back.
    fn content(vp: ViewportKey, tag: &str) -> CommandPayload {
        payload(
            CommandRecipient::Viewport(vp),
            Command::AnonymousAnnouncement {
                content: tag.into(),
            },
        )
    }

    fn enter(vp: ViewportKey, who: ActorKey) -> CommandPayload {
        payload(
            CommandRecipient::Actor(who),
            Command::EnterViewport {
                viewport: vp,
                actor: who,
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

    fn history(log: &[CommandPayload]) -> History {
        let mut history = History::new(0);
        history.append_engine(log.to_vec());
        history
    }

    fn tags(outputs: &[ServerOutput]) -> Vec<String> {
        outputs
            .iter()
            .filter_map(|out| match &out.data {
                OutputData::Engine(Command::AnonymousAnnouncement { content }) => {
                    Some(content.clone())
                }
                _ => None,
            })
            .collect()
    }

    // the tags of everything inside every LogDump, in delivery order, flattened per dump.
    fn dump_contents(outputs: &[ServerOutput]) -> Vec<Vec<String>> {
        outputs
            .iter()
            .filter_map(|out| match &out.data {
                OutputData::Server(ServerCmd::LogDump { data, .. }) => Some(
                    data.iter()
                        .filter_map(|lc| match &lc.data {
                            Command::AnonymousAnnouncement { content } => Some(content.clone()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    fn replay(privileges: &Privileges, log: &[CommandPayload]) -> Vec<String> {
        let history = history(log);
        let outputs = history.build_batch(&mut DeliveryData::default(), privileges, 0);
        tags(&outputs)
    }

    #[test]
    fn system_is_admin_only() {
        let log = vec![payload(
            CommandRecipient::System,
            Command::AnonymousAnnouncement {
                content: "mirror".into(),
            },
        )];
        assert_eq!(replay(&privileges(&[], true), &log), ["mirror"]);
        assert!(replay(&privileges(&[], false), &log).is_empty());
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
        assert_eq!(replay(&privileges(&[a], false), &log), ["for-a"]);
        assert!(replay(&privileges(&[b], false), &log).is_empty());
    }

    #[test]
    fn viewport_content_needs_access() {
        let vp = viewport(1);
        let log = vec![content(vp, "secret")];
        assert!(replay(&privileges(&[actor(1)], false), &log).is_empty());
    }

    #[test]
    fn entry_backfills_history() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a), content(vp, "after")];

        assert_eq!(replay(&privileges(&[a], false), &log), ["before", "after"]);
    }

    #[test]
    fn enter_precedes_its_own_backfill() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a)];
        let history = history(&log);
        let outputs =
            history.build_batch(&mut DeliveryData::default(), &privileges(&[a], false), 0);

        let enter_pos = outputs
            .iter()
            .position(|out| matches!(&out.data, OutputData::Engine(Command::EnterViewport { .. })))
            .expect("the enter is addressed to this actor");
        let content_pos = outputs
            .iter()
            .position(|out| {
                matches!(
                    &out.data,
                    OutputData::Engine(Command::AnonymousAnnouncement { .. })
                )
            })
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

        assert_eq!(replay(&privileges(&[a], false), &log), ["seen"]);
    }

    // Re-entry delivers exactly the gap, in order -- not a snapshot, and not the whole history.
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
            replay(&privileges(&[a], false), &log),
            ["first", "gap-1", "gap-2", "live"]
        );
    }

    // Two actors on one connection. The second entry must not re-deliver history the first already
    // brought in, and one leaving must not cut the other off.
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
            replay(&privileges(&[a, b], false), &log),
            ["history", "shared", "still-b"]
        );
    }

    // Access is per-key: an Enter for an actor this key does not hold grants it nothing.
    #[test]
    fn another_actors_entry_grants_nothing() {
        let (a, b, vp) = (actor(1), actor(2), viewport(1));
        let log = vec![enter(vp, b), content(vp, "theirs")];

        assert!(replay(&privileges(&[a], false), &log).is_empty());
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

        assert_eq!(replay(&privileges(&[a], false), &log), ["mine-1", "mine-2"]);
    }

    // THE invariant the whole design rests on: a reconnecting client receives the same outputs in
    // the same order as one that was connected throughout.
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
        let privs = privileges(&[a, b], false);

        // Live: one output per command, each advancing the same connection data over a history that
        // grows by one, exactly as the game task feeds it.
        let mut data = DeliveryData::default();
        let mut growing = History::new(0);
        let mut live = Vec::new();
        for payload in &log {
            let at = growing.append_engine(vec![payload.clone()]);
            let outputs = growing.build_batch(&mut data, &privs, at);
            live.extend(tags(&outputs));
        }

        assert_eq!(live, replay(&privs, &log));
    }

    // An admin key reads every viewport unconditionally, including ones nobody ever entered.
    #[test]
    fn admin_sees_every_viewport_including_unentered() {
        let (a, vp) = (actor(1), viewport(1));
        let log = vec![content(vp, "before"), enter(vp, a), content(vp, "after")];

        let h = history(&log);
        let outputs = h.build_batch(&mut DeliveryData::default(), &privileges(&[], true), 0);
        assert_eq!(tags(&outputs), ["before", "after"]);
        let orphan = history(&[content(viewport(2), "orphaned")]);
        let outputs = orphan.build_batch(&mut DeliveryData::default(), &privileges(&[], true), 0);
        assert_eq!(tags(&outputs), ["orphaned"]);
    }

    // TODO:
    // tap ins and autopsies are NOT full log dumps. they have conditional gates:
    // scoped to the one log they name, trimmed to their time window, and gated to the actor owed
    // them -- covered below.

    // An autopsy reveal is transformed into a LogDump of the TARGET's own record, trimmed to the
    // reveal's time window and scoped to that one log -- nothing older than the window, nothing from
    // any other log, and the raw reveal command itself never reaches the client. Gated to whoever
    // performed it.
    #[test]
    fn an_autopsy_dump_fills_only_the_targets_log_within_its_window() {
        let (other_log, target_log) = (1u16, 2u16);
        let (performer, stranger) = (actor(1), actor(2));

        let record = |log: u16, time: Time, tag: &str| {
            payload_at(
                time,
                CommandRecipient::Log(log),
                Command::AnonymousAnnouncement {
                    content: tag.into(),
                },
            )
        };
        let autopsy = |time: Time| {
            payload_at(
                time,
                CommandRecipient::Actor(performer),
                Command::RevealAutopsyMessages {
                    log: target_log,
                    range: 50,
                    redact_names: false,
                },
            )
        };

        // A 50-unit window ending at the autopsy (t=100): only the target's log, only at/after t=50.
        let log = vec![
            record(target_log, 10, "too-old"),
            record(other_log, 20, "other-log"),
            record(other_log, 80, "other-fresh"),
            record(target_log, 60, "fresh"),
            autopsy(100),
        ];
        let history = history(&log);
        let outputs = history.build_batch(
            &mut DeliveryData::default(),
            &privileges(&[performer], false),
            0,
        );

        assert_eq!(dump_contents(&outputs), vec![vec!["fresh"]]);
        // the raw reveal is never handed out; the autopsy is addressed only to its performer.
        assert!(outputs.iter().all(|o| !matches!(
            &o.data,
            OutputData::Engine(Command::RevealAutopsyMessages { .. })
        )));
        assert!(
            history
                .build_batch(
                    &mut DeliveryData::default(),
                    &privileges(&[stranger], false),
                    0
                )
                .is_empty()
        );
    }

    // A tap-in over a window behaves like the autopsy: only that log, only the messages that fall
    // inside the window. Older traffic in the same log is not swept in.
    #[test]
    fn a_tap_in_dump_respects_its_time_window() {
        let log_id = 7u16;
        let guess = actor(1);

        let record = |time: Time, tag: &str| {
            payload_at(
                time,
                CommandRecipient::Log(log_id),
                Command::AnonymousAnnouncement {
                    content: tag.into(),
                },
            )
        };
        let tap_in = |time: Time, range: Option<Time>| {
            payload_at(
                time,
                CommandRecipient::Actor(guess),
                Command::TapInResult {
                    contact_id: 5,
                    outcome: TapInOutcome::Found { log: log_id, range },
                },
            )
        };

        let log = vec![
            record(10, "too-old"),
            record(60, "in-window"),
            tap_in(100, Some(50)),
        ];
        let history = history(&log);
        let outputs = history.build_batch(
            &mut DeliveryData::default(),
            &privileges(&[guess], false),
            0,
        );

        assert_eq!(dump_contents(&outputs), vec![vec!["in-window"]]);
        // the found tap-in result is replaced by the dump, never delivered raw.
        assert!(
            outputs
                .iter()
                .all(|o| !matches!(&o.data, OutputData::Engine(Command::TapInResult { .. })))
        );
    }

    // Only a found tap-in is a dump. A miss and a dark channel are real results, delivered to the
    // guesser as the raw command -- nothing more is owed them, so no dump and no stray log traffic.
    #[test]
    fn a_miss_tap_in_is_reported_but_builds_no_dump() {
        let log_id = 7u16;
        let guess = actor(1);

        let record = |tag: &str| {
            payload(
                CommandRecipient::Log(log_id),
                Command::AnonymousAnnouncement {
                    content: tag.into(),
                },
            )
        };
        let tap_in = |outcome: TapInOutcome| {
            payload(
                CommandRecipient::Actor(guess),
                Command::TapInResult {
                    contact_id: 5,
                    outcome,
                },
            )
        };

        for outcome in [TapInOutcome::NoSuchContact, TapInOutcome::NotLoggable] {
            let log = vec![record("said-before"), tap_in(outcome)];
            let outputs = history(&log).build_batch(
                &mut DeliveryData::default(),
                &privileges(&[guess], false),
                0,
            );

            // the result rides through untouched, and nothing else -- no dump, no leaked record.
            assert!(outputs.iter().any(|o| {
                matches!(
                    &o.data,
                    OutputData::Engine(Command::TapInResult { outcome: oo, .. }) if *oo == outcome
                )
            }));
            assert_eq!(dump_contents(&outputs), Vec::<Vec<String>>::new());
            assert!(!tags(&outputs).iter().any(|t| t == "said-before"));
        }
    }

    // A tap-in (autopsy included) only ever answers from the log it names. Traffic addressed to a
    // different log, even right before it, never leaks into its dump.
    #[test]
    fn a_tap_in_dump_never_leaks_a_sibling_log() {
        let (mine, theirs) = (1u16, 2u16);
        let guess = actor(1);

        let record = |log: u16, tag: &str| {
            payload(
                CommandRecipient::Log(log),
                Command::AnonymousAnnouncement {
                    content: tag.into(),
                },
            )
        };
        let tap_in = || {
            payload(
                CommandRecipient::Actor(guess),
                Command::TapInResult {
                    contact_id: 5,
                    outcome: TapInOutcome::Found {
                        log: mine,
                        range: None,
                    },
                },
            )
        };

        let log = vec![
            record(theirs, "theirs-1"),
            record(mine, "mine-1"),
            record(theirs, "theirs-2"),
            tap_in(),
        ];
        let outputs = history(&log).build_batch(
            &mut DeliveryData::default(),
            &privileges(&[guess], false),
            0,
        );

        assert_eq!(dump_contents(&outputs), vec![vec!["mine-1"]]);
    }

    // Log-recipient records reach nobody, but a later autopsy/tap-in dump collects everything in
    // that log up to and including its own point -- each dump its own distinct instance, repeats
    // allowed.
    #[test]
    fn a_tap_in_dump_fills_everything_up_to_it() {
        let log_id: LogID = 7;
        let (guess, other) = (actor(1), actor(2));

        let record = |tag: &str| {
            payload(
                CommandRecipient::Log(log_id),
                Command::AnonymousAnnouncement {
                    content: tag.into(),
                },
            )
        };
        let tap_in = |who: ActorKey| {
            payload(
                CommandRecipient::Actor(who),
                Command::TapInResult {
                    contact_id: 5,
                    outcome: TapInOutcome::Found {
                        log: log_id,
                        range: None,
                    },
                },
            )
        };

        let log = vec![
            record("a"),
            record("b"),
            tap_in(guess),
            record("c"),
            tap_in(guess),
        ];
        let history = history(&log);

        // records address Log, so nobody sees them as commands; each tap-in is transformed into a
        // dump carrying the log up to that point, and repeats across dumps are fine.
        let outputs = history.build_batch(
            &mut DeliveryData::default(),
            &privileges(&[guess], false),
            0,
        );
        assert!(
            !tags(&outputs)
                .iter()
                .any(|t| t == "a" || t == "b" || t == "c")
        );

        let dumps: Vec<&ServerOutput> = outputs
            .iter()
            .filter(|out| matches!(&out.data, OutputData::Server(ServerCmd::LogDump { .. })))
            .collect();
        assert_eq!(dumps.len(), 2);

        let dump = |out: &ServerOutput| -> Vec<String> {
            match &out.data {
                OutputData::Server(ServerCmd::LogDump { data, .. }) => data
                    .iter()
                    .filter_map(|log_cmd| match &log_cmd.data {
                        Command::AnonymousAnnouncement { content } => Some(content.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            }
        };

        assert_eq!(dump(dumps[0]), ["a", "b"]);
        assert_eq!(dump(dumps[1]), ["a", "b", "c"]);

        // the guess's own dump is gated to them; a stranger sees nothing.
        let stranger = history.build_batch(
            &mut DeliveryData::default(),
            &privileges(&[other], false),
            0,
        );
        assert!(stranger.is_empty());
    }

    #[test]
    fn key_roster_is_admin_gated_and_carries_every_set() {
        let mut ledger = HashMap::new();
        let admin = Key::generate();
        let player = Key::generate();
        ledger.insert(
            admin.clone(),
            crate::auth::KeyData {
                cancel: tokio_util::sync::CancellationToken::new(),
                tickets: HashSet::new(),
                privileges: Privileges {
                    actors: Scope::All,
                    capabilities: Capability::Administer.into(),
                },
            },
        );
        ledger.insert(
            player.clone(),
            crate::auth::KeyData {
                cancel: tokio_util::sync::CancellationToken::new(),
                tickets: HashSet::new(),
                privileges: Privileges {
                    actors: Scope::Only(HashSet::from([actor(1)])),
                    capabilities: BitFlags::empty(),
                },
            },
        );

        let out = key_roster_output(&ledger, 0);
        assert!(out.view_gates.contains(&ViewGate::Admin));

        // an administrator passes the Admin gate; a plain player is not entrusted with it.
        let admin_data = DeliveryData::default();
        assert!(admin_data.gate_passes(&privileges(&[], true), &ViewGate::Admin));
        let player_data = DeliveryData::default();
        assert!(!player_data.gate_passes(&privileges(&[], false), &ViewGate::Admin));

        let keys = match &out.data {
            OutputData::Server(ServerCmd::KeyRoster { keys }) => keys,
            _ => panic!("expected a key roster"),
        };
        assert_eq!(keys.len(), 2);
        let restored: HashMap<_, _> = keys.iter().cloned().collect();
        assert_eq!(
            restored.get(&admin).unwrap().capabilities,
            vec![Capability::Administer]
        );
        assert!(restored.get(&player).unwrap().capabilities.is_empty());
    }

    #[test]
    fn profile_roster_rides_the_data_viewport_and_follows_its_access() {
        let vp = viewport(1);
        let mut profiles = HashMap::new();
        profiles.insert(
            actor(1),
            Profile {
                display_name: Some("Robyn".into()),
            },
        );

        let out = profile_roster_output(vp, &profiles, 0);
        assert!(out.view_gates.contains(&ViewGate::Viewport(vp)));

        // someone who has been walked the actor mappings (holds the data viewport) sees the roster;
        let mut seen = DeliveryData::default();
        seen.viewports.insert(
            vp,
            ViewportData {
                delivered_to: 0,
                players: HashSet::from([actor(1)]),
            },
        );
        assert!(
            out.view_gates
                .iter()
                .any(|g| seen.gate_passes(&privileges(&[actor(1)], false), g))
        );

        // someone outside the viewport sees nothing -- an admin still sees everything (System reads
        // every viewport, matching the engine's own gate pair), but a plain player who has not been
        // walked the mappings is not handed names for slots it does not hold.
        let stranger = DeliveryData::default();
        assert!(
            !out.view_gates
                .iter()
                .any(|g| stranger.gate_passes(&privileges(&[actor(2)], false), g))
        );

        let keys = match &out.data {
            OutputData::Server(ServerCmd::ProfileRoster { profiles }) => profiles,
            _ => panic!("expected a profile roster"),
        };
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].1.display_name.as_deref(), Some("Robyn"));
    }
}
