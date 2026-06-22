/*
* Prosecution lifecycle: Custody → Trial → Voting
*
* Participants:
* - Prosecutor: has displays (stored in the trial channel, not here)
* - Defendant: same; currently always raw (no anonymous defendant mechanic)
* - Lawyer (optional): defendant may select one during custody. No selection before custody
*   ends means no lawyer.
* - Autonomous flag: if false, a host must manually approve every phase transition.
*
* Custody period:
*   Ends when both sides signal ready OR the timeout fires. In non-autonomous mode,
*   host approval is also required before advancing.
*
* Trial period:
*   Each side gets a presentation slot. The active side starts in a grace subphase with its
*   own timer. Sending any message during grace immediately advances to presentation and
*   replaces the timer with the presentation duration. If the grace timer fires instead,
*   the advance still happens. After both presentations, a debate period begins. If one side
*   signals done the timer is shortened; if both signal done the debate ends immediately.
*   When the debate timer expires, speaking privileges are revoked for both sides regardless
*   of host input. Host approval is still required to advance to the voting phase if
*   non-autonomous. Advancing out of the trial phase entirely also requires host approval
*   if non-autonomous.
*
* Voting period:
*   An anonymous poll is added to the trial channel. Guilty majority → defendant executed;
*   otherwise they are released. The vote runs for a fixed duration.
*
* Termination conditions:
* - Custody or Trial: prosecutor or defendant gains NoPresence → immediate termination.
*   (Lawyer state is irrelevant after selection.)
* - Source ability (if applicable) is destroyed (within any phase), or prosecutor is not in the source ability's
*   owning organization during the custody or trial phase.
* - Voting: defendant dies → immediate termination.
*
* Disruption rules (not yet implemented):
* - If trial visibility is lost (e.g. blackout), the trial restarts when it returns.
* - If poll visibility is lost during voting, the voting period is extended by the
*   duration of the disruption.
*
* Other rules:
* - Custody wiretaps the defendant (a custody bug is created by SetCustody).
* - Selecting a lawyer opens a private channel between defendant and lawyer, open until
*   the voting period begins.
* - The only uniqueness constraint is on defendants: a player may not be the defendant in
*   more than one active prosecution at a time. There is no restriction on how many
*   prosecutions a player may initiate, nor on prosecuting someone while being prosecuted
*   yourself.
*/

// Termination note:
// Archived channels/prosecutions are marked as non-interactive on the frontend but remain
// visible. Deferred commands handle the case where a player receives a visibility grant
// for an already-archived object — the frontend should label it archived and block interaction.

use crate::{ActorKey, ChannelKey, PollKey, common::JobID};

pub use lawliet_types::prosecution::ProsecutionSource;

#[derive(Debug)]
pub struct Lawyer {
    pub actor_id: ActorKey,
    pub channel_id: ChannelKey,
}

#[derive(Debug)]
pub struct ProsecutionDefense {
    pub defendant: ActorKey,
    pub lawyer: Option<Lawyer>,
}

#[derive(Debug)]
pub enum TrialSubphase {
    Grace,
    Presentation,
}

#[derive(Debug)]
pub enum TrialPhase {
    Prosecutor(TrialSubphase),
    Defense(TrialSubphase),
    // one done → timer shortened; both done → immediately end (host approval still applies)
    // timer expiry revokes speaking privileges for both sides regardless of host input
    Debate {
        prosecutor_done: bool,
        defense_done: bool,
    },
}

#[derive(Debug)]
pub enum ProsecutionPhase {
    // Advances when both ready flags are set OR timeout fires.
    // In non-autonomous mode, host must also call AdvanceProsecution to confirm.
    Custody {
        prosecutor_ready: bool,
        defense_ready: bool,
        timeout_job_id: JobID,
    },

    // timeout_job_id tracks the current active timer and is replaced on every subphase transition.
    //
    // Grace → Presentation: first message from the active side OR grace timeout fires;
    //   cancel the grace job and schedule the presentation timer.
    // Presentation → next phase: presentation timeout fires.
    // Debate → Voting: timeout fires (speaking privileges revoked immediately), OR one done
    //   flag shortens it, OR both done ends it immediately. Non-autonomous: host approval
    //   required to advance to the voting phase, but privilege revocation happens regardless.
    Trial {
        phase: TrialPhase,
        channel_id: ChannelKey,
        timeout_job_id: JobID,
    },

    Voting {
        poll_id: PollKey,
    },
}

// TODO:
// add non-autonomous behaviour

#[derive(Debug)]
pub struct Prosecution {
    pub source: ProsecutionSource,
    pub prosecutor: ActorKey,
    pub defense: ProsecutionDefense,
    pub phase: ProsecutionPhase,
    pub autonomous: bool,
}
