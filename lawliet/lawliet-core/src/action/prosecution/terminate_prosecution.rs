/*
* SYSTEM / ADMIN ACTION
* Terminate a prosecution and clean up all associated state.
*
* On execution:
* - cancel the active timeout job (Custody or Trial phase)
* - if Trial: archive the trial channel
* - if Voting: cancel the poll
* - if lawyer selected: archive the lawyer channel
* - SetCustody { defendant, custody: false }
* - remove prosecution from world
*
* TODO: commands
*/

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        DestroyChannel, PollCleanup, SetCustody,
    },
    common::Version,
    engine::Engine,
    helpers::get_prosecution,
    poll::PollOutcome,
    prosecution::ProsecutionPhase,
};

use super::broadcast_prosecution_close;

pub use crate::action::{TerminateProsecution, TerminateProsecutionResponse};

impl ActionInterface for TerminateProsecution {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let prosecution = get_prosecution(eng, self.prosecution_id)?;
        let defendant_id = prosecution.defense.defendant;
        let lawyer_channel = prosecution.defense.lawyer.as_ref().map(|l| l.channel_id);

        let timeout_job_id = match &prosecution.phase {
            ProsecutionPhase::Custody { timeout_job_id, .. }
            | ProsecutionPhase::Trial { timeout_job_id, .. } => Some(*timeout_job_id),
            ProsecutionPhase::Voting { .. } => None,
        };
        let trial_channel = match &prosecution.phase {
            ProsecutionPhase::Trial { channel_id, .. }
            | ProsecutionPhase::Voting { channel_id, .. } => Some(*channel_id),
            _ => None,
        };
        let voting_poll = match &prosecution.phase {
            ProsecutionPhase::Voting { poll_id, .. } => Some(*poll_id),
            _ => None,
        };

        if let Some(job_id) = timeout_job_id
            && mutate
        {
            eng.jobs.cancel_id(job_id);
        }

        if let Some(poll_id) = voting_poll
            && eng.world.polls.contains_key(poll_id)
        {
            Action::PollCleanup(PollCleanup {
                poll_id,
                outcome: PollOutcome::Cancelled,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        if let Some(channel_id) = trial_channel {
            Action::DestroyChannel(DestroyChannel {
                channel_id,
                archive: true,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        if let Some(channel_id) = lawyer_channel {
            Action::DestroyChannel(DestroyChannel {
                channel_id,
                archive: true,
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Action::SetCustody(SetCustody {
            defendant_id,
            custody: false,
        })
        .handle(eng, ctx, &ActionActor::System, version, mutate)?;

        // Tell everyone the prosecution ended before dropping it.
        broadcast_prosecution_close(eng, ctx, self.prosecution_id, mutate);

        if mutate {
            eng.world.remove_prosecution(self.prosecution_id);
        }

        Ok(ActionResponse::TerminateProsecution(
            TerminateProsecutionResponse {},
        ))
    }
}
