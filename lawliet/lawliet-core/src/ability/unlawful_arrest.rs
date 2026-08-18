// A civilian arrest with the vote taken out: the target is jailed immediately, for
// `unlawful_arrest_time`, and released automatically. The world sees an ordinary incarceration —
// an incarceration never discloses its source, so nobody learns it was unlawful, let alone whose.

use lawliet_types::{
    ability::{AbilityName, UnlawfulArrest},
    action::{Action, ActionActor, CreateIncarceration},
    incarceration::IncarcerationSource,
};

use crate::{
    common::Version,ability::AbilityInterface, action::ActionInterface, helpers::get_player};

impl AbilityInterface for UnlawfulArrest {
    fn ability_name(&self) -> AbilityName {
        AbilityName::UnlawfulArrest
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        _actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: Version,
        mutate: bool,
    ) -> super::AbilityResult {
        // You can only arrest a player; the arrester need not be one.
        get_player(eng, self.target)?;

        Action::CreateIncarceration(CreateIncarceration {
            victim_id: self.target,
            source: IncarcerationSource::Ability(ability),
            duration: Some(eng.config.defaults.unlawful_arrest_time),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, UnlawfulArrest},
        action::{ActionError, CreateAndGiveAbility},
        actor::State,
        command::Command,
    };

    use crate::{
        config::role::Role,
        engine::Engine,
        helpers::get_actor,
        test_helpers::{
            add_player, init_engine, null_action, quick_ability, quick_kill, use_ability,
        },
    };

    fn armed(eng: &mut Engine) -> (crate::ActorKey, crate::ActorKey, crate::AbilityKey) {
        init_engine(eng);
        let user = add_player(eng, 0, Role::Civilian, "user");
        let target = add_player(eng, 0, Role::Civilian, "target");
        let ability = quick_ability(
            eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::UnlawfulArrest,
                variant: 0,
                actor_id: user,
                volatile: false,
                transferrable: false,
            },
        );
        (user, target, ability)
    }

    #[test]
    fn it_jails_the_target_with_no_vote() {
        let mut eng = Engine::new();
        let (user, target, ability) = armed(&mut eng);

        let (_, ctx) = use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnlawfulArrest(UnlawfulArrest { target }),
        )
        .unwrap();

        assert!(
            get_actor(&eng, target)
                .unwrap()
                .has_state(State::Incarcerated)
        );
        // No poll stands between the use and the cell.
        assert!(
            !ctx.commands
                .iter()
                .any(|p| matches!(&p.cmd, Command::UpdatePoll { .. }))
        );
    }

    // An incarceration never discloses its source, so nothing on the wire names the arrester.
    #[test]
    fn the_announcement_names_no_arrester() {
        let mut eng = Engine::new();
        let (user, target, ability) = armed(&mut eng);

        let (_, ctx) = use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnlawfulArrest(UnlawfulArrest { target }),
        )
        .unwrap();

        let announced = ctx
            .commands
            .iter()
            .find_map(|p| match &p.cmd {
                Command::Incarceration {
                    victim_id,
                    duration,
                    ..
                } => Some((*victim_id, *duration)),
                _ => None,
            })
            .expect("announced");
        assert_eq!(
            announced,
            (target, Some(eng.config.defaults.unlawful_arrest_time))
        );
    }

    #[test]
    fn the_sentence_expires_on_its_own() {
        let mut eng = Engine::new();
        let (user, target, ability) = armed(&mut eng);
        use_ability(
            &mut eng,
            1,
            user,
            ability,
            AbilityBehaviour::UnlawfulArrest(UnlawfulArrest { target }),
        )
        .unwrap();

        let past_release = 1 + eng.config.defaults.unlawful_arrest_time + 1;
        null_action(&mut eng, past_release);

        assert!(
            !get_actor(&eng, target)
                .unwrap()
                .has_state(State::Incarcerated)
        );
    }

    // Dead players have NoPresence, which CreateIncarceration refuses.
    #[test]
    fn a_dead_target_cannot_be_arrested() {
        let mut eng = Engine::new();
        let (user, target, ability) = armed(&mut eng);
        quick_kill(&mut eng, 1, false, false, false, target);

        let err = use_ability(
            &mut eng,
            2,
            user,
            ability,
            AbilityBehaviour::UnlawfulArrest(UnlawfulArrest { target }),
        )
        .unwrap_err()
        .0;

        assert!(matches!(err, ActionError::UserNotPresent));
    }
}
