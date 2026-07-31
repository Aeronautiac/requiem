/*
* SYSTEM ACTION
* Reconcile who is in the prison channel against who is actually locked up.
*
* The prison hands out no seats of its own, deliberately: a channel everybody belonged to would be
* one every client had to hold and hide. So membership is managed here, and State::Incarcerated is
* the authority — it is set and cleared from more places than could each be trusted to remember
* this.
*
* A seat is worth talking and listening for as long as its holder is alive. Nothing else is asked:
* being locked up is what put them here, and the contact it cuts everywhere else must not reach
* the one room it left them.
*/

use lawliet_types::{
    action::ActionResponse,
    actor::ActorDisplay,
    channel::{AlivePolicy, PermUpdatePolicy},
};
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResult, CreateAndGiveProfile,
        RemoveFromChannel,
    },
    actor::{ActorType, state::State},
    common::{ActorKey, Version},
    config::world::WorldChannelName,
    engine::Engine,
    helpers::{get_channel, get_world_channel_id},
};

pub use crate::action::{UpdatePrisonChannel, UpdatePrisonChannelResponse};

impl ActionInterface for UpdatePrisonChannel {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Before the world is built there is no prison to be in.
        let Ok(channel_id) = get_world_channel_id(eng, WorldChannelName::Prison) else {
            return Ok(ActionResponse::UpdatePrisonChannel(
                UpdatePrisonChannelResponse {},
            ));
        };

        let locked_up: SmallVec<[ActorKey; 8]> = eng
            .world
            .actors
            .iter()
            .filter(|(_, a)| matches!(a.actor_type, ActorType::Player(_)))
            .filter(|(_, a)| a.has_state(State::Incarcerated))
            .map(|(id, _)| id)
            .collect();

        let (owed, released): (SmallVec<[ActorKey; 8]>, SmallVec<[ActorKey; 8]>) = {
            let channel = get_channel(eng, channel_id)?;
            (
                locked_up
                    .iter()
                    .copied()
                    .filter(|id| !channel.is_member(*id))
                    .collect(),
                channel
                    .members
                    .keys()
                    .copied()
                    .filter(|id| !locked_up.contains(id))
                    .collect(),
            )
        };

        for player_id in owed {
            Action::CreateAndGiveProfile(CreateAndGiveProfile {
                channel_id,
                player_id,
                display: ActorDisplay::Raw(player_id),
                visible: true,
                shared: false,
                transferrable: false,
                perm_policy: PermUpdatePolicy::Alive(AlivePolicy {}),
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        for player_id in released {
            Action::RemoveFromChannel(RemoveFromChannel {
                channel_id,
                player_id,
            })
            .handle(eng, ctx, actor, version, mutate)?;
        }

        Ok(ActionResponse::UpdatePrisonChannel(
            UpdatePrisonChannelResponse {},
        ))
    }
}
