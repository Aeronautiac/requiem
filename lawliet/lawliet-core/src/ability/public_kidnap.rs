use lawliet_types::{
    ability::{AbilityName, PublicKidnap},
    action::{Action, ActionActor, ActionError, Kidnap},
    actor::ActorDisplay,
    kidnapping::{KidnappingSource, KidnappingType},
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    actor::modifier::Modifier,
    helpers::{get_actor, get_org, get_player},
};

impl AbilityInterface for PublicKidnap {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::PublicKidnap
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        get_player(eng, self.target)?;

        // The publicly-shown kidnapper. A player is always themselves and may not designate
        // anyone else; an org picks one of its own, defaulting to the acting member.
        let performer = match actor {
            ActionActor::Player(id) => {
                if self.performer.is_some() {
                    return Err(ActionError::PerformerRequiresOrg);
                }
                *id
            }
            ActionActor::Organization(org) => {
                let performer = self.performer.unwrap_or(org.player_id);
                // The public face must belong to this org and be present.
                if !get_org(eng, org.org_id)?.has_member(performer) {
                    return Err(ActionError::PlayerNotInOrg);
                }
                if get_actor(eng, performer)?.has_modifier(Modifier::NoPresence) {
                    return Err(ActionError::UserNotPresent);
                }
                performer
            }
            ActionActor::Admin | ActionActor::System => {
                return Err(ActionError::InsufficientPermissions);
            }
        };

        Action::Kidnap(Kidnap {
            victim_id: self.target,
            kidnapping_type: KidnappingType::Public(ActorDisplay::Raw(performer)),
            source: KidnappingSource::Ability(ability),
            duration: Some(eng.config.defaults.kidnap_time),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
