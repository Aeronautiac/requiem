/*
* SYSTEM ACTION
* Update channel membership for every active kidnapping based on current world state.
*
* Called from AddState, RemoveState, AddToOrg, RemoveFromOrg.
*
* Victim:
*   alive + currently kidnapped → Send | View, Raw display
*   otherwise                   → EMPTY perms
*
* Kidnapper side (derived from source ability owner):
*   owner is org  → each present member gets Send | View
*   owner is player → that player gets Send | View if present
*   display: Mysterious (anonymous) or Raw (public)
*
* TODO: commands & optimizations
*/

use indexmap::indexset;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        SetMember,
    },
    actor::{ActorDisplay, ActorType, modifier::Modifier, state::State},
    channel::{ChannelMember, ChannelPermission, ChannelPermissions},
    common::{ActorKey, ChannelKey, Version},
    engine::Engine,
    helpers::{get_ability, get_actor},
    kidnapping::{KidnappingSource, KidnappingType},
};

struct KidnappingData {
    victim: ActorKey,
    channel_id: ChannelKey,
    kidnapping_type: KidnappingType,
    source: KidnappingSource,
}

struct MemberUpdate {
    player_id: ActorKey,
    channel_id: ChannelKey,
    settings: ChannelMember,
}

pub use crate::action::{UpdateKidnapChannels, UpdateKidnapChannelsResponse};

impl ActionInterface for UpdateKidnapChannels {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let kidnappings: Vec<KidnappingData> = eng
            .world
            .kidnappings
            .values()
            .map(|k| KidnappingData {
                victim: k.victim,
                channel_id: k.channel_id,
                kidnapping_type: k.kidnapping_type,
                source: k.source,
            })
            .collect();

        let mut updates: Vec<MemberUpdate> = vec![];

        for kd in kidnappings {
            let victim_actor = get_actor(eng, kd.victim).expect("kidnapping victim must be valid");
            let victim_perms = if !victim_actor.has_state(State::Dead)
                && victim_actor.has_state(State::Kidnapped)
            {
                ChannelPermission::Send | ChannelPermission::View
            } else {
                ChannelPermissions::EMPTY
            };
            updates.push(MemberUpdate {
                player_id: kd.victim,
                channel_id: kd.channel_id,
                settings: ChannelMember {
                    perms: victim_perms,
                    displays: indexset![ActorDisplay::Raw(kd.victim)],
                },
            });

            // derive kidnapper-side members from the source ability's owner
            let kidnapper_members: Vec<ActorKey> = match kd.source {
                KidnappingSource::None => vec![],
                KidnappingSource::Ability(ab_key) => {
                    match get_ability(eng, ab_key)
                        .ok()
                        .and_then(|ab| ab.ownership_struct.owner)
                    {
                        None => vec![],
                        Some(owner_id) => {
                            let owner =
                                get_actor(eng, owner_id).expect("ability owner must be valid");
                            if let ActorType::Org(org) = &owner.actor_type {
                                org.members.keys().copied().collect()
                            } else {
                                vec![owner_id]
                            }
                        }
                    }
                }
            };

            for member_id in kidnapper_members {
                let member_actor =
                    get_actor(eng, member_id).expect("kidnapper-side member must be valid");
                let perms = if !member_actor.has_modifier(Modifier::NoPresence) {
                    ChannelPermission::Send | ChannelPermission::View
                } else {
                    ChannelPermissions::EMPTY
                };
                let display = match kd.kidnapping_type {
                    KidnappingType::Anonymous => ActorDisplay::Mysterious,
                    KidnappingType::Public(_) => ActorDisplay::Raw(member_id),
                };
                updates.push(MemberUpdate {
                    player_id: member_id,
                    channel_id: kd.channel_id,
                    settings: ChannelMember {
                        perms,
                        displays: indexset![display],
                    },
                });
            }
        }

        for update in updates {
            Action::SetMember(SetMember {
                player_id: update.player_id,
                channel_id: update.channel_id,
                settings: Some(update.settings),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::UpdateKidnapChannels(
            UpdateKidnapChannelsResponse {},
        ))
    }
}
