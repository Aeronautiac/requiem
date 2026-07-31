/*
* SYSTEM ACTION
* Re-derive who is on the captors' side of every active kidnapping, and seat them.
*
* A sweep over KIDNAPPINGS rather than over channels. Who the captors are is a property of what
* started the kidnapping, which is not a question a channel can answer about itself — and it is the
* part that moves, since who is taking part in an org changes while somebody is still being held.
*
* The two kinds want opposite things, and both are the point of their kind:
*
*   Anonymous  one mask, worn by all of them. The victim sees somebody and never how many, and no
*              amount of watching who says what pulls them apart. Its permissions are fixed,
*              necessarily: a name worn by several people has no single owner whose standing could
*              be consulted.
*   Public     the face of the thing is visible from the start, and everyone standing behind it is
*              hidden until they speak. Taking someone in the open still lets you stay at the back
*              of the room.
*
* The victim is seated by CreateKidnapping and is no business of this sweep.
*/

use lawliet_types::{
    action::{ActionError, ActionResponse},
    actor::ActorDisplay,
    channel::{ChannelPerm, FixedPolicy, PermUpdatePolicy, PresencePolicy},
};
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResult, AddProfile,
        CreateAndGiveProfile, RemoveFromChannel, SetProfileAccess,
    },
    actor::ActorType,
    common::{ActorKey, ChannelKey, KidnappingKey, ProfileKey, Version},
    engine::Engine,
    helpers::{get_ability, get_actor, get_channel},
    kidnapping::{KidnappingSource, KidnappingType},
};

pub use crate::action::{UpdateKidnappings, UpdateKidnappingsResponse};

struct Sweep {
    id: KidnappingKey,
    channel_id: ChannelKey,
    victim: ActorKey,
    kidnapping_type: KidnappingType,
    mask: Option<ProfileKey>,
    captors: SmallVec<[ActorKey; 8]>,
}

impl ActionInterface for UpdateKidnappings {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let sweeps: SmallVec<[Sweep; 8]> = eng
            .world
            .kidnappings
            .iter()
            .map(|(id, k)| Sweep {
                id,
                channel_id: k.channel_id,
                victim: k.victim,
                kidnapping_type: k.kidnapping_type,
                mask: k.mask,
                captors: captors_of(eng, k.source),
            })
            .collect();

        for sweep in sweeps {
            match sweep.kidnapping_type {
                KidnappingType::Anonymous => {
                    wear_the_mask(eng, ctx, actor, version, mutate, &sweep)?
                }
                KidnappingType::Public(face) => {
                    seat_each_captor(eng, ctx, actor, version, mutate, &sweep, face)?
                }
            }
            evict_departed(eng, ctx, actor, version, mutate, &sweep)?;
        }

        Ok(ActionResponse::UpdateKidnappings(
            UpdateKidnappingsResponse {},
        ))
    }
}

// Whoever is on the captors' side right now.
//
// For an org, that is whoever can currently REACH the org, not whoever is on its roster. The two
// come apart constantly: a dead or imprisoned member is still listed and is taking no part in
// anything, and it is the org's channel viewport that already answers this correctly for every
// other purpose.
fn captors_of(eng: &Engine, source: KidnappingSource) -> SmallVec<[ActorKey; 8]> {
    let KidnappingSource::Ability(ability_id) = source else {
        return SmallVec::new();
    };
    let Some(owner_id) = get_ability(eng, ability_id)
        .ok()
        .and_then(|ability| ability.ownership_struct.owner)
    else {
        return SmallVec::new();
    };
    let Ok(owner) = get_actor(eng, owner_id) else {
        return SmallVec::new();
    };

    let ActorType::Org(org) = &owner.actor_type else {
        return SmallVec::from_slice(&[owner_id]);
    };
    let Ok(channel) = get_channel(eng, org.channel_id) else {
        return SmallVec::new();
    };
    eng.world
        .get_viewport(channel.viewport)
        .map(|viewport| viewport.members().collect())
        .unwrap_or_default()
}

// One mask for all of them, made on the first sweep that finds anybody to wear it.
fn wear_the_mask(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    actor: &ActionActor,
    version: Version,
    mutate: bool,
    sweep: &Sweep,
) -> Result<(), ActionError> {
    if sweep.captors.is_empty() {
        return Ok(());
    }

    let mask = match sweep.mask {
        Some(mask) => mask,
        None => {
            let response = Action::AddProfile(AddProfile {
                channel_id: sweep.channel_id,
                display: ActorDisplay::Mysterious,
                visible: true,
                shared: true,
                // It belongs to the kidnapping rather than to any of them, so it survives all of
                // them leaving.
                transferrable: true,
                perm_policy: PermUpdatePolicy::Fixed(FixedPolicy {
                    perms: ChannelPerm::Send | ChannelPerm::View,
                }),
            })
            .handle(eng, ctx, actor, version, mutate)?;
            let ActionResponse::AddProfile(data) = response else {
                unreachable!()
            };

            if mutate && let Some(kidnapping) = eng.world.get_kidnapping_mut(sweep.id) {
                kidnapping.mask = Some(data.profile_id);
            }
            data.profile_id
        }
    };

    for captor in &sweep.captors {
        Action::SetProfileAccess(SetProfileAccess {
            channel_id: sweep.channel_id,
            profile_id: mask,
            player_id: *captor,
            granted: true,
        })
        .handle(eng, ctx, actor, version, mutate)?;
    }

    Ok(())
}

// A name each. The one the kidnapping is publicly wearing starts visible, since it is what the act
// announced; everybody else behind it is hidden until they use theirs.
fn seat_each_captor(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    actor: &ActionActor,
    version: Version,
    mutate: bool,
    sweep: &Sweep,
    face: ActorDisplay,
) -> Result<(), ActionError> {
    let owed: SmallVec<[ActorKey; 8]> = {
        let channel = get_channel(eng, sweep.channel_id)?;
        sweep
            .captors
            .iter()
            .copied()
            .filter(|id| !channel.is_member(*id))
            .collect()
    };

    for captor in owed {
        let display = ActorDisplay::Raw(captor);
        Action::CreateAndGiveProfile(CreateAndGiveProfile {
            channel_id: sweep.channel_id,
            player_id: captor,
            display,
            visible: display == face,
            shared: false,
            transferrable: false,
            perm_policy: PermUpdatePolicy::Presence(PresencePolicy {
                perms: ChannelPerm::Send | ChannelPerm::View,
            }),
        })
        .handle(eng, ctx, actor, version, mutate)?;
    }

    Ok(())
}

// Anyone in the channel who is neither the victim nor a captor any more. Dropping out of the org
// that took somebody takes you out of the room they are being held in.
fn evict_departed(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    actor: &ActionActor,
    version: Version,
    mutate: bool,
    sweep: &Sweep,
) -> Result<(), ActionError> {
    let departed: SmallVec<[ActorKey; 8]> = get_channel(eng, sweep.channel_id)?
        .members
        .keys()
        .copied()
        .filter(|id| *id != sweep.victim && !sweep.captors.contains(id))
        .collect();

    for gone in departed {
        Action::RemoveFromChannel(RemoveFromChannel {
            channel_id: sweep.channel_id,
            player_id: gone,
        })
        .handle(eng, ctx, actor, version, mutate)?;
    }

    Ok(())
}
