// A viewport is an opaque identity that commands are addressed to (see
// lawliet_types::viewport). This is the engine-side record that identity names: who currently
// has access, and what kind of object allocated it.
//
// The record is NOT the authority on access. Every viewport belongs to an object — a channel, a
// bug, a poll, a passive, or the world itself — and that object's own visibility rule is what
// decides membership. The viewport is written from that rule and never consulted to answer it,
// so there is exactly one direction of flow and nothing to keep in sync.
//
// Lifetime belongs to actions, not to World: the action that creates an object allocates its
// viewport and the action that tears the object down frees it. Allocating inside a World helper
// would inherit the helper's caller's `mutate` gate invisibly, and it would put the viewport's
// lifetime somewhere other than the semantics that define it. The cost of that choice is that it
// can be forgotten, which leaks a live member set routing commands at a dead object.

use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::common::ActorKey;

pub use lawliet_types::viewport::ViewportKind;

#[derive(Debug)]
pub struct Viewport {
    pub kind: ViewportKind,
    members: IndexSet<ActorKey>,
}

// What changed when a viewport's membership was replaced wholesale. Only real transitions are
// reported, so a recompute that changes nothing reports nothing. Inline: a recompute runs on
// every state change and ability transfer, and virtually all of them move one or two actors —
// heap-allocating a pair of vectors each time is pure fragmentation.
pub type MembershipChange = SmallVec<[ActorKey; 8]>;

#[derive(Debug, Default)]
pub struct MembershipDiff {
    pub entered: MembershipChange,
    pub exited: MembershipChange,
}

impl MembershipDiff {
    pub fn is_empty(&self) -> bool {
        self.entered.is_empty() && self.exited.is_empty()
    }
}

impl Viewport {
    pub fn new(kind: ViewportKind) -> Self {
        Viewport {
            kind,
            members: IndexSet::new(),
        }
    }

    pub fn contains(&self, actor: ActorKey) -> bool {
        self.members.contains(&actor)
    }

    pub fn members(&self) -> impl Iterator<Item = ActorKey> + '_ {
        self.members.iter().copied()
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    // Returns true only on a real transition, so callers emit a command per actual change.
    pub(crate) fn grant(&mut self, actor: ActorKey) -> bool {
        self.members.insert(actor)
    }

    pub(crate) fn revoke(&mut self, actor: ActorKey) -> bool {
        self.members.swap_remove(&actor)
    }

    // Replace membership wholesale, reporting only what actually changed. This is what a
    // recompute-everything visibility pass should use: it turns a full re-evaluation into the
    // handful of commands that represent genuine access changes.
    pub(crate) fn set_members(&mut self, next: IndexSet<ActorKey>) -> MembershipDiff {
        let entered: MembershipChange = next.difference(&self.members).copied().collect();
        let exited: MembershipChange = self.members.difference(&next).copied().collect();
        self.members = next;
        MembershipDiff { entered, exited }
    }
}

#[cfg(test)]
mod presence_tests {
    use lawliet_types::command::{Command, CommandRecipient};
    use lawliet_types::role::Role;

    use crate::{
        action::{Action, ActionActor, ActionContext, ActionRequest, ActionResponse, Kill},
        actor::state::State,
        common::ActorKey,
        engine::Engine,
        test_helpers::{add_player, add_state, remove_state},
        viewport::ViewportKind,
    };

    fn has_presence(eng: &Engine, id: ActorKey) -> bool {
        eng.world
            .get_viewport(eng.world.presence_viewport)
            .unwrap()
            .contains(id)
    }

    // quick_kill is silent (no announcement), so these need a real one.
    fn announced_kill(eng: &mut Engine, target: ActorKey) -> (ActionResponse, ActionContext) {
        eng.execute(ActionRequest {
            timestamp: 0,
            actor: ActionActor::System,
            payload: Action::Kill(Kill {
                target_id: target,
                killer_id: None,
                death_message: None,
                silent: false,
                set_books_dormant: false,
                allow_link_chaining: true,
                sever_links: true,
            }),
        })
        .unwrap()
    }

    // These cover what the deferred-command queue used to guarantee. The mechanism is different
    // — membership of one viewport instead of a per-player queue re-tested on every flush — so
    // the assertions are about access, not about delivery: what a client actually receives is
    // decided by the server replaying the log against these access changes.

    #[test]
    fn new_player_starts_present() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        assert!(has_presence(&eng, p1));
    }

    #[test]
    fn losing_presence_exits_the_viewport() {
        let mut eng = Engine::new();
        let present = add_player(&mut eng, 0, Role::Civilian, "present");
        let absent = add_player(&mut eng, 0, Role::Civilian, "absent");

        // Incarceration grants NoPresence.
        let (_, ctx) = add_state(&mut eng, 0, absent, State::Incarcerated);

        assert!(has_presence(&eng, present));
        assert!(!has_presence(&eng, absent));
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(absent)
                && matches!(&p.cmd, Command::ExitViewport { actor, .. } if *actor == absent)
        }));
    }

    #[test]
    fn regaining_presence_re_enters_the_viewport() {
        let mut eng = Engine::new();
        let absent = add_player(&mut eng, 0, Role::Civilian, "absent");
        add_state(&mut eng, 0, absent, State::Incarcerated);

        let (_, ctx) = remove_state(&mut eng, 0, absent, State::Incarcerated);

        assert!(has_presence(&eng, absent));
        // Re-entry is what replays the world events missed while away: everything addressed to
        // the presence viewport since the exit is delivered, in order, on this one command.
        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Actor(absent)
                && matches!(
                    &p.cmd,
                    Command::EnterViewport { actor, kind: ViewportKind::Presence, .. }
                        if *actor == absent
                )
        }));
    }

    #[test]
    fn world_events_are_addressed_to_the_presence_viewport() {
        let mut eng = Engine::new();
        add_player(&mut eng, 0, Role::Civilian, "p1");
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (_, ctx) = announced_kill(&mut eng, victim);
        let presence = eng.world.presence_viewport;

        assert!(ctx.commands.iter().any(|p| {
            p.recipient == CommandRecipient::Viewport(presence)
                && matches!(&p.cmd, Command::Death { target_id, .. } if *target_id == victim)
        }));
        // Exactly once, and NOT mirrored to System. Admin reads every viewport, so a mirror
        // would be a duplicate — and a mirror carrying different content would be worse than a
        // duplicate: it would show admin the truth INSTEAD of the deception, hiding what the
        // players were actually told. A deception exposes its truth as a separate command.
        assert_eq!(
            ctx.commands
                .iter()
                .filter(|p| matches!(&p.cmd, Command::Death { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn the_dying_player_exits_before_their_death_is_announced() {
        let mut eng = Engine::new();
        let victim = add_player(&mut eng, 0, Role::Civilian, "victim");

        let (_, ctx) = announced_kill(&mut eng, victim);

        // Death adds State::Dead (hence NoPresence) before announcing, so the victim is already
        // out of the viewport when the announcement is addressed to it. The old deferred queue
        // reached the same outcome by queueing a copy that stayed blocked forever.
        assert!(!has_presence(&eng, victim));
        let exit_pos = ctx
            .commands
            .iter()
            .position(|p| matches!(&p.cmd, Command::ExitViewport { actor, .. } if *actor == victim))
            .expect("victim should exit the presence viewport");
        let death_pos = ctx
            .commands
            .iter()
            .position(|p| {
                matches!(p.recipient, CommandRecipient::Viewport(_))
                    && matches!(&p.cmd, Command::Death { .. })
            })
            .expect("death should be announced to the presence viewport");
        assert!(exit_pos < death_pos);
    }
}
