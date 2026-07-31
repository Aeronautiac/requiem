/*
* PLAYER ACTION (defendant only)
* Select a lawyer during the custody phase.
*
* Preconditions:
* - caller is the defendant in this prosecution
* - prosecution is in the Custody phase
* - lawyer target exists, has presence, and is not the defendant
* - no lawyer has already been selected
*
* On execution:
* - create a private channel between defendant and lawyer
* - set defense.lawyer
*
* Commands: MapChannel to the new channel's viewport, then SetMember for each side (which
* emits their perms and the roster). The lawyer's identity reaches everyone else on the next
* prosecution snapshot, not from here.
*/

use lawliet_types::{
    channel::{ContactPolicy, PermUpdatePolicy},
    command::Command,
};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResponse,
        ActionResult, CreateAndGiveProfile, CreateChannel,
    },
    actor::{ActorDisplay, modifier::Modifier},
    channel::ChannelKind,
    common::Version,
    engine::Engine,
    helpers::{cmd_channel, get_actor, get_prosecution_mut, player_id, require_player},
    prosecution::{Lawyer, ProsecutionPhase},
};

pub use crate::action::{SelectLawyer, SelectLawyerResponse};

impl ActionInterface for SelectLawyer {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.player_only()?;
        let caller = player_id(actor).expect("already validated as player");

        let prosecution = get_prosecution_mut(eng, self.prosecution_id)?;

        if prosecution.defense.defendant != caller {
            return Err(ActionError::NotInProsecution);
        }
        if prosecution.defense.lawyer.is_some() {
            return Err(ActionError::LawyerAlreadySelected);
        }
        if !matches!(prosecution.phase, ProsecutionPhase::Custody { .. }) {
            return Err(ActionError::NotACustodyPhase);
        }
        if self.lawyer_id == caller {
            return Err(ActionError::CannotBeOwnLawyer);
        }

        require_player(eng, self.lawyer_id)?;
        if get_actor(eng, self.lawyer_id)
            .expect("already validated")
            .has_modifier(Modifier::NoPresence)
        {
            return Err(ActionError::UserNotPresent);
        }

        let channel_response = Action::CreateChannel(CreateChannel {
            loggable: false,
            base_profile: None,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        let ActionResponse::CreateChannel(channel_data) = channel_response else {
            unreachable!()
        };
        let channel_id = channel_data.id;

        // The channel only exists on the mutate pass, so everything that reaches for it lives here.
        if mutate {
            // Before the names that put people in it: seating them enters them into the viewport,
            // and a Map emitted afterwards would arrive behind history they already hold. The
            // frontend also indexes `channels` directly on AddMessage, so an unmapped channel is
            // fatal there.
            cmd_channel(
                eng,
                ctx,
                Command::MapChannel {
                    channel_id,
                    kind: ChannelKind::Lawyer(self.prosecution_id),
                },
                channel_id,
                false,
                None,
            );

            // Both sides, shown to each other as themselves — there is no anonymity between a
            // defendant and their own counsel. A contact line like any other, which is what keeps
            // it open through custody: being held cuts contact, and the accused is meant to keep
            // reaching their lawyer regardless.
            for player_id in [caller, self.lawyer_id] {
                Action::CreateAndGiveProfile(CreateAndGiveProfile {
                    channel_id,
                    player_id,
                    display: ActorDisplay::Raw(player_id),
                    visible: true,
                    shared: false,
                    transferrable: false,
                    perm_policy: PermUpdatePolicy::Contact(ContactPolicy {}),
                })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }

            get_prosecution_mut(eng, self.prosecution_id)
                .expect("already validated")
                .defense
                .lawyer = Some(Lawyer {
                actor_id: self.lawyer_id,
                channel_id: Some(channel_id),
            });
        }

        Ok(ActionResponse::SelectLawyer(SelectLawyerResponse {}))
    }
}
