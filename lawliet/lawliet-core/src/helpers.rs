// Shared accessor/require API for actions. Not every helper has a caller yet — this is
// an intentional surface, so dead code is allowed module-wide rather than per item.
#![allow(dead_code)]

use lawliet_types::{action::ActionContext, command::CommandRecipient};

use crate::{
    Time,
    ability::Ability,
    action::{ActionActor, ActionError},
    actor::{
        Actor, ActorLinkType, ActorType, Organization, Player,
        modifier::{Modifier, Modifiers},
        state::State,
    },
    bug::Bug,
    channel::Channel,
    chargepool::ChargePool,
    command::{Command, CommandPayload, DeferredCommand},
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, IncarcerationKey,
        KidnappingKey, LoungeKey, NotebookKey, PassiveKey, PollKey, PollWeight, ProsecutionKey,
    },
    config::{
        ability::{AbilityConfig, AbilityIdentifier},
        role::{Role, RoleConfig},
        world::WorldChannelName,
    },
    engine::Engine,
    groupchat::Groupchat,
    incarceration::Incarceration,
    kidnapping::Kidnapping,
    lounge::Lounge,
    notebook::Notebook,
    passive::{Passive, PassiveType},
    poll::Poll,
    prosecution::Prosecution,
};

pub fn get_actor(eng: &Engine, actor_id: ActorKey) -> Result<&Actor, ActionError> {
    let target = eng
        .world
        .get_actor(actor_id)
        .ok_or(ActionError::ActorNotFound)?;
    Ok(target)
}
pub fn get_actor_mut(eng: &mut Engine, actor_id: ActorKey) -> Result<&mut Actor, ActionError> {
    let target = eng
        .world
        .get_actor_mut(actor_id)
        .ok_or(ActionError::ActorNotFound)?;
    Ok(target)
}
pub fn require_player(eng: &Engine, actor_id: ActorKey) -> Result<(), ActionError> {
    let target = get_actor(eng, actor_id)?;
    if !matches!(target.actor_type, ActorType::Player(_)) {
        Err(ActionError::ActorIsNotPlayer)
    } else {
        Ok(())
    }
}

pub fn actor_id(actor: &ActionActor) -> Option<ActorKey> {
    match actor {
        ActionActor::System | ActionActor::Admin => None,
        ActionActor::Player(id) => Some(*id),
        ActionActor::Organization(org_info) => Some(org_info.org_id),
    }
}

pub fn player_id(actor: &ActionActor) -> Option<ActorKey> {
    match actor {
        ActionActor::System | ActionActor::Admin => None,
        ActionActor::Player(id) => Some(*id),
        ActionActor::Organization(org_info) => Some(org_info.player_id),
    }
}

pub fn require_time_not_passed(eng: &Engine, t: Time) -> Result<(), ActionError> {
    if eng.is_future_timestamp(t) {
        Ok(())
    } else {
        Err(ActionError::TimeAlreadyPassed)
    }
}

pub fn require_alive(eng: &Engine, actor_id: ActorKey) -> Result<(), ActionError> {
    require_player(eng, actor_id)?;
    let actor = get_actor(eng, actor_id)?;
    if actor.states.contains(State::Dead) {
        return Err(ActionError::ActorIsDead);
    }
    Ok(())
}

pub fn require_dead(eng: &Engine, actor_id: ActorKey) -> Result<(), ActionError> {
    require_player(eng, actor_id)?;
    let actor = get_actor(eng, actor_id)?;
    if actor.states.contains(State::Dead) {
        return Ok(());
    }
    Err(ActionError::ActorIsAlive)
}

pub fn get_ability_mut(
    eng: &mut Engine,
    ability_id: AbilityKey,
) -> Result<&mut Ability, ActionError> {
    let target = eng
        .world
        .get_ability_mut(ability_id)
        .ok_or(ActionError::AbilityNotFound)?;
    Ok(target)
}

pub fn get_ability(eng: &Engine, ability_id: AbilityKey) -> Result<&Ability, ActionError> {
    let target = eng
        .world
        .get_ability(ability_id)
        .ok_or(ActionError::AbilityNotFound)?;
    Ok(target)
}

pub fn get_passive_mut(
    eng: &mut Engine,
    passive_id: PassiveKey,
) -> Result<&mut Passive, ActionError> {
    let target = eng
        .world
        .get_passive_mut(passive_id)
        .ok_or(ActionError::PassiveNotFound)?;
    Ok(target)
}

pub fn get_passive(eng: &Engine, passive_id: PassiveKey) -> Result<&Passive, ActionError> {
    let target = eng
        .world
        .get_passive(passive_id)
        .ok_or(ActionError::PassiveNotFound)?;
    Ok(target)
}

pub fn get_ability_config(
    eng: &Engine,
    ability: AbilityKey,
) -> Result<&AbilityConfig, ActionError> {
    let ability = get_ability(eng, ability)?;
    let target = eng.config.abilities.get(&AbilityIdentifier {
        name: ability.ability_name,
        variant: ability.variant,
    });
    if let Some(data) = target {
        Ok(data)
    } else {
        Err(ActionError::AbilityConfigNotFound)
    }
}

pub fn get_role_config(eng: &Engine, role: Role) -> Result<&RoleConfig, ActionError> {
    if let Some(role_config) = eng.config.roles.get(&role) {
        Ok(role_config)
    } else {
        Err(ActionError::RoleNotImplemented)
    }
}

pub fn actor_get_effective_passive(
    eng: &Engine,
    actor_id: ActorKey,
    check: impl Fn(&PassiveType) -> bool + Copy,
) -> Option<PassiveKey> {
    let actor_data = eng.world.get_actor(actor_id)?;
    for id in actor_data.passives.iter() {
        let passive = eng.world.get_passive(*id).unwrap(); // if the list is not accurate
        // to the passives that actually exist, then something is wrong with the engine and a crash
        // is warranted.
        if passive.ownership_struct.owner == Some(actor_id) && check(&passive.passive_type) {
            return Some(*id);
        }
    }
    for link in &actor_data.actor_links {
        if link.link_type == ActorLinkType::Passive {
            let other_actor = get_actor(eng, link.link_dest).unwrap();
            if let Some(found_id) = actor_get_effective_passive(eng, link.link_dest, check)
                && !other_actor.has_modifier(Modifier::DisablePassiveLinks)
            {
                return Some(found_id);
            };
        }
    }
    None
}

pub fn get_player(eng: &Engine, id: ActorKey) -> Result<&Player, ActionError> {
    let actor = get_actor(eng, id)?;
    if let ActorType::Player(player) = &actor.actor_type {
        Ok(player)
    } else {
        Err(ActionError::ActorIsNotPlayer)
    }
}

pub fn get_player_mut(eng: &mut Engine, id: ActorKey) -> Result<&mut Player, ActionError> {
    let actor = get_actor_mut(eng, id)?;
    if let ActorType::Player(player) = &mut actor.actor_type {
        Ok(player)
    } else {
        Err(ActionError::ActorIsNotPlayer)
    }
}

pub fn get_org_mut(eng: &mut Engine, id: ActorKey) -> Result<&mut Organization, ActionError> {
    let actor = get_actor_mut(eng, id)?;
    if let ActorType::Org(org) = &mut actor.actor_type {
        Ok(org)
    } else {
        Err(ActionError::ActorIsNotPlayer)
    }
}

pub fn get_org(eng: &Engine, id: ActorKey) -> Result<&Organization, ActionError> {
    let actor = get_actor(eng, id)?;
    if let ActorType::Org(org) = &actor.actor_type {
        Ok(org)
    } else {
        Err(ActionError::ActorIsNotPlayer)
    }
}

pub fn get_notebook(eng: &Engine, id: NotebookKey) -> Result<&Notebook, ActionError> {
    let notebook = eng.world.get_notebook(id);
    if let Some(notebook_data) = notebook {
        Ok(notebook_data)
    } else {
        Err(ActionError::NotebookNotFound)
    }
}

pub fn get_notebook_mut(eng: &mut Engine, id: NotebookKey) -> Result<&mut Notebook, ActionError> {
    let notebook = eng.world.get_notebook_mut(id);
    if let Some(notebook_data) = notebook {
        Ok(notebook_data)
    } else {
        Err(ActionError::NotebookNotFound)
    }
}

pub fn get_charge_pool(eng: &Engine, id: ChargePoolKey) -> Result<&ChargePool, ActionError> {
    let pool = eng.world.get_charge_pool(id);
    if let Some(data) = pool {
        Ok(data)
    } else {
        Err(ActionError::ChargePoolNotFound)
    }
}

pub fn get_charge_pool_mut(
    eng: &mut Engine,
    id: ChargePoolKey,
) -> Result<&mut ChargePool, ActionError> {
    let pool = eng.world.get_charge_pool_mut(id);
    if let Some(data) = pool {
        Ok(data)
    } else {
        Err(ActionError::ChargePoolNotFound)
    }
}

pub fn get_poll(eng: &Engine, id: PollKey) -> Result<&Poll, ActionError> {
    let poll = eng.world.get_poll(id);
    if let Some(data) = poll {
        Ok(data)
    } else {
        Err(ActionError::PollDoesntExist)
    }
}

pub fn get_poll_mut(eng: &mut Engine, id: PollKey) -> Result<&mut Poll, ActionError> {
    let poll = eng.world.get_poll_mut(id);
    if let Some(data) = poll {
        Ok(data)
    } else {
        Err(ActionError::PollDoesntExist)
    }
}

// return 0 for organizations, return 1 for normal players, return some other number if they have
// the vote amplification passive
pub fn get_voter_weight(eng: &Engine, id: ActorKey) -> PollWeight {
    get_actor(eng, id).expect("Expected a valid actor ID");
    if get_player(eng, id).is_ok() {
        let passive_id = actor_get_effective_passive(eng, id, |passive_type| {
            matches!(
                passive_type,
                PassiveType::VoteAmplification { multiplier: _ }
            )
        });
        if let Some(id) = passive_id {
            let passive = get_passive(eng, id).expect("Expected passive to exist");
            let PassiveType::VoteAmplification { multiplier: val } = passive.passive_type else {
                unreachable!();
            };
            val
        } else {
            1
        }
    } else {
        0
    }
}

pub fn get_channel(eng: &Engine, id: ChannelKey) -> Result<&Channel, ActionError> {
    let channel = eng.world.get_channel(id);
    if let Some(data) = channel {
        Ok(data)
    } else {
        Err(ActionError::ChannelDoesntExist)
    }
}

pub fn get_channel_mut(eng: &mut Engine, id: ChannelKey) -> Result<&mut Channel, ActionError> {
    let channel = eng.world.get_channel_mut(id);
    if let Some(data) = channel {
        Ok(data)
    } else {
        Err(ActionError::ChannelDoesntExist)
    }
}

pub fn get_world_channel(eng: &Engine, name: WorldChannelName) -> Result<&Channel, ActionError> {
    let &id = eng
        .world
        .world_channel_map
        .get(&name)
        .ok_or(ActionError::ChannelDoesntExist)?;
    get_channel(eng, id)
}

pub fn get_world_channel_id(
    eng: &Engine,
    name: WorldChannelName,
) -> Result<ChannelKey, ActionError> {
    eng.world
        .world_channel_map
        .get(&name)
        .copied()
        .ok_or(ActionError::ChannelDoesntExist)
}

pub fn get_lounge(eng: &Engine, id: LoungeKey) -> Result<&Lounge, ActionError> {
    let lounge = eng.world.get_lounge(id);
    if let Some(data) = lounge {
        Ok(data)
    } else {
        Err(ActionError::LoungeDoesntExist)
    }
}

pub fn get_lounge_mut(eng: &mut Engine, id: LoungeKey) -> Result<&mut Lounge, ActionError> {
    let lounge = eng.world.get_lounge_mut(id);
    if let Some(data) = lounge {
        Ok(data)
    } else {
        Err(ActionError::LoungeDoesntExist)
    }
}

pub fn get_gc(eng: &Engine, id: GroupchatKey) -> Result<&Groupchat, ActionError> {
    let gc = eng.world.get_groupchat(id);
    if let Some(data) = gc {
        Ok(data)
    } else {
        Err(ActionError::GroupchatDoesntExist)
    }
}

pub fn get_gc_mut(eng: &mut Engine, id: GroupchatKey) -> Result<&mut Groupchat, ActionError> {
    let gc = eng.world.get_groupchat_mut(id);
    if let Some(data) = gc {
        Ok(data)
    } else {
        Err(ActionError::GroupchatDoesntExist)
    }
}

pub fn get_bug(eng: &Engine, id: BugKey) -> Result<&Bug, ActionError> {
    eng.world.get_bug(id).ok_or(ActionError::BugNotFound)
}

pub fn get_bug_mut(eng: &mut Engine, id: BugKey) -> Result<&mut Bug, ActionError> {
    eng.world.get_bug_mut(id).ok_or(ActionError::BugNotFound)
}

pub fn get_prosecution(eng: &Engine, id: ProsecutionKey) -> Result<&Prosecution, ActionError> {
    eng.world
        .get_prosecution(id)
        .ok_or(ActionError::ProsecutionNotFound)
}

pub fn get_prosecution_mut(
    eng: &mut Engine,
    id: ProsecutionKey,
) -> Result<&mut Prosecution, ActionError> {
    eng.world
        .get_prosecution_mut(id)
        .ok_or(ActionError::ProsecutionNotFound)
}

pub fn get_kidnapping(eng: &Engine, id: KidnappingKey) -> Result<&Kidnapping, ActionError> {
    eng.world
        .get_kidnapping(id)
        .ok_or(ActionError::KidnappingNotFound)
}

pub fn actor_owns_ability(eng: &Engine, actor: &ActionActor, ability_id: AbilityKey) -> bool {
    let Some(acting_id) = actor_id(actor) else {
        return false;
    };
    get_ability(eng, ability_id)
        .ok()
        .and_then(|a| a.ownership_struct.owner)
        .is_some_and(|owner| owner == acting_id)
}

pub fn get_incarceration(
    eng: &Engine,
    id: IncarcerationKey,
) -> Result<&Incarceration, ActionError> {
    eng.world
        .get_incarceration(id)
        .ok_or(ActionError::IncarcerationNotFound)
}

pub fn get_incarceration_mut(
    eng: &mut Engine,
    id: IncarcerationKey,
) -> Result<&mut Incarceration, ActionError> {
    eng.world
        .get_incarceration_mut(id)
        .ok_or(ActionError::IncarcerationNotFound)
}

pub fn get_kidnapping_mut(
    eng: &mut Engine,
    id: KidnappingKey,
) -> Result<&mut Kidnapping, ActionError> {
    eng.world
        .get_kidnapping_mut(id)
        .ok_or(ActionError::KidnappingNotFound)
}

pub fn require_not_defendant(eng: &Engine, actor_id: ActorKey) -> Result<(), ActionError> {
    let actor = get_actor(eng, actor_id)?;
    if actor.has_state(State::Custody) {
        Err(ActionError::AlreadyADefendant)
    } else {
        Ok(())
    }
}

pub fn cmd_all_deferred(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    cmd: Command,
    blocking_modifiers: Modifiers,
    include_base: bool,
    include_system: bool,
    mutate: bool,
) {
    // Queuing deferred commands mutates engine state, so it must only happen on the
    // execute pass. Without the mutate gate the validate pass queues them too, and
    // every deferred command ends up delivered twice.
    if mutate {
        let player_ids: Vec<ActorKey> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, actor)| {
                matches!(actor.actor_type, ActorType::Player(_)).then_some(id)
            })
            .collect();
        for id in player_ids {
            let payload = CommandPayload {
                timestamp: eng.time,
                recipient: CommandRecipient::Actor(id),
                cmd: cmd.clone(),
            };
            let def_cmd = DeferredCommand {
                payload: payload.clone(),
                blocking_modifiers,
            };
            eng.deferred_commands.push(def_cmd);
        }
    }
    // System (admin) receives the event immediately and unredacted: admin isn't
    // subject to per-player blocking, so this is not deferred. Kept separate from
    // include_base so a deceptive event can be fed to players while admin sees truth.
    if include_system {
        ctx.push_cmd(cmd.clone(), CommandRecipient::System, eng.time);
    }
    if include_base {
        ctx.push_cmd(cmd, CommandRecipient::BasePlayer, eng.time);
    }
}
