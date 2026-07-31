/*
* Prosecution lifecycle: Custody → Trial → Voting
*
* Participants:
* - Prosecutor: has displays (stored in the trial channel, not here)
* - Defendant: same; currently always raw (no anonymous defendant mechanic)
* - Lawyer (optional): defendant may select one during custody. No selection before custody
*   ends means no lawyer.
* - Autonomous flag: if false, a host must confirm the two major phase boundaries — Custody ->
*   Trial and Debate -> Voting. Movement WITHIN the trial (grace -> presentation, one side's slot
*   to the other's) is never held: those are the trial running, not the trial ending, and a host
*   confirming each of them would mean approving every time someone starts talking.
*
*   A held boundary sets pending_advance and cancels the timer that would have fired again. The
*   prosecution then waits indefinitely for an admin AdvanceProsecution. Players see the wait
*   (awaiting_host on the phase snapshot) so a stalled trial reads as deliberate rather than broken.
*
* Custody period:
*   Ends when both sides signal ready OR the timeout fires. Non-autonomous: held here.
*
* Trial period:
*   Each side gets a presentation slot. The active side starts in a grace subphase with its
*   own timer. Sending any message during grace immediately advances to presentation and
*   replaces the timer with the presentation duration. If the grace timer fires instead,
*   the advance still happens. After both presentations, a debate period begins. If one side
*   signals done the timer is shortened; if both signal done the debate ends immediately.
*   Non-autonomous: held at the end of the debate — but the floor closes either way, since a held
*   debate is over in everything but the confirmation.
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
* Disruption. Losing visibility — a blackout is one cause, not the only one — FREEZES the
* prosecution, in every phase. Nothing is restarted and no channel is replaced or hidden; the
* prosecution simply stops moving and resumes where it left off.
*
* A frozen prosecution does three things. It holds its advances — both sides signalling ready is
* not enough to leave a phase — which is a second and independent reason to hold alongside
* pending_advance, so a frozen non-autonomous trial is waiting on both. It stops its timers, which
* is UpdateTimers' doing: the phases carry a TimerKey gated on the same viewport `frozen` reads,
* so a frozen prosecution's clock is stopped by the same sweep that stops every other one. And it
* closes the floor, the same way a held debate does — UpdateProsecutionChannels reads the floor off
* prosecution state, so this is one more input to it rather than anything new.
*
* A trial channel is a world channel in spirit, and is only kept out of WorldChannelConfig because
* that is keyed by a singleton name while trials are many and come and go. The closure is derived
* per prosecution because that is where the state is, not because a trial channel is a different
* kind of thing from News.
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

use crate::{
    ActorKey, ChannelKey, PollKey,
    actor::ActorDisplay,
    common::{TimerKey, ViewportKey},
    engine::Engine,
};

pub use lawliet_types::prosecution::{
    ProsecutionPhaseView, ProsecutionSide, ProsecutionSource, TrialPhaseView, TrialSubphaseView,
};

#[derive(Debug)]
pub struct Lawyer {
    pub actor_id: ActorKey,
    // None once the private channel has been closed, which happens when voting begins. Who defended
    // the accused outlives the line they used to talk on.
    pub channel_id: Option<ChannelKey>,
}

#[derive(Debug)]
pub struct ProsecutionProsecutor {
    pub prosecutor: ActorKey,
    // How the prosecutor appears in the trial channel. Seeded onto the channel member when the
    // trial channel is created (Anonymous/Silent prosecutions pass a Mysterious display here).
    pub prosecutor_display: ActorDisplay,
}

#[derive(Debug)]
pub struct ProsecutionDefense {
    pub defendant: ActorKey,
    // How the defendant appears in the trial channel. Seeded onto the channel member when the
    // trial channel is created; currently always Raw (no anonymous-defendant mechanic).
    pub defendant_display: ActorDisplay,
    pub lawyer: Option<Lawyer>,
}

#[derive(Debug)]
pub enum TrialSubphase {
    Grace,
    Presentation,
}

impl TrialSubphase {
    pub fn view(&self) -> TrialSubphaseView {
        match self {
            Self::Grace => TrialSubphaseView::Grace,
            Self::Presentation => TrialSubphaseView::Presentation,
        }
    }
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
        timer: TimerKey,
    },

    // timer tracks the current active countdown and is replaced on every subphase transition.
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
        timer: TimerKey,
    },

    Voting {
        poll_id: PollKey,
        // retained from the trial phase so the channel stays viewable (send revoked) during
        // the vote, and the frontend can keep rendering the trial alongside the verdict poll.
        channel_id: ChannelKey,
    },
}

#[derive(Debug)]
pub struct Prosecution {
    pub source: ProsecutionSource,
    pub prosecution: ProsecutionProsecutor,
    pub defense: ProsecutionDefense,
    pub phase: ProsecutionPhase,
    pub autonomous: bool,
    // The condition for leaving this phase has been met, but the prosecution is non-autonomous and
    // is holding at the boundary until a host confirms. Only the two major boundaries set it —
    // Custody -> Trial and Debate -> Voting. Subphase movement inside the trial never waits.
    //
    // Set here rather than as a phase of its own because the prosecution has not moved: it is still
    // in custody, or still in the debate. What changed is that nothing further will happen on its
    // own. Held debates also close the floor, which UpdateProsecutionChannels reads off this.
    pub pending_advance: bool,
}

impl Prosecution {
    // The audience a prosecution plays to. A trial is a public spectacle, so that is the world's
    // events viewport — the same one its snapshot is broadcast on, which is what makes the two
    // agree by construction: whoever cannot be told the trial moved is not watching it move.
    pub fn viewport(&self, eng: &Engine) -> ViewportKey {
        eng.world.events_viewport
    }

    // Nobody is watching, so nothing happens. Derived rather than stored: a freeze is not a state
    // the prosecution enters and has to be let out of again, it is the absence of an audience, and
    // storing it would mean every way an audience can come and go needing to remember to say so.
    //
    // The timer on this prosecution's phase is gated on this same viewport, so its clock stops on
    // exactly the terms this does, without either being told about the other.
    pub fn frozen(&self, eng: &Engine) -> bool {
        eng.world
            .get_viewport(self.viewport(eng))
            .is_none_or(|viewport| viewport.is_empty())
    }

    // The countdown driving this phase, if it has one. Voting has none: the verdict poll runs its
    // own, and the prosecution is waiting on the poll rather than on the clock.
    pub fn timer(&self) -> Option<TimerKey> {
        match &self.phase {
            ProsecutionPhase::Custody { timer, .. } | ProsecutionPhase::Trial { timer, .. } => {
                Some(*timer)
            }
            ProsecutionPhase::Voting { .. } => None,
        }
    }

    // Both sides have said they are finished with the phase they are in. A standing fact about the
    // phase rather than a moment, which is what lets a prosecution that could not act on it when it
    // was set — frozen, or already waiting on a host — act on it as soon as that clears.
    pub fn both_signalled(&self) -> bool {
        match &self.phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                ..
            } => *prosecutor_ready && *defense_ready,
            ProsecutionPhase::Trial {
                phase:
                    TrialPhase::Debate {
                        prosecutor_done,
                        defense_done,
                    },
                ..
            } => *prosecutor_done && *defense_done,
            _ => false,
        }
    }

    // Map the internal phase to the client-facing view and locate the trial channel (None during
    // custody, before the channel exists).
    pub fn phase_view(&self) -> (ProsecutionPhaseView, Option<ChannelKey>) {
        match &self.phase {
            ProsecutionPhase::Custody {
                prosecutor_ready,
                defense_ready,
                ..
            } => (
                ProsecutionPhaseView::Custody {
                    prosecutor_ready: *prosecutor_ready,
                    defense_ready: *defense_ready,
                    awaiting_host: self.pending_advance,
                },
                None,
            ),
            ProsecutionPhase::Trial {
                phase, channel_id, ..
            } => {
                let trial = match phase {
                    TrialPhase::Prosecutor(sub) => TrialPhaseView::Prosecutor(sub.view()),
                    TrialPhase::Defense(sub) => TrialPhaseView::Defense(sub.view()),
                    TrialPhase::Debate {
                        prosecutor_done,
                        defense_done,
                    } => TrialPhaseView::Debate {
                        prosecutor_done: *prosecutor_done,
                        defense_done: *defense_done,
                        awaiting_host: self.pending_advance,
                    },
                };
                (ProsecutionPhaseView::Trial(trial), Some(*channel_id))
            }
            ProsecutionPhase::Voting { channel_id, .. } => {
                (ProsecutionPhaseView::Voting, Some(*channel_id))
            }
        }
    }
}
