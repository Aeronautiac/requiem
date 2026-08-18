use lawliet_types::{
    ability::{AbilityName, Prosecute},
    action::{Action, ActionActor, StartProsecution},
    actor::ActorDisplay,
    prosecution::ProsecutionSource,
};

use crate::{
    common::Version,
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{get_player, player_id, require_no_blackout},
};

impl AbilityInterface for Prosecute {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Prosecute
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: Version,
        mutate: bool,
    ) -> super::AbilityResult {
        let prosecutor_id =
            player_id(actor).expect("expected valid actor to have prosecute ability");
        get_player(eng, prosecutor_id)?;

        // A trial cannot run in the dark — its phases freeze while the world-events viewport is
        // empty — so opening one during a blackout is refused outright rather than started frozen.
        require_no_blackout(eng)?;

        Action::StartProsecution(StartProsecution {
            autonomous: eng.config.defaults.prosecution_autonomous,
            defendant_id: self.target,
            source: ProsecutionSource::Ability(ability),
            defendant_display: ActorDisplay::Raw(self.target),
            prosecutor_display: ActorDisplay::Raw(prosecutor_id),
            prosecutor_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, Prosecute},
        action::CreateAndGiveAbility,
    };

    use crate::{
        config::role::Role,
        engine::Engine,
        test_helpers::{add_player, init_engine, quick_ability, set_blackout, use_ability},
    };

    // A trial cannot run in the dark, so filing one is refused outright — no prosecution is opened.
    #[test]
    fn a_blackout_blocks_filing() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let prosecutor = add_player(&mut eng, 0, Role::Civilian, "prosecutor");
        let defendant = add_player(&mut eng, 0, Role::Civilian, "defendant");
        let ability = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::Prosecute,
                variant: 0,
                actor_id: prosecutor,
                volatile: false,
                transferrable: false,
            },
        );

        set_blackout(&mut eng, 1, true);

        assert!(
            use_ability(
                &mut eng,
                2,
                prosecutor,
                ability,
                AbilityBehaviour::Prosecute(Prosecute { target: defendant }),
            )
            .is_err()
        );
        assert!(eng.world.prosecutions.is_empty());
    }
}
