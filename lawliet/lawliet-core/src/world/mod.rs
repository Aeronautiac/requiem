use std::{
    collections::{BTreeMap, btree_map::Entry},
    rc::Rc,
};

use indexmap::IndexMap;
use slotmap::SlotMap;

use crate::{
    ability::Ability,
    actor::{Actor, ActorType, Player, organization::LeadershipStruct},
    bug::Bug,
    channel::Channel,
    chargepool::ChargePool,
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, IncarcerationKey,
        KidnappingKey, LoungeKey, NotebookKey, PassiveKey, PollKey, ProsecutionKey,
    },
    config::{
        actor::organization::OrganizationName,
        role::Role,
        world::{WorldChannelName, WorldChargePoolName},
    },
    groupchat::Groupchat,
    incarceration::Incarceration,
    kidnapping::Kidnapping,
    lounge::Lounge,
    notebook::Notebook,
    passive::Passive,
    poll::Poll,
    prosecution::Prosecution,
};

#[derive(Debug)]
pub enum WorldError {
    DuplicateName,
}

#[derive(Debug)]
pub struct World {
    pub blackout: bool,
    pub actors: SlotMap<ActorKey, Actor>,
    pub player_names: BTreeMap<Rc<str>, ActorKey>, // a map of true names to actor ids
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
}

impl World {
    pub fn new() -> Self {
        World {
            blackout: false,
            actors: SlotMap::with_key(),
            abilities: SlotMap::with_key(),
            notebooks: SlotMap::with_key(),
            player_names: BTreeMap::new(),
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
        }
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
