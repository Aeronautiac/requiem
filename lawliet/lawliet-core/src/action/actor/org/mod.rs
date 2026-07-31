pub mod add_to_org;
pub mod change_org_leader;
pub mod create_and_give_org_ability;
pub mod create_org;
pub mod give_org_ability;
pub mod remove_from_org;
pub mod resign_leadership;
pub mod set_blacklist_status;
pub mod set_leadership;
pub mod set_og_status;
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
    use lawliet_types::command::{Command, CommandRecipient};

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
        helpers::{actor_get_effective_passive, get_actor, get_channel, get_org},
        passive::PassiveType,
        test_helpers::*,
    };

    #[test]
    fn basic_addition() {
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        quick_kill(&mut eng, 0, true, true, false, p1);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.has_member(p1));
    }

    #[test]
    fn remove_dead() {
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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

    // AddToOrg fixes what somebody was when they joined; this is the only thing that moves it
    // afterwards, in either direction.
    #[test]
    fn change_og_status() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        add_to_org(&mut eng, 0, o1, p1, false, false).unwrap();

        let og = |eng: &Engine| get_org(eng, o1).unwrap().members.get(&p1).unwrap().og;
        assert!(!og(&eng));

        set_og_status(&mut eng, 0, o1, p1, true).unwrap();
        assert!(og(&eng));

        set_og_status(&mut eng, 0, o1, p1, false).unwrap();
        assert!(!og(&eng));
    }

    // Personal info, like a role or a true name: you know your own standing and admin can inspect
    // anyone's, but the org is told only that somebody joined — never what they joined as.
    #[test]
    fn og_status_is_private_to_the_member() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        let org_channel = get_org(&eng, o1).unwrap().channel_id;
        let org_viewport = get_channel(&eng, org_channel).unwrap().viewport;

        let ctx = add_to_org(&mut eng, 0, o1, p1, false, true).unwrap().1;

        let recipients: Vec<_> = ctx
            .commands
            .iter()
            .filter(|c| matches!(&c.cmd, Command::OgStatus { .. }))
            .map(|c| c.recipient.clone())
            .collect();
        assert_eq!(
            recipients,
            vec![CommandRecipient::Actor(p1), CommandRecipient::System]
        );
        assert!(!recipients.contains(&CommandRecipient::Viewport(org_viewport)));
    }

    // OG is a property OF a membership, so there is nothing for it to hang on without one. Note
    // this is also true of someone who has LEFT — rejoining starts them at whatever AddToOrg says.
    #[test]
    fn og_status_needs_a_membership() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        assert!(set_og_status(&mut eng, 0, o1, p1, true).is_err());

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        remove_from_org(&mut eng, 0, o1, p1).unwrap();
        assert!(set_og_status(&mut eng, 0, o1, p1, true).is_err());
    }

    #[test]
    fn already_member() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        assert!(add_to_org(&mut eng, 0, o1, p1, false, true).is_err());
    }

    #[test]
    fn kick_non_member() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        assert!(remove_from_org(&mut eng, 0, o1, p1).is_err());
    }

    // replace an existing leader with a new leader
    #[test]
    fn leader_replace() {
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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

        add_vote(&mut eng, 0, poll_id, p1, REJECT).unwrap();
        add_vote(&mut eng, 0, poll_id, p2, ACCEPT).unwrap();
        add_vote(&mut eng, 0, poll_id, p3, ACCEPT).unwrap();

        let p1_data = get_actor(&eng, p1).unwrap();
        assert!(p1_data.has_state(State::Dead));
    }

    // they shouldnt be allowed to start votes and such if theyre not present
    #[test]
    fn dead_use_ability() {
        let mut eng = started_engine();
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

    // An org's abilities belong to the org, and reaching for one is something only its members may
    // do. Nothing else in the chain says so: the ability is owned by the ORG, so UseAbility's
    // ownership check passes for anyone who names it, and the presence and member-count gates are
    // about the org's condition rather than about who is asking.
    #[test]
    fn non_member_cannot_use_ability() {
        let mut eng = started_engine();
        let member = add_player(&mut eng, 0, Role::Civilian, "member");
        let outsider = add_player(&mut eng, 0, Role::Civilian, "outsider");
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
        add_to_org(&mut eng, 0, o1, member, false, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                outsider,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: member }),
            )
            .is_err()
        );
        assert!(!get_actor(&eng, member).unwrap().has_state(State::Dead));
    }

    // The vote path is the same question asked earlier: opening one is itself a use, so an outsider
    // must not be able to put the org's ability to a vote of its members either.
    #[test]
    fn non_member_cannot_open_an_ability_vote() {
        let mut eng = started_engine();
        let member = add_player(&mut eng, 0, Role::Civilian, "member");
        let outsider = add_player(&mut eng, 0, Role::Civilian, "outsider");
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
        add_to_org(&mut eng, 0, o1, member, false, true).unwrap();

        assert!(
            use_org_ability(
                &mut eng,
                0,
                outsider,
                o1,
                a1,
                AbilityBehaviour::Gun(Gun { target_id: member }),
            )
            .is_err()
        );
        assert!(eng.world.polls.is_empty());
    }

    #[test]
    fn role_requirements_has_role() {
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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
        let mut eng = started_engine();
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

    // A bar that let a sitting member stay would not be one.
    #[test]
    fn blacklist_in_org() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();

        set_blacklist_status(&mut eng, 0, o1, p1, true).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(org.is_blacklisted(p1));
        assert!(!org.has_member(p1));
        // Kicked through RemoveFromOrg, so the membership is torn down the same way it is by any
        // other kick — the passive link to the org included.
        assert!(
            actor_get_effective_passive(&eng, p1, |passive| { *passive == PassiveType::Wanted })
                .is_none()
        );
        assert!(add_to_org(&mut eng, 0, o1, p1, false, true).is_err());
    }

    // Barring somebody who was never in is the ordinary case: nothing to kick, just no way back in.
    #[test]
    fn blacklist_not_in_org() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);

        set_blacklist_status(&mut eng, 0, o1, p1, true).unwrap();

        assert!(get_org(&eng, o1).unwrap().is_blacklisted(p1));
        assert!(add_to_org(&mut eng, 0, o1, p1, false, true).is_err());
    }

    // Lifting the bar makes someone eligible again; it never puts them back. Rejoining is a
    // decision the org has to take separately.
    #[test]
    fn unblacklisting_does_not_readmit() {
        let mut eng = started_engine();
        let p1 = add_player(&mut eng, 0, Role::Civilian, "p1");
        let o1 = add_org(&mut eng, 0, OrganizationName::NULL);
        add_to_org(&mut eng, 0, o1, p1, false, true).unwrap();
        set_blacklist_status(&mut eng, 0, o1, p1, true).unwrap();

        set_blacklist_status(&mut eng, 0, o1, p1, false).unwrap();

        let org = get_org(&eng, o1).unwrap();
        assert!(!org.is_blacklisted(p1));
        assert!(!org.has_member(p1));

        add_to_org(&mut eng, 0, o1, p1, false, false).unwrap();
        assert!(get_org(&eng, o1).unwrap().has_member(p1));
    }
}
