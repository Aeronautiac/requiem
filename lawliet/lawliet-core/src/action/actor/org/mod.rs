pub mod add_to_org;
pub mod change_org_leader;
pub mod create_and_give_org_ability;
pub mod create_org;
pub mod give_org_ability;
pub mod remove_from_org;
pub mod resign_leadership;
pub mod set_leadership;
pub mod system_use_org_ability;
pub mod use_org_ability;

// you should be allowed to add and remove dead people to/from an org. these restrictions shall be
// applied through invite abilities and similar if necessary.
// when someone dies, they remain an org member

// org members who are not present should not be allowed to use abilities

// org additions
// org leadership
// org passives
// abilities that require votes and dont require votes
// leader only abilities
// og status
// blacklists
// member requirements
// leadership changes
//
// things like specific invite abilities SHOULD NOT be tested here, only the general org system

#[cfg(test)]
mod org_tests {
    use indexmap::{IndexSet, indexset};

    use crate::{
        ability::{AbilityBehaviour, gun::Gun},
        action::{
            ActionResponse, actor::org::create_and_give_org_ability::CreateAndGiveOrgAbility,
        },
        actor::{
            organization::{
                LeadershipTransferPolicies, OrgAbility, OrgAbilityPolicies, OrgAbilityPolicy,
            },
            state::State,
        },
        config::{ability::AbilityName, actor::organization::OrganizationName, role::Role},
        engine::Engine,
        helpers::{actor_get_effective_passive, get_actor, get_org},
        passive::PassiveType,
        test_helpers::*,
    };

    #[test]
    fn basic_addition() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let org = get_org(&eng, o1).unwrap();
        assert!(!org.has_member(p1));

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.has_member(p1));
    }

    #[test]
    fn basic_removal() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        remove_from_org(&mut eng, 0, o1, p1).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(!org.has_member(p1));
    }

    // operations on dead people should be allowed. these restrictions are only applied through
    // invite abilities if applicable.
    #[test]
    fn add_dead() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        quick_kill(&mut eng, 0, true, true, false, p1);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.has_member(p1));
    }

    #[test]
    fn remove_dead() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        quick_kill(&mut eng, 0, true, true, false, p1);
        remove_from_org(&mut eng, 0, o1, p1).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(!org.has_member(p1));
    }

    #[test]
    fn leader_no_old() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        set_leadership(&mut eng, 0, o1, Some(LeadershipTransferPolicies::ALL));

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.get_leader().is_none());

        change_leader(&mut eng, 0, o1, Some(p1)).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.get_leader() == Some(p1));
    }

    // TODO: implement actions for modifying OG status
    #[test]
    fn change_og_status() {}

    #[test]
    fn already_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        assert!(add_to_org(&mut eng, 0, o1, p1, false, true).is_err());
    }

    #[test]
    fn kick_non_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        assert!(remove_from_org(&mut eng, 0, o1, p1).is_err());
    }

    // replace an existing leader with a new leader
    #[test]
    fn leader_replace() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        set_leadership(&mut eng, 0, o1, Some(LeadershipTransferPolicies::ALL));

        add_to_org(&mut eng, 0, o1, p1, true, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.get_leader() == Some(p1));

        add_to_org(&mut eng, 0, o1, p2, true, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.get_leader() == Some(p2));
    }

    #[test]
    fn leader_replace_non_member() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        set_leadership(&mut eng, 0, o1, Some(LeadershipTransferPolicies::ALL));

        add_to_org(&mut eng, 0, o1, p1, true, true).unwrap();
        assert!(change_leader(&mut eng, 0, o1, Some(p2)).is_err());
    }

    // you should be allowed to replace the leader with a dead person
    #[test]
    fn leader_replace_dead() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        set_leadership(&mut eng, 0, o1, Some(LeadershipTransferPolicies::ALL));

        quick_kill(&mut eng, 0, true, true, false, p2);
        add_to_org(&mut eng, 0, o1, p1, true, true).unwrap();
        add_to_org(&mut eng, 0, o1, p2, true, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.get_leader() == Some(p2));
    }

    // ensure that only the leader can use these abilities
    #[test]
    fn leader_only_ability() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        set_leadership(&mut eng, 0, o1, Some(LeadershipTransferPolicies::ALL));

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: IndexSet::new(),
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireLeader.into(),
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        add_to_org(&mut eng, 0, o1, p2, true, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                p1,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: p1 })
            )
            .is_err()
        );

        use_org_ability(
            &mut eng,
            0,
            p2,
            o1,
            a1,
            AbilityBehaviour::Gun(Gun { target_id: p1 }),
        )
        .unwrap();

        let p1_data = get_actor(&eng, p1).unwrap();
        assert!(p1_data.has_state(State::Dead))
    }

    // ensure that these abilities are used instantly
    #[test]
    fn no_vote_ability() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: IndexSet::new(),
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        use_org_ability(
            &mut eng,
            0,
            p1,
            o1,
            a1,
            AbilityBehaviour::Gun(Gun { target_id: p1 }),
        )
        .unwrap();

        let p1_data = get_actor(&eng, p1).unwrap();
        assert!(p1_data.has_state(State::Dead))
    }

    // ensure that these abilities are only used when votes go through
    #[test]
    fn vote_ability() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let p2 = add_player(&mut eng, 0, Role::Civilian, "p2");
        let p3 = add_player(&mut eng, 0, Role::Civilian, "p3");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: IndexSet::new(),
                    require_members: 0,
                    usage_policies: OrgAbilityPolicy::RequireVote.into(),
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        add_to_org(&mut eng, 0, o1, p2, false, true).unwrap();
        add_to_org(&mut eng, 0, o1, p3, false, true).unwrap();

        let response = use_org_ability(
            &mut eng,
            0,
            p1,
            o1,
            a1,
            AbilityBehaviour::Gun(Gun { target_id: p1 }),
        )
        .unwrap()
        .0;
        let ActionResponse::UseOrgAbility(data) = response else {
            unreachable!()
        };

        let poll_id = data.poll_id.unwrap();

        let p1_data = get_actor(&eng, p1).unwrap();
        assert!(!p1_data.has_state(State::Dead));

        add_vote(&mut eng, 0, poll_id, p1, false).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, true).unwrap();
        add_vote(&mut eng, 0, poll_id, p3, true).unwrap();

        let p1_data = get_actor(&eng, p1).unwrap();
        assert!(p1_data.has_state(State::Dead));
    }

    // they shouldnt be allowed to start votes and such if theyre not present
    #[test]
    fn dead_use_ability() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: IndexSet::new(),
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        quick_kill(&mut eng, 0, true, true, false, p1);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                p1,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: p1 }),
            )
            .is_err()
        );
    }

    #[test]
    fn role_requirements_has_role() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::RogueCivilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: indexset![Role::RogueCivilian],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        use_org_ability(
            &mut eng,
            0,
            p1,
            o1,
            a1,
            AbilityBehaviour::Gun(Gun { target_id: p1 }),
        )
        .unwrap();
    }

    #[test]
    fn role_requirements_doesnt_have_role() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::RogueCivilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: indexset![Role::ConArtist],
                    require_members: 0,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                p1,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: p1 }),
            )
            .is_err()
        )
    }

    #[test]
    fn member_requirements_met() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: indexset![],
                    require_members: 1,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        use_org_ability(
            &mut eng,
            0,
            p1,
            o1,
            a1,
            AbilityBehaviour::Gun(Gun { target_id: p1 }),
        )
        .unwrap();
    }

    #[test]
    fn member_requirements_unmet() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        let a1 = quick_org_ability(
            &mut eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Gun,
                variant: 0,
                org_id: o1,
                settings: OrgAbility {
                    require_roles: indexset![],
                    require_members: 2,
                    usage_policies: OrgAbilityPolicies::EMPTY,
                },
            },
        );
        force_charges(&mut eng, 0, a1, 100);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                p1,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: p1 }),
            )
            .is_err()
        );
    }

    // check that members have the passives of the org
    #[test]
    fn members_have_effective_passives() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        quick_passive(&mut eng, 0, o1, PassiveType::Wanted, false);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        assert!(
            actor_get_effective_passive(&eng, p1, |passive| { *passive == PassiveType::Wanted })
                .is_some()
        );
    }

    #[test]
    fn links_get_severed() {
        let mut eng = Engine::new();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        quick_passive(&mut eng, 0, o1, PassiveType::Wanted, false);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        remove_from_org(&mut eng, 0, o1, p1).unwrap();

        assert!(
            actor_get_effective_passive(&eng, p1, |passive| { *passive == PassiveType::Wanted })
                .is_none()
        );
    }

    // TODO:
    // blacklisting

    // blacklisting someone kicks them from the org if applicable and prevents them from rejoining
    #[test]
    fn blacklist_in_org() {}

    // blacklisting someone who is not in an org just removes their ability to join the org
    #[test]
    fn blacklist_not_in_org() {}
}
