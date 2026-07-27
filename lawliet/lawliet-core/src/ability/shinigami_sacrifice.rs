// Org ability. The org spends one of its founding members for a name: the sacrifice dies, and the
// true name of some other player is revealed to the org rather than to whoever happened to spend
// the charge.
//
// Only OG members can be spent — the org pays with its own, not with somebody it recruited for the
// purpose. The death is announced with its own message, so the world learns a sacrifice happened
// while learning nothing about who was named.

use lawliet_types::{ability::AbilityName, action::ActionError, command::Command};

use crate::{
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, Kill},
    helpers::{actor_id, cmd_channel, get_org, get_player, require_alive},
};

pub use lawliet_types::ability::ShinigamiSacrifice;

impl AbilityInterface for ShinigamiSacrifice {
    fn ability_name(&self) -> AbilityName {
        AbilityName::ShinigamiSacrifice
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

        if self.sacrifice == self.name_target {
            return Err(ActionError::CannotSacrificeForOwnName);
        }

        let is_og = get_org(eng, org_id)?
            .members
            .get(&self.sacrifice)
            .ok_or(ActionError::PlayerNotInOrg)?
            .og;
        if !is_og {
            return Err(ActionError::NotAnOgMember);
        }

        require_alive(eng, self.sacrifice)?;
        require_alive(eng, self.name_target)?;

        // Read before the kill: a life link could take the name target down with the sacrifice, and
        // the org paid for the name either way.
        let true_name = get_player(eng, self.name_target)?.true_name.to_string();
        let org_channel = get_org(eng, org_id)?.channel_id;

        Action::Kill(Kill {
            allow_link_chaining: true,
            death_message: Some(eng.config.defaults.sacrifice_death_message.clone()),
            killer_id: None,
            target_id: self.sacrifice,
            set_books_dormant: false,
            sever_links: true,
            silent: false,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        // Addressed to the org's backing channel: who may learn what the org bought is exactly who
        // may see the org.
        cmd_channel(
            eng,
            ctx,
            Command::RevealTrueName {
                target_id: self.name_target,
                true_name,
            },
            org_channel,
        );

        Ok(super::AbilityStatus::Success)
    }
}
