// Shared accessor/require API for actions. Not every helper has a caller yet — this is
// an intentional surface, so dead code is allowed module-wide rather than per item.
#![allow(dead_code)]

use indexmap::IndexSet;
use lawliet_types::{action::ActionContext, command::CommandRecipient};

use crate::{
    Time,
    ability::Ability,
    action::{ActionActor, ActionError},
    actor::{
        Actor, ActorLinkType, ActorType, Organization, Player, modifier::Modifier, state::State,
    },
    bug::Bug,
    channel::Channel,
    chargepool::ChargePool,
    command::Command,
    common::{
        AbilityKey, ActorKey, BugKey, ChannelKey, ChargePoolKey, GroupchatKey, IncarcerationKey,
        KidnappingKey, LoungeKey, NotebookKey, PassiveKey, PollKey, PollWeight, ProsecutionKey,
        ViewportKey,
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

// Address a command to a channel. Most "who should see this" questions in the comms layer
// reduce to this one, because notebooks, lounges, groupchats and orgs all hang off a backing
// channel and inherit its answer.
//
// The mutate gate is not an optimization, it is the point. Resolving a recipient on the validate
// pass is meaningless — push_cmd discards the command anyway — and actively dangerous, because
// an action that creates a channel holds ChannelKey::default() on that pass and would "fail"
// validation on a channel that only doesn't exist yet. That is the two-pass divergence hazard
// the viewport design flagged, and keeping resolution behind the same gate as emission is what
// closes it.
//
// Past that gate a missing channel has no benign reading, so it panics rather than dropping the
// command. Returning quietly would leave every client short of state with nothing to show for
// it at the point of failure — the engine's contract is to die on an inconsistent world and be
// rebuilt from the action log.
pub fn cmd_channel(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    cmd: Command,
    channel_id: ChannelKey,
) {
    if !ctx.mutate {
        return;
    }
    let viewport = eng
        .world
        .get_channel(channel_id)
        .expect("channel addressed by a command does not exist: engine invariant violated")
        .viewport;
    ctx.push_cmd(cmd, CommandRecipient::Viewport(viewport), eng.time);
}

// Where a view of something an ACTOR owns should be addressed.
//
// A player owns their abilities and passives alone, so it goes to them. An org's are seen by
// everyone in the org — which is now just "everyone who can see the org's channel", so it goes
// to that channel's viewport. Addressing an org actor directly was the old way of expressing
// "everyone in this org"; nothing is attached to an org actor's stream any more, because no
// client holds an org as a view.
//
// Callers must already be past their mutate gate: an org whose channel is missing is an
// inconsistent world, same rule as cmd_channel.
pub fn owner_view_recipient(eng: &Engine, owner_id: ActorKey) -> CommandRecipient {
    match get_org(eng, owner_id) {
        Ok(org) => CommandRecipient::Viewport(
            get_channel(eng, org.channel_id)
                .expect("org channel does not exist: engine invariant violated")
                .viewport,
        ),
        Err(_) => CommandRecipient::Actor(owner_id),
    }
}

// Grant an actor access to a viewport and announce it. No-ops if they already had access, so
// callers may be as liberal as they like about calling it — but panics if the viewport is gone
// (see World::viewport_grant). "Already a member" and "the viewport no longer exists" are
// different answers and only the first one is benign.
pub fn grant_viewport(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    viewport: ViewportKey,
    actor: ActorKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    let Some(kind) = eng.world.viewport_grant(viewport, actor) else {
        return;
    };
    ctx.push_cmd(
        Command::EnterViewport {
            viewport,
            actor,
            kind,
        },
        CommandRecipient::Actor(actor),
        eng.time,
    );
}

// Revoke an actor's access and announce it. Nothing already delivered is retracted — this only
// stops what comes next.
pub fn revoke_viewport(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    viewport: ViewportKey,
    actor: ActorKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    if !eng.world.viewport_revoke(viewport, actor) {
        return;
    }
    ctx.push_cmd(
        Command::ExitViewport { viewport, actor },
        CommandRecipient::Actor(actor),
        eng.time,
    );
}

// Point a viewport at a freshly computed membership set, announcing only genuine transitions.
// For visibility rules evaluated by recomputing the whole answer rather than by delta. Panics
// if the viewport is gone (see World::viewport_grant); note that teardown paths must sync the
// membership to empty BEFORE freeing the viewport, never after.
pub fn sync_viewport(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    viewport: ViewportKey,
    members: IndexSet<ActorKey>,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    let kind = eng
        .world
        .get_viewport(viewport)
        .expect("viewport does not exist: engine invariant violated")
        .kind;
    let diff = eng.world.viewport_set_members(viewport, members);
    let time = eng.time;
    for actor in diff.exited {
        ctx.push_cmd(
            Command::ExitViewport { viewport, actor },
            CommandRecipient::Actor(actor),
            time,
        );
    }
    for actor in diff.entered {
        ctx.push_cmd(
            Command::EnterViewport {
                viewport,
                actor,
                kind,
            },
            CommandRecipient::Actor(actor),
            time,
        );
    }
}

// Resync the presence viewport: every player who currently has presence, and nobody else.
//
// This is the whole of what the deferred-command queue used to do. That queue held a copy of
// every world event per absent player and re-tested `blocking_modifiers` on each flush; all
// seven of its call sites blocked on Modifier::NoPresence, so presence was the only condition it
// ever expressed. Membership of one viewport says the same thing, and re-entry backfills the
// backlog in order instead of a queue replaying it.
//
// Call it wherever presence can change. That is exactly AddState, RemoveState and player
// creation: modifiers are only ever written by add_state/remove_state, so nothing else can move
// an actor across this line.
pub fn sync_presence(eng: &mut Engine, ctx: &mut ActionContext, mutate: bool) {
    if !mutate {
        return;
    }
    let present: IndexSet<ActorKey> = eng
        .world
        .actors
        .iter()
        .filter_map(|(id, actor)| {
            (matches!(actor.actor_type, ActorType::Player(_))
                && !actor.has_modifier(Modifier::NoPresence))
            .then_some(id)
        })
        .collect();
    let viewport = eng.world.presence_viewport;
    sync_viewport(eng, ctx, viewport, present, mutate);
}

// A world event: something that happened in the world at large, which anyone present learns
// about. Addressed to the presence viewport, so a player who is absent simply isn't a member
// and receives it (in order, with everything else they missed) when presence returns.
//
// There is deliberately NO separate System copy. Admin reads every viewport, so it already gets
// this — and a mirror carrying different content would be actively worse than none: it would show
// admin the truth INSTEAD of the deception, leaving them unable to see what the players were
// actually told. Deception belongs in a second command carrying the truth, addressed to System,
// so admin receives both and can see that they differ. One command, one meaning, as everywhere
// else in this protocol.
pub fn cmd_world_event(eng: &mut Engine, ctx: &mut ActionContext, cmd: Command) {
    ctx.push_cmd(
        cmd,
        CommandRecipient::Viewport(eng.world.presence_viewport),
        eng.time,
    );
}
