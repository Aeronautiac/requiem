// Shared accessor/require API for actions. Not every helper has a caller yet — this is
// an intentional surface, so dead code is allowed module-wide rather than per item.
#![allow(dead_code)]

use indexmap::IndexSet;
use lawliet_types::{
    action::ActionContext,
    command::{Command, CommandRecipient},
    organization::OrgMemberView,
    world::WorldPhase,
};
use smallvec::SmallVec;

use crate::{
    Time,
    ability::Ability,
    action::{ActionActor, ActionError},
    actor::{
        Actor, ActorLinkType, ActorType, Organization, Player,
        modifier::Modifier,
        state::{State, Status, Statuses},
    },
    bug::{Bug, BugSource},
    channel::{Channel, ProfileOwners},
    chargepool::ChargePool,
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
    passive::{ContactLog, ContactLogType, Passive, PassiveType},
    poll::Poll,
    prosecution::Prosecution,
    viewport::ViewportKind,
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

// Refuse anything that is PLAY while the world is not running.
//
// Not a blanket gate on players: a world in setup is a real, populated place, and a player may talk
// in any channel they can already see — ordinary channel permission answers that, with no phase
// rule needed. What waits is exactly two things: you cannot use abilities, and you cannot use or
// pass a notebook.
//
// Abilities collapse to one gate because contact IS an ability, and group chats, lounges, polls and
// prosecutions all descend from one — blocking the root blocks the branches.
//
// Checks FOR Running rather than against Setup, so a phase added on the other end of the game
// (post-game, ended) is refused by default instead of silently counting as play.
pub fn require_running(eng: &Engine) -> Result<(), ActionError> {
    if eng.world.phase != WorldPhase::Running {
        return Err(ActionError::GameNotStarted);
    }
    Ok(())
}

pub fn require_alive(eng: &Engine, actor_id: ActorKey) -> Result<(), ActionError> {
    require_player(eng, actor_id)?;
    let actor = get_actor(eng, actor_id)?;
    if actor.states.contains(State::Dead) {
        return Err(ActionError::ActorIsDead);
    }
    Ok(())
}

pub fn require_no_blackout(eng: &Engine) -> Result<(), ActionError> {
    if eng.world.blackout {
        return Err(ActionError::WorldIsBlackedOut);
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

// Whether `actor_id` effectively possesses `passive_id` — owning it outright, or reaching it
// through an ActorLinkType::Passive link to someone who does.
//
// This is the inverse of actor_get_effective_passive, which searches by TYPE and stops at the first
// match. Passive VIEWPORT membership is what needs this direction: two passives of the same type can
// coexist, so "does this actor reach passive P" cannot be answered by asking "what passive of P's
// type does this actor reach".
//
// Follows the same traversal, so it inherits the same shape: a Passive link cycle would recurse
// forever, which no configuration currently produces.
pub fn actor_reaches_passive(eng: &Engine, actor_id: ActorKey, passive_id: PassiveKey) -> bool {
    let Some(actor_data) = eng.world.get_actor(actor_id) else {
        return false;
    };

    if actor_data.passives.contains(&passive_id) {
        return true;
    }

    actor_data.actor_links.iter().any(|link| {
        link.link_type == ActorLinkType::Passive
            && eng
                .world
                .get_actor(link.link_dest)
                .is_some_and(|other| !other.has_modifier(Modifier::DisablePassiveLinks))
            && actor_reaches_passive(eng, link.link_dest, passive_id)
    })
}

// Write a contact to every contact-log passive entitled to see it, addressed to that passive's own
// viewport.
//
// `initiator` is the actor who actually performed the contact, and is used for nothing but the
// LogNullification check — it never reaches the log, which carries displays only. The two come
// apart for a fabricated lounge, where the displays name a pair that never spoke.
//
// None when the engine itself did it: there is no actor to be off the record, so it is always
// logged, and the log names ActorDisplay::System as the contactor.
pub fn cmd_contact_log(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    initiator: Option<ActorKey>,
    log: ContactLog,
) {
    if initiator.is_some_and(|id| {
        get_actor(eng, id).is_ok_and(|actor| actor.has_modifier(Modifier::LogNullification))
    }) {
        return;
    }

    // The record lives on the world's three log viewports, not on any passive, so a contact is
    // written whether or not anyone holds the passive to read it yet. Full takes every contact;
    // Even and Odd split on the contact-id parity. A reader is entered into the matching viewport
    // by the UpdateContactLogViewports sweep and backfills the history on the way in.
    let targets: SmallVec<[(ContactLogType, ViewportKey); 3]> = eng
        .world
        .contact_log_viewports()
        .filter(|(kind, _)| kind.covers(log.contact_id))
        .collect();

    let time = eng.time;
    for (kind, viewport) in targets {
        ctx.push_cmd(
            Command::AddContactLog { kind, log },
            CommandRecipient::Viewport(viewport),
            time,
        );
    }
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

// Address a command to a channel, and to that channel's record when it belongs there. Most "who
// should see this" questions in the comms layer reduce to this one, because notebooks, lounges,
// groupchats and orgs all hang off a backing channel and inherit its answer.
//
// The membership viewport always receives it — the room witnessed whatever this is, and a monotonic
// client cannot be told otherwise afterwards. The log viewport is the separate thing a tap-in reads
// back, so being off the record only ever means staying out of it, never unsaying it.
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
    // Whether somebody in the room would have WITNESSED this. False for the plumbing that makes a
    // channel work — registrations, membership bookkeeping, the loggable flag itself — which nobody
    // witnesses and which is therefore no part of the record.
    witnessed: bool,
    // Whoever is doing it, used for nothing but the LogNullification check. None when the engine
    // itself did it: there is no actor to be off the record, so it is always logged.
    initiator: Option<ActorKey>,
) {
    if !ctx.mutate {
        return;
    }
    let channel = eng
        .world
        .get_channel(channel_id)
        .expect("channel addressed by a command does not exist: engine invariant violated");
    let (viewport, log, loggable) = (channel.viewport, channel.log, channel.loggable);

    // Nullification covers events and not just messages, because the modifier would be worth
    // nothing otherwise: taking yourself off the record is pointless if one Kira attempt puts you
    // back on it.
    let nullified = initiator.is_some_and(|id| {
        get_actor(eng, id).is_ok_and(|actor| actor.has_modifier(Modifier::LogNullification))
    });

    if witnessed && loggable && !nullified {
        ctx.push_cmd(cmd.clone(), CommandRecipient::Log(log), eng.time);
    }
    ctx.push_cmd(cmd, CommandRecipient::Viewport(viewport), eng.time);
}

// Tell everyone who can see a channel which names are in it.
//
// Synchronised. The whole visible set, directed at each viewer.
// Addressing it to the channel's viewport instead would be a leak. A viewport hands its whole history to anyone who enters,
// so a late arrival would be told every name the channel has ever held, including things like old death
// note owners, and that's not good.
//
// Call it after anything that changes which profiles exist, what they show, what they permit, or
// who can see them. It is idempotent, so calling it when nothing moved costs a few small commands
// and says nothing new.
pub fn cmd_channel_roster(eng: &mut Engine, ctx: &mut ActionContext, channel_id: ChannelKey) {
    let Ok(channel) = get_channel(eng, channel_id) else {
        return;
    };
    let profiles = channel.visible_profiles().into_vec();
    let viewers = channel.viewers();

    for viewer in viewers {
        ctx.push_cmd(
            Command::ChannelRoster {
                channel_id,
                profiles: profiles.clone(),
            },
            CommandRecipient::Actor(viewer),
            eng.time,
        );
    }

    // Admin observes every channel, member or not. It gets the same visible roster the room sees,
    // plus the one thing the room never learns: who is behind each of those names. Both ride the
    // System recipient and key by profile_id, so the ownership map lines up with the roster.
    let owners: Vec<ProfileOwners> = channel
        .profiles
        .iter()
        .filter(|(_, profile)| profile.visible)
        .map(|(id, profile)| ProfileOwners {
            profile_id: id,
            owners: profile.ownership.owners().into_vec(),
        })
        .collect();

    ctx.push_cmd(
        Command::ChannelRoster {
            channel_id,
            profiles,
        },
        CommandRecipient::System,
        eng.time,
    );
    ctx.push_cmd(
        Command::ProfileOwnership { channel_id, owners },
        CommandRecipient::System,
        eng.time,
    );
}

// Tell an actor which names in a channel are theirs to speak as.
//
// Directed, because it is per-actor by nature: the room is told which names exist at all, and each
// member is told separately which of them they hold. Whether they can READ the channel is a
// separate matter answered by its viewport.
pub fn cmd_profile_access(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    channel_id: ChannelKey,
    actor_id: ActorKey,
) {
    let Ok(channel) = get_channel(eng, channel_id) else {
        return;
    };
    let profiles = channel.accessible_profiles(actor_id).into_vec();
    ctx.push_cmd(
        Command::ProfileAccess {
            channel_id,
            profiles,
        },
        CommandRecipient::Actor(actor_id),
        eng.time,
    );
}

// Tell an actor what states they are currently in.
//
// The whole set rather than the change: a client that ever missed one would otherwise stay wrong
// forever, and the set is a handful of bits.
//
// Directed to the actor alone: their own raw state set, including states others never see (like
// UnderTheRadar). What OTHERS may see is the curated projection in cmd_actor_status, not this.
// Presence therefore does not gate this: a player who has just lost presence still learns they are
// dead, because this is addressed to them and not to a viewport.
pub fn cmd_actor_state(eng: &mut Engine, ctx: &mut ActionContext, actor_id: ActorKey) {
    let Ok(actor) = get_actor(eng, actor_id) else {
        return;
    };
    let state = actor.states;
    ctx.push_cmd(
        Command::ActorState { state, actor_id },
        owner_view_recipient(eng, actor_id),
        eng.time,
    );
}

// True while an enabled bug is trained on this actor for a reason the public Status doesn't already
// tell. A custody bug is excluded: it is incidental to being held, and the `Custody` flag already
// carries that — surfacing `Bugged` too would just restate custody. A bug archived (disabled) but
// kept in the world for its history no longer counts.
pub fn actor_is_bugged(eng: &Engine, actor_id: ActorKey) -> bool {
    eng.world.bugs.values().any(|bug| {
        bug.target_id == actor_id && bug.enabled && !matches!(bug.source, BugSource::Custody)
    })
}

// Broadcast the public projection of an actor's condition on the world-data viewport — the
// companion to cmd_actor_state, which stays private to the actor. Call this wherever a contributing
// fact moves: a relevant state, a bug landing or being archived, or a blackout toggling.
//
// Only players are projected; orgs have no status here. Emits only on a genuine change (diffed
// against the actor's last_status) and stores the projection back, so the next call knows what the
// world has already been told.
//
// Blackout blur: a presence-removing state (one carrying NoPresence) is withheld while the world is
// dark UNLESS it was already public before the blackout — a fact once broadcast is never retracted.
// Whatever is withheld surfaces only as `missing`, so the world learns someone is gone, not why.
pub fn cmd_actor_status(eng: &mut Engine, ctx: &mut ActionContext, actor_id: ActorKey) {
    let Ok(actor) = get_actor(eng, actor_id) else {
        return;
    };
    if !matches!(actor.actor_type, ActorType::Player(_)) {
        return;
    }

    let blackout = eng.world.blackout;
    let last = actor.last_status;

    let removes_presence = |state: State| {
        eng.config
            .state_modifiers
            .get(&state)
            .is_some_and(|m| m.contains(Modifier::NoPresence))
    };

    let mut status = Statuses::empty();
    if actor_is_bugged(eng, actor_id) {
        status |= Status::Bugged;
    }

    // A presence-removing state hidden by this blackout is withheld and folds into `Missing`; one
    // the world already knew (`known`) is shown through the blackout instead of being retracted.
    let mut missing = false;
    {
        let mut project = |flag: Status, state: State, known: bool| {
            if !actor.states.contains(state) {
                return;
            }
            if blackout && removes_presence(state) && !known {
                missing = true;
            } else {
                status |= flag;
            }
        };

        project(Status::Dead, State::Dead, last.contains(Status::Dead));
        project(
            Status::Incarcerated,
            State::Incarcerated,
            last.contains(Status::Incarcerated),
        );
        project(
            Status::Kidnapped,
            State::Kidnapped,
            last.contains(Status::Kidnapped),
        );
        project(
            Status::Custody,
            State::Custody,
            last.contains(Status::Custody),
        );
        project(Status::Ipp, State::Ipp, last.contains(Status::Ipp));
    }

    if missing {
        status |= Status::Missing;
    }

    if status != last {
        ctx.push_cmd(
            Command::ActorStatus { actor_id, status },
            CommandRecipient::Viewport(eng.world.data_viewport),
            eng.time,
        );
    }

    if ctx.mutate
        && let Ok(actor) = get_actor_mut(eng, actor_id)
    {
        actor.last_status = status;
    }
}

// Broadcast which of an org's members currently COUNT toward its ability member requirements — the
// present subset of the roster — on the org's channel viewport. The org counterpart to
// cmd_actor_status: emitted only on a genuine change (diffed against last_effective) and stored
// back, so sweeping every org on every Update is silent when nothing has moved.
//
// "Present" is `!NoPresence`, exactly the predicate SystemUseOrgAbility counts with, so the list
// never disagrees with the gate it describes. Not witnessed — this is membership bookkeeping the
// room does not "see", so it never reaches the record.
pub fn cmd_org_effective_members(eng: &mut Engine, ctx: &mut ActionContext, org_id: ActorKey) {
    let Ok(org) = get_org(eng, org_id) else {
        return;
    };
    let channel_id = org.channel_id;
    let member_ids: SmallVec<[ActorKey; 16]> = org.members.keys().copied().collect();

    let present: IndexSet<ActorKey> = member_ids
        .into_iter()
        .filter(|id| get_actor(eng, *id).is_ok_and(|a| !a.has_modifier(Modifier::NoPresence)))
        .collect();

    if get_org(eng, org_id).is_ok_and(|o| o.last_effective == present) {
        return;
    }

    let members: Vec<ActorKey> = present.iter().copied().collect();
    cmd_channel(
        eng,
        ctx,
        Command::OrgEffectiveMembers { org_id, members },
        channel_id,
        false,
        None,
    );

    if ctx.mutate
        && let Ok(org) = get_org_mut(eng, org_id)
    {
        org.last_effective = present;
    }
}

// Tell a member whether they are an OG of an org, and mirror it to System for the admin inspector.
//
// Personal info, addressed exactly like a role or a true name: yours alone. The rest of the org
// hears nothing — its roster is a separate stream on the org's channel that says only who is in it,
// so who an org may spend is something each member knows about themselves and must be told about
// anyone else.
pub fn cmd_og_status(
    eng: &Engine,
    ctx: &mut ActionContext,
    org_id: ActorKey,
    target_id: ActorKey,
    og: bool,
) {
    for recipient in [CommandRecipient::Actor(target_id), CommandRecipient::System] {
        ctx.push_cmd(
            Command::OgStatus {
                target_id,
                org_id,
                og,
            },
            recipient,
            eng.time,
        );
    }
}

// Tell a notebook's original owner whether their book is a decoy, mirrored to System for the admin
// inspector. Only the ORIGINAL owner is entitled to know — not whoever merely holds it now. Someone
// who borrowed it, or who came to own it by killing the original owner and inheriting the book,
// deduces the decoy from a write that fails to kill, or never learns at all. If no original owner
// is set yet, only System hears it.
//
// The one statement of the fake status, shared by the two events that need to make it: the give
// that establishes the original owner, and any later change to the flag. Reads the current value
// off the notebook rather than taking it, so a caller cannot state one thing and store another.
pub fn cmd_notebook_fake_status(eng: &Engine, ctx: &mut ActionContext, notebook_id: NotebookKey) {
    let Ok(notebook) = get_notebook(eng, notebook_id) else {
        return;
    };
    let fake = notebook.fake;

    for recipient in [
        notebook.original_owner.map(CommandRecipient::Actor),
        Some(CommandRecipient::System),
    ]
    .into_iter()
    .flatten()
    {
        ctx.push_cmd(
            Command::NotebookFakeStatus { notebook_id, fake },
            recipient,
            eng.time,
        );
    }
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

// Bring a viewport into existence: allocate it and announce what it belongs to, on itself.
//
// Every viewport an action owns is opened here, so the announcement cannot be forgotten and cannot
// name a kind other than the one it was allocated with. It heads that viewport's history, which is
// what lets it be the first thing anyone entering later is handed.
//
// Callers are the actions that own the object, per the lifetime rule in the viewport module, and
// they call this on the mutate pass only — a validate pass allocates nothing, and the key it hands
// back would be a real slot in a world that is about to be thrown away.
pub fn open_viewport(eng: &mut Engine, ctx: &mut ActionContext, kind: ViewportKind) -> ViewportKey {
    let viewport = eng.world.add_viewport(kind);
    ctx.push_cmd(
        Command::MapViewport { viewport, kind },
        CommandRecipient::Viewport(viewport),
        eng.time,
    );
    viewport
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
    if !eng.world.viewport_grant(viewport, actor) {
        return;
    }
    ctx.push_cmd(
        Command::EnterViewport { viewport, actor },
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
            Command::EnterViewport { viewport, actor },
            CommandRecipient::Actor(actor),
            time,
        );
    }
}

// A world event: something that HAPPENED in the world at large, which anyone present learns
// about. Addressed to the world-events viewport, so a player who is absent — or a world under
// blackout — simply isn't a member, and receives it (in order, with everything else it missed)
// when access returns.
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
        CommandRecipient::Viewport(eng.world.events_viewport),
        eng.time,
    );
}

// A structural fact about the world rather than an announcement of something that happened: who
// exists, what day it is, whether the lights are on. Same audience as cmd_world_event and the same
// backfill on re-entry — the difference is that a blackout does not take this away.
//
// The line between the two is what a player is entitled to know while the world is dark. They may
// work out from the rosters and the channels that somebody is gone; they are not told that they
// died, or why, until the blackout lifts.
pub fn cmd_world_data(eng: &mut Engine, ctx: &mut ActionContext, cmd: Command) {
    ctx.push_cmd(
        cmd,
        CommandRecipient::Viewport(eng.world.data_viewport),
        eng.time,
    );
}

pub fn member_views(eng: &Engine, p_id: ActorKey) -> Vec<(ActorKey, OrgMemberView)> {
    let orgs: SmallVec<[ActorKey; 8]> = eng
        .world
        .actors
        .iter()
        .filter(|(id, actor)| {
            matches!(actor.actor_type, ActorType::Org(_))
                && get_org(eng, *id)
                    .expect("existence already validated")
                    .has_member(p_id)
        })
        .map(|(id, _actor)| id)
        .collect();

    orgs.into_iter()
        .map(|id| {
            let org = get_org(eng, id).expect("already validated");
            (
                id,
                org.member_view(p_id).expect("membership already validated"),
            )
        })
        .collect()
}
