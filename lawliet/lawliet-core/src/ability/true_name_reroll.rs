// Give the target a new true name, retiring whatever anyone had learned about the old one. The
// private investigator's single reroll for the whole game.
//
// The name arrives on the action, already replaced by the server. It is NOT fetched from the client
// mid-action: the engine replays its log, so an action that waited on an answer would leave a window
// no replay could reproduce — and a client picking its own true name is not something to trust it
// with anyway. Same treatment as timestamps.

use lawliet_types::ability::{AbilityName, TrueNameReroll};

use crate::{
    common::Version,
    ability::AbilityInterface,
    action::{Action, ActionActor, ActionInterface, SetTrueName},
    helpers::require_alive,
};

impl AbilityInterface for TrueNameReroll {
    fn ability_name(&self) -> AbilityName {
        AbilityName::TrueNameReroll
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut crate::action::ActionContext,
        _actor: &crate::action::ActionActor,
        _ability: crate::AbilityKey,
        version: Version,
        mutate: bool,
    ) -> super::AbilityResult {
        require_alive(eng, self.target)?;

        // Rejects a name another player already holds, which is the one way the server's choice can
        // still be wrong by the time it lands.
        Action::SetTrueName(SetTrueName {
            target_id: self.target,
            true_name: self.true_name.clone(),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, TrueNameReroll},
        action::{ActionError, CreateAndGiveAbility},
        command::{Command, CommandRecipient},
    };

    use crate::{
        ActorKey,
        config::role::Role,
        engine::Engine,
        helpers::get_player,
        test_helpers::{add_player, init_engine, quick_ability, quick_kill, use_ability},
    };

    fn armed(eng: &mut Engine) -> (ActorKey, ActorKey, crate::AbilityKey) {
        init_engine(eng);
        let pi = add_player(eng, 0, Role::PrivateInvestigator, "pi");
        let target = add_player(eng, 0, Role::Civilian, "old name");
        let ability = quick_ability(
            eng,
            0,
            CreateAndGiveAbility {
                ability_name: AbilityName::TrueNameReroll,
                variant: 0,
                actor_id: pi,
                volatile: false,
                transferrable: false,
            },
        );
        (pi, target, ability)
    }

    #[test]
    fn it_replaces_the_targets_true_name() {
        let mut eng = Engine::new();
        let (pi, target, ability) = armed(&mut eng);

        use_ability(
            &mut eng,
            1,
            pi,
            ability,
            AbilityBehaviour::TrueNameReroll(TrueNameReroll {
                target,
                true_name: "new name".into(),
            }),
        )
        .unwrap();

        assert_eq!(&*get_player(&eng, target).unwrap().true_name, "new name");
    }

    // The reroll is told to the player it happened to, and mirrored to System — the same pair
    // SetTrueName always notifies. Nobody else learns the new name from this.
    #[test]
    fn only_the_target_and_system_are_told() {
        let mut eng = Engine::new();
        let (pi, target, ability) = armed(&mut eng);

        let (_, ctx) = use_ability(
            &mut eng,
            1,
            pi,
            ability,
            AbilityBehaviour::TrueNameReroll(TrueNameReroll {
                target,
                true_name: "new name".into(),
            }),
        )
        .unwrap();

        let told: Vec<CommandRecipient> = ctx
            .commands
            .iter()
            .filter(|p| matches!(&p.cmd, Command::TrueNameUpdate { .. }))
            .map(|p| p.recipient.clone())
            .collect();
        assert_eq!(
            told,
            vec![CommandRecipient::Actor(target), CommandRecipient::System]
        );
    }

    // A name someone else holds is refused, so the reroll cannot be used to impersonate.
    #[test]
    fn a_taken_name_is_refused() {
        let mut eng = Engine::new();
        let (pi, target, ability) = armed(&mut eng);
        add_player(&mut eng, 0, Role::Civilian, "taken");

        let err = use_ability(
            &mut eng,
            1,
            pi,
            ability,
            AbilityBehaviour::TrueNameReroll(TrueNameReroll {
                target,
                true_name: "taken".into(),
            }),
        )
        .unwrap_err()
        .0;

        assert!(matches!(err, ActionError::NameNotUnique));
        assert_eq!(&*get_player(&eng, target).unwrap().true_name, "old name");
    }

    #[test]
    fn a_dead_target_cannot_be_rerolled() {
        let mut eng = Engine::new();
        let (pi, target, ability) = armed(&mut eng);
        quick_kill(&mut eng, 1, false, false, false, target);

        let err = use_ability(
            &mut eng,
            2,
            pi,
            ability,
            AbilityBehaviour::TrueNameReroll(TrueNameReroll {
                target,
                true_name: "new name".into(),
            }),
        )
        .unwrap_err()
        .0;

        assert!(matches!(err, ActionError::ActorIsDead));
    }
}
