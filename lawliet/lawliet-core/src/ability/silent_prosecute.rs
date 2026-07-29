// Org ability. Name a player as wanted and act on it immediately: no trial, no vote, and no
// prosecution object at all. Where Prosecute opens a case, this closes one that was never opened.
//
// The check is EFFECTIVE possession of Wanted, so a passive link counts. That is the whole of what
// makes this ability interesting after a blackout: Kira's Kingdom holds Wanted, its members inherit
// it through their org link, and everyone who joined afterwards is exposed by association without
// ever being marked themselves.
//
// The two outcomes are deliberately lopsided. Being right costs nothing and is uncapped — the
// ability draws no charge pool. Being wrong costs the accuser their place in the org and their
// anonymity, which is the only thing rationing this.

use lawliet_types::{
    ability::{AbilityName, SilentProsecute},
    action::{Action, ActionActor, Kill, SetBlacklistStatus},
    command::Command,
    passive::PassiveType,
};

use crate::{
    ability::{AbilityInterface, AbilityStatus},
    action::ActionInterface,
    helpers::{
        actor_get_effective_passive, actor_id, cmd_world_event, get_org, get_player, player_id,
        require_alive,
    },
};

impl AbilityInterface for SilentProsecute {
    fn ability_name(&self) -> AbilityName {
        AbilityName::SilentProsecute
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &ActionActor,
        _ability: crate::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        actor.org_only()?;
        let org_id = actor_id(actor).expect("org actor has an id");
        let accuser_id = player_id(actor).expect("org actor names the member acting for it");

        require_alive(eng, self.target)?;

        let wanted = actor_get_effective_passive(eng, self.target, |passive_type| {
            matches!(passive_type, PassiveType::Wanted)
        })
        .is_some();

        if wanted {
            // The accuser is the killer, exactly as the prosecutor is on a guilty verdict: this IS
            // the verdict, so what an execution would have transferred transfers here too.
            Action::Kill(Kill {
                allow_link_chaining: true,
                sever_links: true,
                silent: false,
                set_books_dormant: false,
                death_message: Some(
                    eng.config.defaults.silent_prosecution_death_message.clone(),
                ),
                killer_id: Some(accuser_id),
                target_id: self.target,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;

            return Ok(AbilityStatus::Success);
        }

        // Both read before the blacklist, which kicks: afterwards the accuser is no longer a member
        // of the org this has to name, and nothing below may depend on that ordering silently.
        let true_name = get_player(eng, accuser_id)?.true_name.to_string();
        let org = get_org(eng, org_id)?.org_name;

        Action::SetBlacklistStatus(SetBlacklistStatus {
            actor_id: accuser_id,
            org_id,
            blacklisted: true,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        cmd_world_event(
            eng,
            ctx,
            Command::FailedSilentProsecution {
                accuser_id,
                true_name,
                org,
            },
        );

        Ok(AbilityStatus::Failure)
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexSet;
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, SilentProsecute},
        action::CreateAndGiveOrgAbility,
        command::Command,
        organization::OrgAbility,
    };

    use crate::{
        actor::state::State,
        common::{AbilityKey, ActorKey},
        config::{actor::organization::OrganizationName, role::Role},
        engine::Engine,
        helpers::{get_actor, get_org},
        passive::PassiveType,
        test_helpers::{
            add_org, add_player, add_to_org, quick_kill, quick_org_ability, quick_passive,
            started_engine, use_org_ability,
        },
    };

    // An org holding the ability with one member, which is every test's starting position. NULL
    // rather than TF: nothing here should depend on the default org config, and a bare engine has
    // none of the world charge pools TF's other abilities link to.
    fn org_with_ability(eng: &mut Engine, member: ActorKey) -> (ActorKey, AbilityKey) {
        let org = add_org(eng, 0, OrganizationName::NULL);
        let ability = quick_org_ability(
            eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::SilentProsecute,
                variant: 0,
                org_id: org,
                settings: OrgAbility {
                    require_roles: IndexSet::new(),
                    require_members: 0,
                    usage_policies: Default::default(),
                },
            },
        );
        add_to_org(eng, 0, org, member, false, true).unwrap();
        (org, ability)
    }

    #[test]
    fn a_wanted_target_dies_and_costs_the_accuser_nothing() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "accuser");
        let target = add_player(&mut eng, 0, Role::WantedCivilian, "target");
        let (org, ability) = org_with_ability(&mut eng, accuser);

        use_org_ability(
            &mut eng,
            1,
            accuser,
            org,
            ability,
            AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
        )
        .unwrap();

        assert!(get_actor(&eng, target).unwrap().has_state(State::Dead));

        let org_data = get_org(&eng, org).unwrap();
        assert!(org_data.has_member(accuser));
        assert!(!org_data.blacklist.contains(&accuser));
    }

    // The point of the ability after a blackout: the passive sits on the ORG, and everyone linked
    // to it is exposed without ever being marked themselves.
    #[test]
    fn wanted_reached_through_a_passive_link_counts() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "accuser");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let (org, ability) = org_with_ability(&mut eng, accuser);

        let marked_org = add_org(&mut eng, 0, OrganizationName::NULL);
        quick_passive(&mut eng, 0, marked_org, PassiveType::Wanted, false);
        add_to_org(&mut eng, 0, marked_org, target, false, true).unwrap();

        use_org_ability(
            &mut eng,
            1,
            accuser,
            org,
            ability,
            AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
        )
        .unwrap();

        assert!(get_actor(&eng, target).unwrap().has_state(State::Dead));
    }

    #[test]
    fn an_innocent_target_burns_the_accuser() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "Accuser Truename");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let (org, ability) = org_with_ability(&mut eng, accuser);

        let (_, ctx) = use_org_ability(
            &mut eng,
            1,
            accuser,
            org,
            ability,
            AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
        )
        .unwrap();

        assert!(!get_actor(&eng, target).unwrap().has_state(State::Dead));

        let org_data = get_org(&eng, org).unwrap();
        assert!(!org_data.has_member(accuser));
        assert!(org_data.blacklist.contains(&accuser));

        let announced = ctx
            .commands
            .iter()
            .find_map(|p| match &p.cmd {
                Command::FailedSilentProsecution {
                    accuser_id,
                    true_name,
                    org,
                } => Some((*accuser_id, true_name.clone(), *org)),
                _ => None,
            })
            .expect("the world is told");
        assert_eq!(
            announced,
            (
                accuser,
                // The engine normalises true names on the way in; the announcement carries what it
                // stored, not what was typed.
                "accuser truename".to_string(),
                OrganizationName::NULL
            )
        );
    }

    // Who was accused is not in the announcement, and nothing else in the action names them
    // either — an innocent target learns nothing about having been suspected.
    #[test]
    fn a_failed_accusation_never_names_the_target() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "accuser");
        let target = add_player(&mut eng, 0, Role::Civilian, "target");
        let (org, ability) = org_with_ability(&mut eng, accuser);

        let (_, ctx) = use_org_ability(
            &mut eng,
            1,
            accuser,
            org,
            ability,
            AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
        )
        .unwrap();

        assert!(!format!("{:?}", ctx.commands).contains(&format!("{target:?}")));
    }

    // Nothing rations a correct accusation: the ability draws no charge pool, so the second one
    // lands exactly like the first.
    #[test]
    fn success_is_uncapped() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "accuser");
        let first = add_player(&mut eng, 0, Role::WantedCivilian, "first");
        let second = add_player(&mut eng, 0, Role::WantedCivilian, "second");
        let (org, ability) = org_with_ability(&mut eng, accuser);

        for target in [first, second] {
            use_org_ability(
                &mut eng,
                1,
                accuser,
                org,
                ability,
                AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
            )
            .unwrap();
            assert!(get_actor(&eng, target).unwrap().has_state(State::Dead));
        }
    }

    // Refused outright rather than treated as a wrong accusation. The dead are neither wanted nor
    // innocent, and burning an accuser for aiming at a corpse would be a punishment for a mistake
    // the ability can see coming.
    #[test]
    fn a_dead_target_is_refused() {
        let mut eng = started_engine();
        let accuser = add_player(&mut eng, 0, Role::Civilian, "accuser");
        let target = add_player(&mut eng, 0, Role::WantedCivilian, "target");
        let (org, ability) = org_with_ability(&mut eng, accuser);
        quick_kill(&mut eng, 0, false, true, false, target);

        assert!(
            use_org_ability(
                &mut eng,
                1,
                accuser,
                org,
                ability,
                AbilityBehaviour::SilentProsecute(SilentProsecute { target }),
            )
            .is_err()
        );

        let org_data = get_org(&eng, org).unwrap();
        assert!(org_data.has_member(accuser));
        assert!(!org_data.blacklist.contains(&accuser));
    }
}
