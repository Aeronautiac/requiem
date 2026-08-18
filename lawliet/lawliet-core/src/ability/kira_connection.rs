// Reach for Kira through a lounge you already have. Only a Basic lounge qualifies: an anonymous or
// fabricated line does not tell you who is really on the other end, so it cannot establish that you
// found the real one.
//
// Only a living Kira answers — a second Kira is not something you can connect to, and a dead one is
// past connecting to. Nothing is required of the user's own role, though: the ability exists for a
// second Kira to find the first, and enforcing that here would make holding it a confession.
//
// The attempt lands in the lounge either way, naming the user and saying whether it worked. You
// cannot feel for Kira quietly, and the person probed always learns you tried. Finding Kira
// destroys the user's OwnedNotebookBlock, which is what was stopping them writing in a notebook
// they own.

use lawliet_types::{
    ability::{AbilityName, KiraConnection},
    action::ActionError,
    actor::State,
    command::Command,
};
use smallvec::{SmallVec, smallvec};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, DestroyPassive},
    common::PassiveKey,
    config::role::Role,
    helpers::{actor_id, cmd_channel, get_actor, get_lounge, get_passive, get_player},
    lounge::LoungeVariant,
    passive::PassiveType,
};

impl AbilityInterface for KiraConnection {
    fn ability_name(&self) -> AbilityName {
        AbilityName::KiraConnection
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: u64,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.player_only()?;
        let user_id = actor_id(actor).expect("expected valid actor to use KiraConnection");

        let lounge = get_lounge(eng, self.lounge)?;
        let channel_id = lounge.channel_id;
        let LoungeVariant::Basic {
            contacted_id,
            contactor_id,
        } = lounge.variant
        else {
            return Err(ActionError::CannotContact);
        };

        // The other end of the line, whichever end the user is on.
        let other = if contactor_id == user_id {
            contacted_id
        } else if contacted_id == user_id {
            contactor_id
        } else {
            return Err(ActionError::PlayerNotInLounge);
        };

        let found = get_player(eng, other)?.role == Role::Kira
            && !get_actor(eng, other)?.has_state(State::Dead);

        // Emitted before the block comes off, so the lounge reads as the attempt and then its
        // consequence rather than the other way round.
        cmd_channel(
            eng,
            ctx,
            Command::KiraConnectionAttempt {
                channel_id,
                user: user_id,
                success: found,
            },
            channel_id,
            true,
            Some(user_id),
        );

        if !found {
            return Ok(super::AbilityStatus::Failure);
        }

        let mut blocks: SmallVec<[PassiveKey; 2]> = smallvec![];
        for passive_id in get_actor(eng, user_id)?.passives.clone() {
            if matches!(
                get_passive(eng, passive_id)?.passive_type,
                PassiveType::OwnedNotebookBlock
            ) {
                blocks.push(passive_id);
            }
        }

        for passive_id in blocks {
            Action::DestroyPassive(DestroyPassive { passive_id }).handle(
                eng,
                ctx,
                &ActionActor::System,
                version,
                mutate,
            )?;
        }

        Ok(super::AbilityStatus::Success)
    }
}
