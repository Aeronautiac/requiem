use lawliet_types::{
    ability::{AbilityName, AnonymousProsecute},
    action::{Action, ActionActor, StartProsecution},
    actor::ActorDisplay,
    prosecution::ProsecutionSource,
};

use crate::{
    ability::AbilityInterface,
    action::ActionInterface,
    helpers::{actor_id, get_player, require_no_blackout},
};

impl AbilityInterface for AnonymousProsecute {
    fn ability_name(&self) -> lawliet_types::ability::AbilityName {
        AbilityName::AnonymousProsecute
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u64,
        mutate: bool,
    ) -> super::AbilityResult {
        // TODO:
        // potentially display the org rather than the role if its somehow an org, but for now its
        // only players
        let prosecutor_id =
            actor_id(actor).expect("expected valid actor to have anonymous prosecute ability");
        let prosecutor_data = get_player(eng, prosecutor_id)?;

        // As with an open prosecution: a trial cannot run while the world is dark, so filing one —
        // anonymously or not — is refused during a blackout. Silent prosecution, which opens no
        // trial, is deliberately exempt.
        require_no_blackout(eng)?;

        Action::StartProsecution(StartProsecution {
            // Anonymity is about who the prosecutor is shown as, and says nothing about whether a
            // host confirms the phases — so this follows the same config as an open prosecution.
            autonomous: eng.config.defaults.prosecution_autonomous,
            defendant_id: self.target,
            source: ProsecutionSource::Ability(ability),
            defendant_display: ActorDisplay::Raw(self.target),
            prosecutor_display: ActorDisplay::Role(prosecutor_data.role),
            prosecutor_id,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, AnonymousProsecute},
        action::CreateAndGiveAbility,
    };

    use crate::{
        config::role::Role,
        engine::Engine,
        helpers::get_prosecution,
        test_helpers::{add_player, init_engine, quick_ability, set_blackout, started_engine, use_ability},
    };

    // Filing anonymously says who the prosecutor is SHOWN as. It is not a way to opt a trial out of
    // host confirmation, so this follows the same config as an open prosecution — otherwise a host
    // running non-autonomous trials has one ability that quietly bypasses them.
    #[test]
    fn it_honours_the_autonomy_config() {
        for autonomous in [true, false] {
            let mut eng = Engine::new();
            init_engine(&mut eng);
            eng.config.defaults.prosecution_autonomous = autonomous;

            let prosecutor = add_player(&mut eng, 0, Role::Civilian, "prosecutor");
            let defendant = add_player(&mut eng, 0, Role::Civilian, "defendant");
            let ability = quick_ability(
                &mut eng,
                0,
                CreateAndGiveAbility {
                    ability_name: AbilityName::AnonymousProsecute,
                    variant: 0,
                    actor_id: prosecutor,
                    volatile: false,
                    transferrable: false,
                },
            );

            use_ability(
                &mut eng,
                1,
                prosecutor,
                ability,
                AbilityBehaviour::AnonymousProsecute(AnonymousProsecute { target: defendant }),
            )
            .unwrap();

            let id = eng.world.prosecutions.keys().next().expect("a prosecution");
            assert_eq!(get_prosecution(&eng, id).unwrap().autonomous, autonomous);
        }
    }

    // A trial cannot run in the dark, so filing one is refused outright — no prosecution is opened.
    #[test]
    fn a_blackout_blocks_filing() {
        let mut eng = started_engine();
        let prosecutor = add_player(&mut eng, 0, Role::Civilian, "prosecutor");
        let defendant = add_player(&mut eng, 0, Role::Civilian, "defendant");
        let ability = quick_ability(
            &mut eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::AnonymousProsecute,
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
                AbilityBehaviour::AnonymousProsecute(AnonymousProsecute { target: defendant }),
            )
            .is_err()
        );
        assert!(eng.world.prosecutions.is_empty());
    }
}
