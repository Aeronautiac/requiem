// A civilian arrest opens a public vote to jail a player. Any present player may vote,
// the poll passes as soon as it reaches a majority, and a timeout leaves it inconclusive
// (nobody is jailed). On success the target is incarcerated for `civ_arrest_time` and then
// automatically released. Unlike a kidnapping there is no anonymous/public distinction.

use lawliet_types::{
    ability::{AbilityName, CivilianArrest},
    action::{Action, ActionActor, CreatePoll, TimedIncarceration},
    incarceration::IncarcerationSource,
    poll::{PollPolicy, PollSubject, PollVisibility, VoterPolicy},
};

use crate::{ability::AbilityInterface, action::ActionInterface, helpers::get_player};

impl AbilityInterface for CivilianArrest {
    fn ability_name(&self) -> AbilityName {
        AbilityName::CivilianArrest
    }

    fn handle(
        &mut self,
        eng: &mut crate::engine::Engine,
        ctx: &mut lawliet_types::action::ActionContext,
        _actor: &lawliet_types::action::ActionActor,
        ability: lawliet_types::common::AbilityKey,
        version: u8,
        mutate: bool,
    ) -> super::AbilityResult {
        // You can only arrest a player; the arrester need not be one.
        get_player(eng, self.target)?;

        // TODO: give the creator a way to cancel this poll before it resolves.
        Action::CreatePoll(CreatePoll {
            voter_policy: VoterPolicy::Present,
            visibility: PollVisibility::AllPresent,
            subject: PollSubject::CivilianArrest(self.target),
            update_policy: PollPolicy::Majority,
            timeout_policy: PollPolicy::AlwaysInconclusive,
            accept_payload: Box::new(Some(Action::TimedIncarceration(TimedIncarceration {
                victim_id: self.target,
                source: IncarcerationSource::Ability(ability),
                duration: eng.config.defaults.civ_arrest_time,
            }))),
            reject_payload: Box::new(None),
            duration: Some(eng.config.defaults.civ_arrest_vote_time),
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        Ok(super::AbilityStatus::Success)
    }
}
