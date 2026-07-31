// Cut the world's information off at the source, and take the blame for it.
//
// The world goes dark for a fixed span: the world-events viewport empties, so nothing that happens
// is announced to anybody, and every world channel marked blackout_blocked closes. Nobody loses
// presence and nothing is retracted — players keep everything they were already told, and receive
// everything they missed, in order, the moment the lights come back.
//
// What survives the dark is deliberate. Existence and the clock ride world data, so players are
// still told who joined and what day it is; they can work out from a channel roster that somebody
// is gone. They are not told that they died, or how, until it is over.
//
// The cost is that using it names you. Whoever used it is marked Wanted, and if that was an org,
// so is every member it had at the moment it was used — so leaving afterwards does not wash it
// off, while anyone who joins later is exposed through the org's own mark only for as long as they
// stay. That is what SilentProsecute exists to punish, and it is what makes this worth holding.

use lawliet_types::{
    ability::{AbilityName, Blackout},
    action::{Action, ActionActor, CreateAndGivePassive, SetBlackout},
    passive::PassiveType,
};
use smallvec::SmallVec;

use crate::{
    ability::{AbilityInterface, AbilityStatus},
    action::ActionInterface,
    common::ActorKey,
    helpers::{actor_id, get_org},
};

impl AbilityInterface for Blackout {
    fn ability_name(&self) -> AbilityName {
        AbilityName::Blackout
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        actor: &lawliet_types::action::ActionActor,
        _ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        // Read the roster before anything else runs, so the mark lands on exactly who was in the
        // org when the lights went out. A player using this directly marks only themselves, and
        // System marks nobody — an admin taking the world dark is not an accusation.
        let user = actor_id(actor);
        let marked: SmallVec<[ActorKey; 8]> = user
            .into_iter()
            .chain(
                user.and_then(|id| get_org(eng, id).ok())
                    .into_iter()
                    .flat_map(|org| org.members.keys().copied())
                    .collect::<SmallVec<[ActorKey; 8]>>(),
            )
            .collect();

        Action::SetBlackout(SetBlackout { active: true }).handle(
            eng,
            ctx,
            &ActionActor::System,
            version,
            mutate,
        )?;

        // Non-transferrable and permanent: it is not a resource that can be handed on, and there
        // is no undoing having done this.
        //
        // Members of a marked org already inherit Wanted through their org link, which is why the
        // explicit copies matter — the link alone would let today's members walk away clean
        // tomorrow.
        for target in marked {
            Action::CreateAndGivePassive(CreateAndGivePassive {
                passive_type: PassiveType::Wanted,
                transferrable: false,
                actor_id: target,
                volatile: false,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(AbilityStatus::Success)
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexSet;
    use lawliet_types::{
        ability::{AbilityBehaviour, AbilityName, Blackout},
        action::CreateAndGiveOrgAbility,
        channel::{ChannelPerm, ChannelPermSet},
        organization::OrgAbility,
    };

    use crate::{
        common::{AbilityKey, ActorKey, Time},
        config::{actor::organization::OrganizationName, role::Role, world::WorldChannelName},
        engine::Engine,
        helpers::actor_get_effective_passive,
        passive::PassiveType,
        test_helpers::{
            add_org, add_player, add_to_org, init_engine, null_action, quick_org_ability,
            use_org_ability,
        },
    };

    fn org_with_ability(eng: &mut Engine, member: ActorKey) -> (ActorKey, AbilityKey) {
        let org = add_org(eng, 0, OrganizationName::NULL);
        let ability = quick_org_ability(
            eng,
            0,
            CreateAndGiveOrgAbility {
                ability_name: AbilityName::Blackout,
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

    fn go_dark(eng: &mut Engine, time: Time, user: ActorKey, org: ActorKey, ability: AbilityKey) {
        use_org_ability(
            eng,
            time,
            user,
            org,
            ability,
            AbilityBehaviour::Blackout(Blackout {}),
        )
        .unwrap();
    }

    fn in_events(eng: &Engine, id: ActorKey) -> bool {
        eng.world
            .get_viewport(eng.world.events_viewport)
            .unwrap()
            .contains(id)
    }

    fn in_data(eng: &Engine, id: ActorKey) -> bool {
        eng.world
            .get_viewport(eng.world.data_viewport)
            .unwrap()
            .contains(id)
    }

    // Everything the player may do here under any of their names.
    fn perms(eng: &Engine, player: ActorKey, channel: WorldChannelName) -> ChannelPermSet {
        let id = eng.world.world_channel_map[&channel];
        eng.world
            .get_channel(id)
            .unwrap()
            .owned_profiles(player)
            .fold(ChannelPermSet::EMPTY, |acc, profile| acc | profile.perms)
    }

    // The whole mechanism: the events viewport empties, and nothing else about the player moves.
    #[test]
    fn going_dark_empties_the_events_viewport_only() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let user = add_player(&mut eng, 0, Role::Civilian, "user");
        let other = add_player(&mut eng, 0, Role::Civilian, "other");
        let (org, ability) = org_with_ability(&mut eng, user);

        assert!(in_events(&eng, other));

        go_dark(&mut eng, 1, user, org, ability);

        assert!(eng.world.blackout);
        assert!(!in_events(&eng, user));
        assert!(!in_events(&eng, other));
        // Presence itself never moved — existence and the clock still reach everyone.
        assert!(in_data(&eng, user));
        assert!(in_data(&eng, other));
    }

    #[test]
    fn a_blacked_out_channel_loses_every_permission() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let user = add_player(&mut eng, 0, Role::Civilian, "user");
        let (org, ability) = org_with_ability(&mut eng, user);

        assert!(perms(&eng, user, WorldChannelName::News).contains(ChannelPerm::View));

        go_dark(&mut eng, 1, user, org, ability);

        assert_eq!(
            perms(&eng, user, WorldChannelName::News),
            ChannelPermSet::EMPTY
        );
        // Only channels configured for it go dark; talking is not news.
        assert!(perms(&eng, user, WorldChannelName::General).contains(ChannelPerm::Send));
    }

    // Everyone in the org when the lights went out is stained permanently; the org's own mark is
    // what exposes whoever joins afterwards, and only while they stay.
    #[test]
    fn the_org_and_its_members_at_the_time_are_marked_wanted() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let user = add_player(&mut eng, 0, Role::Civilian, "user");
        let latecomer = add_player(&mut eng, 0, Role::Civilian, "latecomer");
        let (org, ability) = org_with_ability(&mut eng, user);

        go_dark(&mut eng, 1, user, org, ability);
        add_to_org(&mut eng, 2, org, latecomer, false, false).unwrap();

        let wanted = |eng: &Engine, id: ActorKey| {
            actor_get_effective_passive(eng, id, |p| matches!(p, PassiveType::Wanted)).is_some()
        };

        assert!(wanted(&eng, org));
        assert!(wanted(&eng, user));
        // Reached through the org link rather than owned outright.
        assert!(wanted(&eng, latecomer));
    }

    // A blackout always ends on its own.
    #[test]
    fn it_lifts_when_the_timer_fires() {
        let mut eng = Engine::new();
        init_engine(&mut eng);
        let user = add_player(&mut eng, 0, Role::Civilian, "user");
        let (org, ability) = org_with_ability(&mut eng, user);

        go_dark(&mut eng, 1, user, org, ability);
        assert!(eng.world.blackout);

        let lifts_at = 1 + eng.config.defaults.blackout_duration;
        null_action(&mut eng, lifts_at);

        assert!(!eng.world.blackout);
        assert!(in_events(&eng, user));
        assert!(perms(&eng, user, WorldChannelName::News).contains(ChannelPerm::View));
    }
}
