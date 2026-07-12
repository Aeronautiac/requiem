// Org ability. Guess a player's true name to invite them into the acting org. The
// guess is compared case-insensitively; on a match the player is added immediately.
//
// Charges (see use_ability's conditional subtraction): this ability returns Success on
// a correct guess and Failure on a wrong one. Its attempts pool is conditioned on both
// outcomes (every guess costs an attempt), while the shared Invite pool is OnSuccess
// (only a correct guess spends an invite).

use lawliet_types::{
    ability::{AbilityName, TrueNameInvite},
    command::{Command, CommandRecipient},
};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, AddToOrg},
    helpers::{actor_id, get_player},
};

impl AbilityInterface for TrueNameInvite {
    fn ability_name(&self) -> AbilityName {
        AbilityName::TrueNameInvite
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
        actor.org_only()?;
        let org_id = actor_id(actor).expect("org actor has an id");

        let (name_matches, true_name) = {
            let target = get_player(eng, self.target)?;
            let true_name = target.true_name.to_string();
            (
                true_name.to_lowercase() == self.true_name.to_lowercase(),
                true_name,
            )
        };

        if name_matches {
            Action::AddToOrg(AddToOrg {
                leader: false,
                og: false,
                actor_id: self.target,
                org_id,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            // A successful invite reveals the new member's true name to the org (the
            // recipient is the org actor; the frontend gates it by org-channel view).
            ctx.push_cmd(
                Command::RevealTrueName {
                    target_id: self.target,
                    true_name,
                },
                CommandRecipient::Actor(org_id),
                eng.time,
            );

            // Success spends both the shared Invite pool and an attempt.
            Ok(super::AbilityStatus::Success)
        } else {
            // A wrong guess spends only an attempt (the attempts pool is conditioned on
            // both outcomes; the Invite pool is OnSuccess, so it's left alone).
            Ok(super::AbilityStatus::Failure)
        }
    }
}
