// go through every channel,
// create base profiles,
// go through every profile,
// apply policies

use indexmap::IndexSet;
use lawliet_types::{action::ActionResponse, actor::ActorDisplay, channel::ChannelPermSet};
use smallvec::SmallVec;

use crate::{
    action::{
        Action, ActionActor, ActionContext, ActionError, ActionInterface, ActionResult,
        CreateAndGiveProfile,
    },
    actor::ActorType,
    channel::{BlueprintDisplayKind, policies::IPermUpdatePolicy},
    common::{ActorKey, ChannelKey, ProfileKey, Version},
    engine::Engine,
    helpers::{
        cmd_channel_roster, cmd_profile_access, get_channel, get_channel_mut, sync_viewport,
    },
};

pub use crate::action::{UpdateChannels, UpdateChannelsResponse};

const MISSING_CHANNEL: &str = "channel vanished mid-sweep: engine invariant violated";

impl ActionInterface for UpdateChannels {
    fn handle(
        &mut self,
        eng: &mut Engine,
        ctx: &mut ActionContext,
        actor: &ActionActor,
        version: Version,
        mutate: bool,
    ) -> ActionResult {
        actor.admin_or_system()?;

        // Nothing here can fail, and a validate pass emits nothing, so there is no answer this
        // could give that the mutate pass would not give again a moment later.
        if !mutate {
            return Ok(ActionResponse::UpdateChannels(UpdateChannelsResponse {}));
        }

        let players: SmallVec<[ActorKey; 16]> = eng
            .world
            .actors
            .iter()
            .filter_map(|(id, a)| matches!(a.actor_type, ActorType::Player(_)).then_some(id))
            .collect();
        let channel_ids: SmallVec<[ChannelKey; 16]> = eng.world.channels.keys().collect();

        for channel_id in channel_ids {
            hand_out_base_profiles(eng, ctx, actor, version, mutate, channel_id, &players)?;
            apply_policies(eng, ctx, channel_id);
        }

        Ok(ActionResponse::UpdateChannels(UpdateChannelsResponse {}))
    }
}

// Stamp the channel's blueprint out for every player who has not been handed one yet.
//
// This is the whole of membership for a channel that has a blueprint, and it is why a player who
// joins the world long after a trial began still ends up in it: the sweep asks who is owed a copy
// rather than being told, so no site has to remember to say so.
fn hand_out_base_profiles(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    actor: &ActionActor,
    version: Version,
    mutate: bool,
    channel_id: ChannelKey,
    players: &[ActorKey],
) -> Result<(), ActionError> {
    let Some(blueprint) = get_channel(eng, channel_id)
        .expect(MISSING_CHANNEL)
        .base_profile
    else {
        return Ok(());
    };

    let owed: SmallVec<[ActorKey; 16]> = {
        let channel = get_channel(eng, channel_id).expect(MISSING_CHANNEL);
        players
            .iter()
            .copied()
            .filter(|id| {
                channel
                    .get_member(*id)
                    .is_none_or(|member| member.base_profile.is_none())
            })
            .collect()
    };

    for player_id in owed {
        let display = match blueprint.display_kind {
            BlueprintDisplayKind::OwnerRaw => ActorDisplay::Raw(player_id),
        };

        // Through the action rather than the channel directly, so a base profile is announced,
        // granted and told to its owner by exactly the same code as any other. A blueprint decides
        // who gets a name here, not what a name is.
        let response = Action::CreateAndGiveProfile(CreateAndGiveProfile {
            channel_id,
            player_id,
            display,
            visible: blueprint.start_visible,
            shared: false,
            // A base profile is the copy handed to one player. It is theirs and nobody else's, so
            // it goes when they do.
            transferrable: false,
            perm_policy: blueprint.perm_policy,
        })
        .handle(eng, ctx, actor, version, mutate)?;
        let ActionResponse::CreateAndGiveProfile(data) = response else {
            unreachable!()
        };

        // Recorded once the grant has made them a member, so the next sweep does not hand them a
        // second copy. From here it is an ordinary profile and may be edited freely.
        get_channel_mut(eng, channel_id)
            .expect(MISSING_CHANNEL)
            .set_base_profile(player_id, data.profile_id);
    }

    Ok(())
}

// Re-decide every profile's permissions from its policy.
//
// Quiet by design: a profile whose answer has not moved says nothing at all, which is what makes
// running this after every single action affordable.
//
// Who holds which name does not move here, only what those names permit — but that is carried by
// both halves of the protocol, so both are restated. A grant runs before the policy deciding what
// it is worth, so the access it announced said EMPTY; without this the owner is left holding a seat
// they cannot use, watching the room see them as able to speak.
fn apply_policies(eng: &mut Engine, ctx: &mut ActionContext, channel_id: ChannelKey) {
    let changes: SmallVec<[(ProfileKey, ChannelPermSet); 8]> = {
        let channel = get_channel(eng, channel_id).expect(MISSING_CHANNEL);
        channel
            .profiles
            .iter()
            .filter_map(|(id, profile)| {
                let perms = profile.perm_policy.eval(eng, channel, profile);
                (perms != profile.perms).then_some((id, perms))
            })
            .collect()
    };
    if changes.is_empty() {
        return;
    }

    // Collected as the writes happen, and deduped: one owner holding two changed names is owed one
    // message, because an access is the whole set of what they hold rather than one name's entry.
    let mut told: IndexSet<ActorKey> = IndexSet::new();
    for (profile_id, perms) in changes {
        let profile = get_channel_mut(eng, channel_id)
            .expect(MISSING_CHANNEL)
            .get_profile_mut(profile_id)
            .expect("profile enumerated a moment ago is gone: engine invariant violated");
        profile.perms = perms;
        told.extend(profile.ownership.owners());
    }

    // The audience is a projection of who holds View and every answer above has just settled.
    let (viewport, viewers) = {
        let channel = get_channel(eng, channel_id).expect(MISSING_CHANNEL);
        (channel.viewport, channel.viewers())
    };
    sync_viewport(eng, ctx, viewport, viewers, true);

    // Once, after everything, rather than per profile: the roster is the whole visible set, so
    // sending it for each name that moved would be the same message several times over.
    cmd_channel_roster(eng, ctx, channel_id);

    // Directed, so it still reaches an owner the resync above has just taken out of the viewport —
    // losing everything a name permitted is exactly the case they most need told.
    for owner in told {
        cmd_profile_access(eng, ctx, channel_id, owner);
    }
}
