/*
* SYSTEM ACTION
* Re-evaluate trial channel membership for a single prosecution based on its current phase.
*
* Trials are public: every present player is granted View. Send is restricted to the side
* whose slot is active:
*   Trial Prosecutor(_) → prosecutor
*   Trial Defense(_)    → defendant + lawyer (if selected)
*   Trial Debate        → both sides
*   Voting              → nobody (view only; the trial stays visible alongside the verdict poll)
*
* Custody has no trial channel yet, so this action is a no-op for it.
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
* TODO: commands & optimizations
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

        let prosecution = get_prosecution(eng, self.prosecution_id)?;

        // Determine the trial channel and the set of actors allowed to send this phase.
        let mut senders: SmallVec<[ActorKey; 4]> = smallvec![];
        let channel_id: ChannelKey = match &prosecution.phase {
            ProsecutionPhase::Custody { .. } => {
                return Ok(ActionResponse::UpdateProsecutionChannels(
                    UpdateProsecutionChannelsResponse {},
                ));
            }
            ProsecutionPhase::Trial { phase, channel_id, .. } => {
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
