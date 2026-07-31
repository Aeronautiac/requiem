use std::rc::Rc;

use indexmap::{IndexMap, IndexSet, map::Entry};
use lawliet_types::{
    common::{ID, IterationCount, JobID},
    world::WorldPhase,
};
use slotmap::SlotMap;

use crate::{
    ability::Ability,
    actor::{Actor, ActorType, Player, organization::LeadershipStruct},
    bug::Bug,
    channel::Channel,
    chargepool::ChargePool,
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, IncarcerationKey,
        KidnappingKey, LogID, LoungeKey, NotebookKey, PassiveKey, PollKey, ProsecutionKey, TimerKey,
        ViewportKey,
    },
    config::{
        actor::organization::OrganizationName,
        role::Role,
        world::{WorldChannelName, WorldChargePoolName},
    },
    engine::jobs::Jobs,
    groupchat::Groupchat,
    incarceration::Incarceration,
    kidnapping::Kidnapping,
    lounge::Lounge,
    notebook::Notebook,
    passive::Passive,
    poll::Poll,
    prosecution::Prosecution,
    timer::Timer,
    viewport::{MembershipDiff, Viewport, ViewportKind},
};

#[derive(Debug)]
pub enum WorldError {
    DuplicateName,
}

const MISSING_VIEWPORT: &str =
    "viewport does not exist: engine invariant violated (an object outlived its viewport)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactChannel {
    Lounge(LoungeKey),
    Gc(GroupchatKey),
}

#[derive(Debug)]
pub struct World {
    pub blackout: bool,
    pub actors: SlotMap<ActorKey, Actor>,
    pub player_names: IndexMap<Rc<str>, ActorKey>, // a map of true names to actor ids
    pub abilities: SlotMap<AbilityKey, Ability>,
    pub notebooks: SlotMap<NotebookKey, Notebook>,
    pub passives: SlotMap<PassiveKey, Passive>,
    pub charge_pools: SlotMap<ChargePoolKey, ChargePool>,
    pub pool_map: IndexMap<WorldChargePoolName, ChargePoolKey>, // things like the world prosecution pool
    pub polls: SlotMap<PollKey, Poll>,
    pub channels: SlotMap<ChannelKey, Channel>,
    pub lounges: SlotMap<LoungeKey, Lounge>,
    pub groupchats: SlotMap<GroupchatKey, Groupchat>,
    pub bugs: SlotMap<BugKey, Bug>,
    pub prosecutions: SlotMap<ProsecutionKey, Prosecution>,
    pub kidnappings: SlotMap<KidnappingKey, Kidnapping>,
    pub incarcerations: SlotMap<IncarcerationKey, Incarceration>,
    pub world_channel_map: IndexMap<WorldChannelName, ChannelKey>,
    pub phase: WorldPhase,
    pub curr_iteration: IterationCount,
    // The scheduled turn of the next day, when days turn on their own. Held so it can be cancelled:
    // an early manual advance has to take the pending one down with it, or the day it starts gets
    // cut short by a timer belonging to the day before. Always None when the host owns the clock.
    pub iteration_job: Option<JobID>,
    pub contact_channels: IndexMap<ID, ContactChannel>,
    pub contact_channel_id: ID,
    pub viewports: SlotMap<ViewportKey, Viewport>,
    // Every countdown in the game, held here rather than on the objects they belong to so that
    // stopping time is one sweep over one map. See the timer module.
    pub timers: SlotMap<TimerKey, Timer>,
    // The two world-level singletons every present player has access to. Every other viewport is
    // allocated by the action that creates its object and freed by the action that tears that
    // object down; these belong to no object, so they are allocated here and never freed.
    //
    // Both carry the same membership except under blackout, which empties the events one and
    // leaves the data one alone. That is the entire mechanism: nobody loses presence, the world
    // just stops announcing. Which of the two a command belongs on is decided at the call site by
    // reaching for cmd_world_event or cmd_world_data.
    pub events_viewport: ViewportKey,
    pub data_viewport: ViewportKey,
    // The pending lift. Held so it can be cancelled: a blackout ended early has to take its timer
    // down with it, or the world goes dark again the moment the old one fires.
    pub blackout_job: Option<JobID>,
    // The next record to hand out. A bare counter rather than a slotmap because a log is an
    // identity and nothing else: it holds no state to store, nobody is ever granted one, and it is
    // never freed — the record of what was said outlives whatever was saying it.
    next_log: LogID,
}

impl World {
    pub fn new() -> Self {
        let mut viewports = SlotMap::with_key();
        let events_viewport = viewports.insert(Viewport::new(ViewportKind::WorldEvents));
        let data_viewport = viewports.insert(Viewport::new(ViewportKind::WorldData));
        World {
            viewports,
            events_viewport,
            data_viewport,
            timers: SlotMap::with_key(),
            phase: WorldPhase::Setup,
            curr_iteration: 0,
            iteration_job: None,
            blackout: false,
            blackout_job: None,
            actors: SlotMap::with_key(),
            abilities: SlotMap::with_key(),
            notebooks: SlotMap::with_key(),
            player_names: IndexMap::new(),
            passives: SlotMap::with_key(),
            charge_pools: SlotMap::with_key(),
            pool_map: IndexMap::new(),
            polls: SlotMap::with_key(),
            channels: SlotMap::with_key(),
            lounges: SlotMap::with_key(),
            groupchats: SlotMap::with_key(),
            bugs: SlotMap::with_key(),
            prosecutions: SlotMap::with_key(),
            kidnappings: SlotMap::with_key(),
            incarcerations: SlotMap::with_key(),
            world_channel_map: IndexMap::new(),
            contact_channels: IndexMap::new(),
            contact_channel_id: 0,
            next_log: 0,
        }
    }

    // Claim a record. Whoever wants one keeps the id; there is nothing else to hold.
    pub fn add_log(&mut self) -> LogID {
        let id = self.next_log;
        self.next_log += 1;
        id
    }

    // Plumbing only. WHEN a viewport is allocated or freed is the owning action's decision (see
    // the viewport module); these are the slotmap operations that decision reaches for.
    pub fn add_viewport(&mut self, kind: ViewportKind) -> ViewportKey {
        self.viewports.insert(Viewport::new(kind))
    }

    pub fn remove_viewport(&mut self, id: ViewportKey) {
        debug_assert!(
            id != self.events_viewport && id != self.data_viewport,
            "the world viewports outlive the world's contents and must never be freed"
        );
        self.viewports.remove(id);
    }

    pub fn get_viewport(&self, id: ViewportKey) -> Option<&Viewport> {
        self.viewports.get(id)
    }

    // Timers, on the same terms as viewports: the object that wants a countdown allocates one and
    // frees it when it is torn down. Freeing cancels — a job outliving the thing it was counting
    // down for is exactly the bug this replaces.
    pub fn add_timer(&mut self, timer: Timer) -> TimerKey {
        self.timers.insert(timer)
    }

    pub fn remove_timer(&mut self, id: TimerKey, jobs: &mut Jobs) {
        if let Some(mut timer) = self.timers.remove(id) {
            timer.cancel(jobs);
        }
    }

    // Grant access. Returns true only on a real transition, so callers emit exactly one command
    // per genuine change.
    //
    // Panics if the viewport is gone. An object holding a key to a viewport that no longer
    // exists is an inconsistent world, not a condition to route around: the engine's contract is
    // to die on that and be rebuilt by replaying the action log. Silently doing nothing would
    // instead drop the command and leave every client permanently short of state, with no
    // symptom at the point of failure.
    pub fn viewport_grant(&mut self, id: ViewportKey, actor: ActorKey) -> bool {
        self.viewports
            .get_mut(id)
            .expect(MISSING_VIEWPORT)
            .grant(actor)
    }

    // Revoke access. Returns true only on a real transition. Panics on a missing viewport, as
    // viewport_grant does.
    pub fn viewport_revoke(&mut self, id: ViewportKey, actor: ActorKey) -> bool {
        self.viewports
            .get_mut(id)
            .expect(MISSING_VIEWPORT)
            .revoke(actor)
    }

    // Replace a viewport's membership wholesale, reporting only genuine transitions. For
    // visibility rules that are evaluated by recomputing the whole answer. Panics on a missing
    // viewport, as viewport_grant does.
    pub fn viewport_set_members(
        &mut self,
        id: ViewportKey,
        members: IndexSet<ActorKey>,
    ) -> MembershipDiff {
        self.viewports
            .get_mut(id)
            .expect(MISSING_VIEWPORT)
            .set_members(members)
    }

    pub fn add_actor(&mut self, actor: Actor) -> ActorKey {
        self.actors.insert(actor)
    }

    pub fn get_actor(&self, id: ActorKey) -> Option<&Actor> {
        self.actors.get(id)
    }

    pub fn get_actor_mut(&mut self, id: ActorKey) -> Option<&mut Actor> {
        self.actors.get_mut(id)
    }

    pub fn get_player_mut(&mut self, id: ActorKey) -> Option<&mut Player> {
        if let Some(actor) = self.actors.get_mut(id) {
            if let ActorType::Player(player) = &mut actor.actor_type {
                Some(player)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_player(&self, id: ActorKey) -> Option<&Player> {
        if let Some(actor) = self.actors.get(id) {
            if let ActorType::Player(player) = &actor.actor_type {
                Some(player)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn get_player_id_by_name(&self, name: &str) -> Option<ActorKey> {
        self.player_names.get(name.to_lowercase().as_str()).copied()
    }

    pub fn add_player(&mut self, true_name: &str, role: Role) -> Result<ActorKey, WorldError> {
        let normalized = true_name.to_lowercase();
        if self.player_names.contains_key(normalized.as_str()) {
            return Err(WorldError::DuplicateName);
        }
        let key = self.actors.insert(Actor::new_player(&normalized, role));
        let name_rc: Rc<str> = self.get_player(key).unwrap().true_name.clone();
        match self.player_names.entry(name_rc) {
            Entry::Vacant(e) => {
                e.insert(key);
                Ok(key)
            }
            Entry::Occupied(_) => unreachable!(), // guarded by contains_key above
        }
    }

    // Rename a player: swap the name index over and update the player's stored true name.
    // Names are normalized to lowercase, matching add_player / get_player_id_by_name.
    // Returns false (renaming nothing) if the name is already held by a DIFFERENT player, or
    // the id isn't a player; true once the rename is applied.
    pub fn set_player_name(&mut self, id: ActorKey, new_name: &str) -> bool {
        let normalized: Rc<str> = Rc::from(new_name.to_lowercase().as_str());
        if let Some(existing) = self.player_names.get(normalized.as_ref())
            && *existing != id
        {
            return false;
        }
        let Some(player) = self.get_player_mut(id) else {
            return false;
        };
        let old = std::mem::replace(&mut player.true_name, normalized.clone());
        self.player_names.swap_remove(old.as_ref());
        self.player_names.insert(normalized, id);
        true
    }

    pub fn add_org(
        &mut self,
        name: OrganizationName,
        leadership_struct: Option<LeadershipStruct>,
        channel_id: ChannelKey,
    ) -> ActorKey {
        self.actors
            .insert(Actor::new_org(name, leadership_struct, channel_id))
    }

    pub fn add_notebook(&mut self, channel_id: ChannelKey, fake: bool) -> NotebookKey {
        self.notebooks.insert(Notebook::new(channel_id, fake))
    }

    pub fn get_notebook_mut(&mut self, id: NotebookKey) -> Option<&mut Notebook> {
        self.notebooks.get_mut(id)
    }

    pub fn get_notebook(&self, id: NotebookKey) -> Option<&Notebook> {
        self.notebooks.get(id)
    }

    pub fn add_ability(&mut self, ability: Ability) -> AbilityKey {
        self.abilities.insert(ability)
    }

    /// be careful that there are no dangling ids
    pub fn remove_ability(&mut self, id: AbilityKey) {
        self.abilities.remove(id);
    }

    pub fn get_ability(&self, id: AbilityKey) -> Option<&Ability> {
        self.abilities.get(id)
    }

    pub fn get_ability_mut(&mut self, id: AbilityKey) -> Option<&mut Ability> {
        self.abilities.get_mut(id)
    }

    pub fn add_passive(&mut self, passive: Passive) -> PassiveKey {
        self.passives.insert(passive)
    }

    /// be careful that there are no dangling ids
    pub fn remove_passive(&mut self, id: PassiveKey) {
        self.passives.remove(id);
    }

    pub fn get_passive(&self, id: PassiveKey) -> Option<&Passive> {
        self.passives.get(id)
    }

    pub fn get_passive_mut(&mut self, id: PassiveKey) -> Option<&mut Passive> {
        self.passives.get_mut(id)
    }

    pub fn remove_notebook(&mut self, id: NotebookKey) {
        self.notebooks.remove(id);
    }

    pub fn add_charge_pool(&mut self, charge_pool: ChargePool) -> ChargePoolKey {
        self.charge_pools.insert(charge_pool)
    }

    pub fn remove_charge_pool(&mut self, id: ChargePoolKey) {
        self.charge_pools.remove(id);
    }

    pub fn get_charge_pool(&self, id: ChargePoolKey) -> Option<&ChargePool> {
        self.charge_pools.get(id)
    }

    pub fn get_charge_pool_mut(&mut self, id: ChargePoolKey) -> Option<&mut ChargePool> {
        self.charge_pools.get_mut(id)
    }

    pub fn get_poll(&self, id: PollKey) -> Option<&Poll> {
        self.polls.get(id)
    }

    pub fn get_poll_mut(&mut self, id: PollKey) -> Option<&mut Poll> {
        self.polls.get_mut(id)
    }

    pub fn add_poll(&mut self, poll: Poll) -> PollKey {
        self.polls.insert(poll)
    }

    pub fn remove_poll(&mut self, id: PollKey) -> bool {
        self.polls.remove(id).is_some()
    }

    pub fn register_contact_channel(&mut self, channel: ContactChannel) -> ID {
        let id = self.contact_channel_id;
        self.contact_channel_id += 1;
        self.contact_channels.insert(id, channel);
        id
    }

    pub fn remove_contact_channel(&mut self, id: ID) {
        self.contact_channels.swap_remove(&id);
    }

    // The contact id a lounge or groupchat was registered under. Contact logs need this direction:
    // they are written wherever the graph changes, and those sites hold the lounge or gc key rather
    // than the id it was given.
    //
    // A scan, because the map is keyed the other way and there are few contact channels.
    pub fn contact_id_of(&self, channel: ContactChannel) -> Option<ID> {
        self.contact_channels
            .iter()
            .find(|(_, registered)| **registered == channel)
            .map(|(id, _)| *id)
    }

    pub fn add_channel(&mut self, channel: Channel) -> ChannelKey {
        self.channels.insert(channel)
    }

    pub fn remove_channel(&mut self, id: ChannelKey) -> bool {
        self.channels.remove(id).is_some()
    }

    pub fn get_channel(&self, id: ChannelKey) -> Option<&Channel> {
        self.channels.get(id)
    }

    pub fn get_channel_mut(&mut self, id: ChannelKey) -> Option<&mut Channel> {
        self.channels.get_mut(id)
    }

    pub fn add_lounge(&mut self, lounge: Lounge) -> LoungeKey {
        self.lounges.insert(lounge)
    }

    pub fn get_lounge(&self, id: LoungeKey) -> Option<&Lounge> {
        self.lounges.get(id)
    }

    pub fn get_lounge_mut(&mut self, id: LoungeKey) -> Option<&mut Lounge> {
        self.lounges.get_mut(id)
    }

    pub fn add_groupchat(&mut self, gc: Groupchat) -> GroupchatKey {
        self.groupchats.insert(gc)
    }

    pub fn get_groupchat(&self, id: GroupchatKey) -> Option<&Groupchat> {
        self.groupchats.get(id)
    }

    pub fn get_groupchat_mut(&mut self, id: GroupchatKey) -> Option<&mut Groupchat> {
        self.groupchats.get_mut(id)
    }

    pub fn add_prosecution(&mut self, prosecution: Prosecution) -> ProsecutionKey {
        self.prosecutions.insert(prosecution)
    }

    pub fn get_prosecution(&self, id: ProsecutionKey) -> Option<&Prosecution> {
        self.prosecutions.get(id)
    }

    pub fn get_prosecution_mut(&mut self, id: ProsecutionKey) -> Option<&mut Prosecution> {
        self.prosecutions.get_mut(id)
    }

    pub fn remove_prosecution(&mut self, id: ProsecutionKey) {
        self.prosecutions.remove(id);
    }

    pub fn add_bug(&mut self, bug: Bug) -> BugKey {
        self.bugs.insert(bug)
    }

    pub fn get_bug(&self, id: BugKey) -> Option<&Bug> {
        self.bugs.get(id)
    }

    pub fn get_bug_mut(&mut self, id: BugKey) -> Option<&mut Bug> {
        self.bugs.get_mut(id)
    }

    pub fn remove_bug(&mut self, id: BugKey) {
        self.bugs.remove(id);
    }

    pub fn add_kidnapping(&mut self, kidnapping: Kidnapping) -> KidnappingKey {
        self.kidnappings.insert(kidnapping)
    }

    pub fn get_kidnapping(&self, id: KidnappingKey) -> Option<&Kidnapping> {
        self.kidnappings.get(id)
    }

    pub fn get_kidnapping_mut(&mut self, id: KidnappingKey) -> Option<&mut Kidnapping> {
        self.kidnappings.get_mut(id)
    }

    pub fn remove_kidnapping(&mut self, id: KidnappingKey) {
        self.kidnappings.remove(id);
    }

    pub fn add_incarceration(&mut self, incarceration: Incarceration) -> IncarcerationKey {
        self.incarcerations.insert(incarceration)
    }

    pub fn get_incarceration(&self, id: IncarcerationKey) -> Option<&Incarceration> {
        self.incarcerations.get(id)
    }

    pub fn get_incarceration_mut(&mut self, id: IncarcerationKey) -> Option<&mut Incarceration> {
        self.incarcerations.get_mut(id)
    }

    pub fn remove_incarceration(&mut self, id: IncarcerationKey) {
        self.incarcerations.remove(id);
    }
}
