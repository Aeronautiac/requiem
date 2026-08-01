// CURRENT PROBLEMS:
// - anonymous prosecutors are exposed indirectly because they are only given anonymous displays, but
// not raw member displays, and they appear to be the ONLY person who doesn't have a regular member
// display. this isnt solved by manually pushing a channel display command. a bad actor can still
// look at command positions to see if a certain command did not follow the typical pattern, and
// from that they can deduce that this person is L for instance.
// - being the lawyer of a trial as well as the prosector leads to weird overlap scenarios, but this is
// an entirely plausible occurance. Say for instance you were just messing around, and you just
// outright hired your prosecutor as your lawyer. Also say for instance that L anonymously
// prosecuted you, and the lawyer you pick happens to be L's hidden identity.
// - Permission gates are currently too scattered, and many places are performing what is
// essentially the same basic job on them (gating permissions on some modifier). There are so
// many update actions with the same or similar shapes. This worked for simpler scenarios, but
// fights you when things get more complex, and the more content you add, the more update
// actions you have to create, inject into other actions, and synchronize with the world state.
// - A host can only grant you overrides to a world channel. Nothing else. This isn't symmetric,
// and it's kind of weird.

// CHANNEL REWORK:
//
// A new primitive is being introduced along with this rework: Log
// A log is essentially a ligher viewport. It is a unique id rather than a stateful object.
// It inherits the "history" behaviour of a viewport without having engine state. It is used for
// things like autopsy and tap in.
//
// A channel is a primitive which facilitates the majority of communications between players.
// A channel stores a viewport, a log, its loggability, a set of profiles, and a set of members.
//
// The membership viewport is what MEMBERS of that channel see if they may access a profile with view permissions.
// The log is a message/event store. It contains everything in the channel which passed the loggability gate and is
// OBSERVED rather than obtained as background data.
// Whether or not a message is logged is gated on the person who sent it. Not the profile. This means that person A
// might be under the radar, and they can send a message in any profil5e, and it wont be saved, but if person B shares a profile
// with person A and sends a message on the same profile, it WILL be saved.
//
// A profile stores a set of permissions, a display, its visibility phase, and whether or not it may be shared.
// Profiles which cannot be shared may be possessed by at most one member at a time.
// A profile may start visible or invisible.
// A profile MUST be revealed to the channel viewport BEFORE a message is sent. Sending a message reveals an invisible profile.
//
// A profile stores a set of members who have sent messages using that profile. This can be edited. It is basically an event log.
// This allows you to do some useful things, examples being:
// - On send, advance a prosecution to the next phase.
// - At the end of a public kidnapping, reveal everyone who spoke in the public kidnapping channel.
//
// A profile stores an optional update policy.
//
// An update policy takes in a channel reference, a profile reference, an engine reference, and spits out
// a new permissions set for that profile.
//
// If an update policy is supplied, but the policy is structurally incompatible with the rest of the profile, the profile's construction
// is rejected outright.
// If a profile was constructed with no initial policy, but then an attempt to update the policy was made, it can either accept or reject,
// but the profile will not be destroyed on rejection.
// Policies can compose, but this is not structurally explicit, rather it is a convenient outcome of what they are, fundamentally.
//
// A policy is expressed as a struct trait with two functions and an optional context enum, where:
// - One function checks if this policy may apply to this profile.
// - One function applies the policy.
//
// A few examples of policies:
// NewsPolicy: "If my owner has the role of News Anchor, then they get every permission" -> BitSet A
// ContactPolicy: "If my owner does not have the NoContact modifier, then they get every permission" -> BitSet B
// NewsPolicy & ContactPolicy: A & B
// Fixed context(PERMISSION SET): "I always get this specific set of permissions"
//
// Permission policies alone are not enough to fully collapse every channel update action into one general mechanism.
// Think about the case where:
// A trial begins -> all players are given profiles on creation -> a new player is added -> existing people's permissions update, new player is never made a member
//
// We also need some kind of automatic membership mechanism on the channel level rather than on the individual level.
// Every kind of channel in the game cleanly falls into two categories:
// 1. Channels that everyone is given some "base" form of membership in.
// 2. Channels with unique membership behaviours.
//
// A policy system is tempting here, but it would be overengineered. A simple "base profile" system on the channel level would
// solve every problem effectively.
//
// A channel has an optional base profile which every player receives a copy of.
// The member struct stores the id of this profile after creation, meaning that once the profile is created, it can be freely modified
// without issue.
//
// MIGRATION:
// Many of the simpler scenarios are collapsed entirely.
// The more complex scenarios transform from permissions + displays + membership synchronization to
// simpler profile management setups.

pub mod policies;

use indexmap::{IndexMap, IndexSet};
use lawliet_types::common::{ActorKey, LogID, ProfileKey};
use slotmap::SlotMap;
use smallvec::SmallVec;

use crate::common::ViewportKey;

pub use lawliet_types::actor::ActorDisplay;
pub use lawliet_types::channel::{
    BlueprintDisplayKind, ChannelKind, ChannelPerm, ChannelPermSet, ChannelProfileView,
    PermUpdatePolicy, ProfileBlueprint, ProfileOwners,
};

#[derive(Debug)]
pub enum ProfileOwnership {
    Single(Option<ActorKey>),
    Multiple(IndexSet<ActorKey>),
}

impl ProfileOwnership {
    // Whoever currently wears this profile, whichever model it follows. Callers that only need to
    // walk the owners should not have to care which one it is.
    pub fn owners(&self) -> SmallVec<[ActorKey; 4]> {
        match self {
            ProfileOwnership::Single(owner) => owner.iter().copied().collect(),
            ProfileOwnership::Multiple(owners) => owners.iter().copied().collect(),
        }
    }

    pub fn contains(&self, actor: ActorKey) -> bool {
        match self {
            ProfileOwnership::Single(owner) => *owner == Some(actor),
            ProfileOwnership::Multiple(owners) => owners.contains(&actor),
        }
    }
}

#[derive(Debug)]
pub struct ChannelMember {
    pub profiles: IndexSet<ProfileKey>,
    pub base_profile: Option<ProfileKey>, // which profile originated as the "base" profile, if
                                          // applicable?
}

#[derive(Debug)]
pub struct ChannelProfile {
    pub visible: bool,
    pub display: ActorDisplay,
    pub perms: ChannelPermSet,
    pub perm_policy: PermUpdatePolicy,
    pub sent: IndexSet<ActorKey>,
    pub ownership: ProfileOwnership,
    // Whether this name could ever belong to somebody other than whoever holds it now. Distinct
    // from the ownership model, which says how many may wear it AT ONCE: a name can be exclusive
    // and still be handed on, and a name can be shared and still belong to a fixed set of people.
    //
    // False means the name is bound to its holder and has no life without them, so losing them is
    // the end of it. That is what makes it safe to destroy a departing member's names without
    // asking what they were for.
    pub transferrable: bool,
}

// What does this need to do?
// It needs to:
// - track its owner
// - handle state transitions
impl ChannelProfile {
    // A name nobody wears yet, holding no permissions until its policy is first evaluated.
    //
    // Built rather than inserted, because a policy is offered the finished article to accept or
    // refuse before the channel ever takes it — and that check needs the engine, which is the
    // caller's to hand over. See policies::IPermUpdatePolicy::fits.
    pub fn new(
        display: ActorDisplay,
        visible: bool,
        shared: bool,
        transferrable: bool,
        perm_policy: PermUpdatePolicy,
    ) -> Self {
        ChannelProfile {
            visible,
            display,
            perms: ChannelPermSet::EMPTY,
            perm_policy,
            sent: IndexSet::new(),
            transferrable,
            ownership: if shared {
                ProfileOwnership::Multiple(IndexSet::new())
            } else {
                ProfileOwnership::Single(None)
            },
        }
    }

    // this runs when someone sends a message using this profile.
    // it returns true iff the action led to a visibility state transition.
    // callers are expected to use this flag to handle command routing and similar.
    pub fn on_send(&mut self, actor: ActorKey) -> bool {
        let old_vis = self.visible;
        self.visible = true;
        self.sent.insert(actor);
        !old_vis && self.visible
    }

    // Put this profile on someone. Returns false if nothing changed, which for a singly-owned
    // profile includes somebody else already wearing it: a name only one person can be wearing is
    // not handed to a second one, it is refused.
    pub fn grant(&mut self, actor: ActorKey) -> bool {
        match &mut self.ownership {
            ProfileOwnership::Single(owner) => match owner {
                Some(_) => false,
                None => {
                    *owner = Some(actor);
                    true
                }
            },
            ProfileOwnership::Multiple(owners) => owners.insert(actor),
        }
    }

    // Take this profile off someone. Returns false if they were not wearing it.
    pub fn revoke(&mut self, actor: ActorKey) -> bool {
        match &mut self.ownership {
            ProfileOwnership::Single(owner) => match owner {
                Some(held) if *held == actor => {
                    *owner = None;
                    true
                }
                _ => false,
            },
            ProfileOwnership::Multiple(owners) => owners.swap_remove(&actor),
        }
    }

    pub fn clear_sent(&mut self) {
        self.sent.clear();
    }
}

#[derive(Debug)]
pub struct Channel {
    pub loggable: bool,
    pub viewport: ViewportKey,
    pub log: LogID,
    pub profiles: SlotMap<ProfileKey, ChannelProfile>,
    // Set for a channel everyone belongs to; None for one whose membership an action owns.
    pub base_profile: Option<ProfileBlueprint>,
    pub members: IndexMap<ActorKey, ChannelMember>,
}

// expose an api for profile construction, deletion, and membership additions/removals as well as
// member to profile linking.
impl Channel {
    pub fn new(
        loggable: bool,
        viewport: ViewportKey,
        log: LogID,
        base_profile: Option<ProfileBlueprint>,
    ) -> Self {
        Channel {
            loggable,
            viewport,
            log,
            base_profile,
            profiles: SlotMap::with_key(),
            members: IndexMap::new(),
        }
    }

    pub fn get_profile(&self, id: ProfileKey) -> Option<&ChannelProfile> {
        self.profiles.get(id)
    }

    pub fn get_profile_mut(&mut self, id: ProfileKey) -> Option<&mut ChannelProfile> {
        self.profiles.get_mut(id)
    }

    pub fn get_member(&self, actor: ActorKey) -> Option<&ChannelMember> {
        self.members.get(&actor)
    }

    pub fn is_member(&self, actor: ActorKey) -> bool {
        self.members.contains_key(&actor)
    }

    // Every name an actor holds here. What a permission question about a person reduces to: holding
    // two names means being able to do whatever either of them can.
    pub fn owned_profiles(&self, actor: ActorKey) -> impl Iterator<Item = &ChannelProfile> {
        self.members
            .get(&actor)
            .into_iter()
            .flat_map(|member| member.profiles.iter())
            .filter_map(|id| self.profiles.get(*id))
    }

    pub fn profile_view(&self, id: ProfileKey) -> Option<ChannelProfileView> {
        let profile = self.profiles.get(id)?;
        Some(ChannelProfileView {
            profile_id: id,
            display: profile.display,
            perms: profile.perms,
        })
    }

    // Every name the room can see. What a viewer's roster of this channel is; an invisible profile
    // is deliberately absent until something reveals it.
    pub fn visible_profiles(&self) -> SmallVec<[ChannelProfileView; 8]> {
        self.profiles
            .iter()
            .filter(|(_, profile)| profile.visible)
            .map(|(id, profile)| ChannelProfileView {
                profile_id: id,
                display: profile.display,
                perms: profile.perms,
            })
            .collect()
    }

    // Every name an actor may speak as here. Almost always one.
    pub fn accessible_profiles(&self, actor: ActorKey) -> SmallVec<[ChannelProfileView; 4]> {
        self.members
            .get(&actor)
            .into_iter()
            .flat_map(|member| member.profiles.iter())
            .filter_map(|id| self.profile_view(*id))
            .collect()
    }

    // Who the channel's viewport should contain. The one question here that is about the actor
    // rather than the profile: an audience is made of people, so holding View under any name puts
    // you in it. The member list is the authority and the viewport is the projection of it.
    pub fn viewers(&self) -> IndexSet<ActorKey> {
        self.members
            .iter()
            .filter(|(_, member)| {
                member.profiles.iter().any(|id| {
                    self.profiles
                        .get(*id)
                        .is_some_and(|profile| profile.perms.contains(ChannelPerm::View))
                })
            })
            .map(|(actor, _)| *actor)
            .collect()
    }

    // Take a name out of the channel, dropping every membership that rested on it alone.
    pub fn remove_profile(&mut self, id: ProfileKey) -> Option<ChannelProfile> {
        let profile = self.profiles.remove(id)?;
        for owner in profile.ownership.owners() {
            self.unlink(owner, id);
        }
        Some(profile)
    }

    // Hand a name to an actor, which is also what makes them a member. False when nothing changed,
    // so callers emit exactly one command per genuine change.
    pub fn grant_profile(&mut self, actor: ActorKey, id: ProfileKey) -> bool {
        let Some(profile) = self.profiles.get_mut(id) else {
            return false;
        };
        if !profile.grant(actor) {
            return false;
        }

        self.members
            .entry(actor)
            .or_insert_with(|| ChannelMember {
                profiles: IndexSet::new(),
                base_profile: None,
            })
            .profiles
            .insert(id);
        true
    }

    // Take a name back. Losing the last one ends the membership outright: there is no being in a
    // channel under no name at all.
    pub fn revoke_profile(&mut self, actor: ActorKey, id: ProfileKey) -> bool {
        let Some(profile) = self.profiles.get_mut(id) else {
            return false;
        };
        if !profile.revoke(actor) {
            return false;
        }
        self.unlink(actor, id);
        true
    }

    // Record which of a member's profiles came from the channel's blueprint, so the sweep knows it
    // has already been handed out and does not stamp a second one.
    pub fn set_base_profile(&mut self, actor: ActorKey, id: ProfileKey) {
        if let Some(member) = self.members.get_mut(&actor) {
            member.base_profile = Some(id);
        }
    }

    // Drop one profile from a member's side of the link, and the member with it if that was their
    // last. The profile's own owner set is the caller's business; this is only the index back.
    fn unlink(&mut self, actor: ActorKey, id: ProfileKey) {
        let Some(member) = self.members.get_mut(&actor) else {
            return;
        };
        member.profiles.swap_remove(&id);
        if member.profiles.is_empty() {
            self.members.swap_remove(&actor);
        }
    }
}
