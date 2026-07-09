use indexmap::indexset;

use crate::{
    ability::AbilityInterface,
    action::{
        Action, ActionActor, ActionError, ActionInterface, ActionResponse, SetGroupchatOwner,
        SetMember, UpdateContactChannels,
    },
    actor::{ActorDisplay, modifier::Modifier},
    channel::{ChannelMember, ChannelPermissions},
    config::ability::AbilityName,
    helpers::{actor_id, get_actor, get_gc, get_gc_mut, get_player_mut},
};

// The action lives under crate::action with the same name as this ability behaviour,
// so alias it to keep the two distinct.
use crate::action::CreateGroupchat as CreateGroupchatAction;

pub use lawliet_types::ability::CreateGroupchat;

impl AbilityInterface for CreateGroupchat {
    fn ability_name(&self) -> AbilityName {
        AbilityName::CreateGroupchat
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        let creator_id =
            actor_id(actor).expect("expected valid actor id within create groupchat ability");

        let creator_data = get_actor(eng, creator_id)?;
        if creator_data.has_modifier(Modifier::NoContact) {
            return Err(ActionError::CannotContact);
        }

        // Create the underlying group chat + channel as the system. On the validation
        // pass this returns a default id and mutates nothing.
        let response = Action::CreateGroupchat(CreateGroupchatAction {})
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        let ActionResponse::CreateGroupchat(data) = response else {
            unreachable!();
        };
        let gc_id = data.id;

        // Make the creator the owner and a member. The membership wiring (channel
        // member entry, gc/player caches, perm application) is only meaningful once the
        // gc actually exists, so it is gated on the mutation pass — exactly like the
        // lounge participant setup in create_lounge.
        if mutate {
            let channel_id = get_gc(eng, gc_id)?.channel_id;

            Action::SetMember(SetMember {
                channel_id,
                player_id: creator_id,
                settings: Some(ChannelMember {
                    perms: ChannelPermissions::EMPTY,
                    displays: indexset![ActorDisplay::Raw(creator_id)],
                }),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            let gc = get_gc_mut(eng, gc_id)?;
            gc.add_member(creator_id);

            let player_data = get_player_mut(eng, creator_id)?;
            player_data.add_groupchat(gc_id);

            // Apply the group chat's channel permissions from current state (Send|View).
            Action::UpdateContactChannels(UpdateContactChannels {
                player_id: creator_id,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            // Hand ownership to the creator.
            Action::SetGroupchatOwner(SetGroupchatOwner {
                groupchat_id: gc_id,
                owner: Some(creator_id),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(())
    }
}
