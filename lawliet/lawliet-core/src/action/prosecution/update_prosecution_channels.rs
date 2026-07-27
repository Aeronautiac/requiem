/*
* SYSTEM ACTION
* Re-evaluate trial channel membership for a single prosecution based on its current phase.
*
* Trials are public: every present player is granted View. Send is restricted to the side
* whose slot is active:
*   Trial Prosecutor(_) → prosecutor
*   Trial Defense(_)    → defendant + lawyer (if selected)
*   Trial Debate        → both sides, unless the debate is held for a host, which closes the floor
*                         before the confirmation arrives
*   Voting              → nobody (view only; the trial stays visible alongside the verdict poll)
*
* Custody has no trial channel yet, so the trial half is a no-op for it.
*
* The lawyer's private channel is re-derived here too, and gates on CONTACT rather than presence:
* it is person-to-person, and custody deliberately leaves contact intact so the accused can still
* reach counsel. It exists from selection until voting begins, when AdvanceProsecution destroys it
* (as does TerminateProsecution).
*
* Displays: the key participants (prosecutor, defendant, lawyer) are seeded onto the channel
* with their proper displays and empty perms when the trial channel is first created. Here we
* only re-derive perms — a member that already exists keeps its seeded display, and any newly
* added present player (a spectator) is given a Raw display of themselves.
*
* Called on trial channel creation, on every subphase transition (active side changes), and on
* actor state changes (presence gained/lost). Non-present players that were already members are
* downgraded to empty perms rather than removed, matching the kidnap channel pattern.
*
* Commands come from the SetMembers this issues; it emits none directly.
*
* TODO: optimizations — this re-derives the whole roster on every call.
*/

use indexmap::indexset;
use smallvec::{SmallVec, smallvec};

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionInterface, ActionResponse, ActionResult,
        SetMember,
    },
    actor::{ActorDisplay, ActorType, modifier::Modifier},
    channel::{ChannelMember, ChannelPermission, ChannelPermissions},
    common::{ActorKey, ChannelKey, Version},
    engine::Engine,
    helpers::{get_actor, get_channel, get_prosecution},
    prosecution::{ProsecutionPhase, TrialPhase},
};

struct MemberUpdate {
    player_id: ActorKey,
    settings: ChannelMember,
}

pub use crate::action::{UpdateProsecutionChannels, UpdateProsecutionChannelsResponse};

impl ActionInterface for UpdateProsecutionChannels {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        let lawyer_line = {
            let prosecution = get_prosecution(eng, self.prosecution_id)?;
            let defendant = prosecution.defense.defendant;
            prosecution
                .defense
                .lawyer
                .as_ref()
                .and_then(|lawyer| Some((lawyer.channel_id?, defendant, lawyer.actor_id)))
        };

        // The lawyer's private line is person-to-person, so it gates on contact where the trial
        // channel below gates on presence. Custody deliberately leaves contact intact — the accused
        // can still reach counsel — while death, incarceration and kidnapping all set NoContact.
        if let Some((channel_id, defendant, lawyer_id)) = lawyer_line {
            for player_id in [defendant, lawyer_id] {
                let blocked = get_actor(eng, player_id)?.has_modifier(Modifier::NoContact);
                let perms = if blocked {
                    ChannelPermissions::EMPTY
                } else {
                    ChannelPermission::Send | ChannelPermission::View
                };

                Action::SetMember(SetMember {
                    player_id,
                    channel_id,
                    settings: Some(ChannelMember {
                        perms,
                        displays: indexset![ActorDisplay::Raw(player_id)],
                    }),
                })
                .handle(eng, ctx, &ActionActor::System, version, mutate)?;
            }
        }

        let prosecution = get_prosecution(eng, self.prosecution_id)?;

        // Determine the trial channel and the set of actors allowed to send this phase.
        let mut senders: SmallVec<[ActorKey; 4]> = smallvec![];
        let channel_id: ChannelKey = match &prosecution.phase {
            ProsecutionPhase::Custody { .. } => {
                return Ok(ActionResponse::UpdateProsecutionChannels(
                    UpdateProsecutionChannelsResponse {},
                ));
            }
            ProsecutionPhase::Trial {
                phase, channel_id, ..
            } => {
                match phase {
                    TrialPhase::Prosecutor(_) => {
                        senders.push(prosecution.prosecution.prosecutor);
                    }
                    TrialPhase::Defense(_) => {
                        senders.push(prosecution.defense.defendant);
                        if let Some(lawyer) = &prosecution.defense.lawyer {
                            senders.push(lawyer.actor_id);
                        }
                    }
                    // A debate held for a host is over in everything but the confirmation, so the
                    // floor is already closed — nobody is added as a sender.
                    TrialPhase::Debate { .. } if prosecution.pending_advance => {}
                    TrialPhase::Debate { .. } => {
                        senders.push(prosecution.prosecution.prosecutor);
                        senders.push(prosecution.defense.defendant);
                        if let Some(lawyer) = &prosecution.defense.lawyer {
                            senders.push(lawyer.actor_id);
                        }
                    }
                }
                *channel_id
            }
            ProsecutionPhase::Voting { channel_id, .. } => *channel_id,
        };

        let player_ids: SmallVec<[ActorKey; 16]> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, a)| matches!(a.actor_type, ActorType::Player(_)).then_some(id))
            .collect();

        let channel = get_channel(eng, channel_id)?;

        let mut updates: SmallVec<[MemberUpdate; 16]> = smallvec![];
        for player_id in player_ids {
            let player = get_actor(eng, player_id).expect("player id from world must be valid");
            let present = !player.has_modifier(Modifier::NoPresence);
            let existing = channel.members.get(&player_id);

            // Skip non-present players who were never in the channel: nothing to grant or revoke.
            if !present && existing.is_none() {
                continue;
            }

            let perms = if present {
                if senders.contains(&player_id) {
                    ChannelPermission::Send | ChannelPermission::View
                } else {
                    ChannelPermission::View.into()
                }
            } else {
                ChannelPermissions::EMPTY
            };

            // Preserve a seeded display if the member already exists; otherwise a spectator
            // joining the public trial is shown raw.
            let displays = match existing {
                Some(member) => member.displays.clone(),
                None => indexset![ActorDisplay::Raw(player_id)],
            };

            updates.push(MemberUpdate {
                player_id,
                settings: ChannelMember { perms, displays },
            });
        }

        for update in updates {
            Action::SetMember(SetMember {
                player_id: update.player_id,
                channel_id,
                settings: Some(update.settings),
            })
            .handle(eng, ctx, &ActionActor::System, version, mutate)?;
        }

        Ok(ActionResponse::UpdateProsecutionChannels(
            UpdateProsecutionChannelsResponse {},
        ))
    }
}
