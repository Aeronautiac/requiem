use lawliet_types::{
    actor::{ActorDisplay, Modifier, State},
    channel::{ChannelPerm, ChannelPermSet},
    common::ActorKey,
    passive::PassiveType,
};

use crate::{
    actor::Actor,
    channel::{Channel, ChannelProfile, ProfileOwnership},
    engine::Engine,
    helpers::{actor_get_effective_passive, get_actor, get_prosecution},
    prosecution::{ProsecutionPhase, TrialPhase},
};

// The rules themselves are defined in lawliet_types::channel, because an action carries one. Only
// what they decide lives here.
pub use lawliet_types::channel::{
    AlivePolicy, ContactPolicy, FixedPolicy, NewsPolicy, PermUpdatePolicy, PresencePolicy,
    TrialPolicy,
};

// Extract the single owner, if any, of a profile.
// Cannot be used within profiles with a multiple ownership model.
fn extract_single(profile: &ChannelProfile) -> Option<ActorKey> {
    let ProfileOwnership::Single(owner) = profile.ownership else {
        unreachable!()
    };
    owner
}

// these gates are all or nothing. they are composition utilities.
fn state_gate(actor: &Actor, state: State) -> ChannelPermSet {
    if actor.has_state(state) {
        ChannelPermSet::ALL
    } else {
        ChannelPermSet::EMPTY
    }
}

fn mod_gate(actor: &Actor, modifier: Modifier) -> ChannelPermSet {
    if actor.has_modifier(modifier) {
        ChannelPermSet::ALL
    } else {
        ChannelPermSet::EMPTY
    }
}

pub trait IPermUpdatePolicy {
    fn fits(&self, eng: &Engine, channel: &Channel, profile: &ChannelProfile) -> bool;
    fn eval(&self, eng: &Engine, channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet;
}

impl IPermUpdatePolicy for ContactPolicy {
    // This policy relies on the status of a single owner. It cannot adapt to multiple.
    fn fits(&self, _eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> bool {
        matches!(profile.ownership, ProfileOwnership::Single(_))
    }

    // If there is a current owner, gate on their status, otherwise, return an empty set.
    fn eval(&self, eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        if let Some(owner) = extract_single(profile) {
            let actor = get_actor(eng, owner).expect("Invalid actor key found within profile");
            let max = ChannelPerm::Send | ChannelPerm::View;
            max & !mod_gate(actor, Modifier::NoContact)
        } else {
            ChannelPermSet::EMPTY
        }
    }
}

impl IPermUpdatePolicy for FixedPolicy {
    fn fits(&self, _eng: &Engine, _channel: &Channel, _profile: &ChannelProfile) -> bool {
        true
    }

    fn eval(&self, _eng: &Engine, _channel: &Channel, _profile: &ChannelProfile) -> ChannelPermSet {
        self.perms
    }
}

impl IPermUpdatePolicy for NewsPolicy {
    // Gated on the owner having the News Anchor role. This policy cannot structurally handle
    // multiple owners.
    fn fits(&self, _eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> bool {
        matches!(profile.ownership, ProfileOwnership::Single(_))
    }

    // Everyone present listens; speaking on the news is a capability, not a role. It is granted by
    // holding NewsAccess (the anchor's own passive) or by being in the press conference (a guest the
    // anchor has let speak). The whole channel goes quiet under a blackout, which is the one world
    // channel that happens to — the news stopping is what the lights going out looks like from inside.
    fn eval(&self, eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        if eng.world.blackout {
            return ChannelPermSet::EMPTY;
        }
        let Some(owner) = extract_single(profile) else {
            return ChannelPermSet::EMPTY;
        };
        let actor = get_actor(eng, owner).expect("Invalid actor key found within profile");

        let mut max: ChannelPermSet = ChannelPerm::View.into();
        let can_speak = eng.world.news.press_conf.contains(&owner)
            || actor_get_effective_passive(eng, owner, |p| matches!(p, PassiveType::NewsAccess))
                .is_some();
        if can_speak {
            max |= ChannelPerm::Send;
        }
        max & !mod_gate(actor, Modifier::NoPresence)
    }
}

impl IPermUpdatePolicy for PresencePolicy {
    // Gated on the owner's presence, which is a single owner's business.
    fn fits(&self, _eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> bool {
        matches!(profile.ownership, ProfileOwnership::Single(_))
    }

    fn eval(&self, eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        let Some(owner) = extract_single(profile) else {
            return ChannelPermSet::EMPTY;
        };
        let actor = get_actor(eng, owner).expect("Invalid actor key found within profile");
        self.perms & !mod_gate(actor, Modifier::NoPresence)
    }
}

impl IPermUpdatePolicy for AlivePolicy {
    fn fits(&self, _eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> bool {
        matches!(profile.ownership, ProfileOwnership::Single(_))
    }

    fn eval(&self, eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        let Some(owner) = extract_single(profile) else {
            return ChannelPermSet::EMPTY;
        };
        let actor = get_actor(eng, owner).expect("Invalid actor key found within profile");
        let max = ChannelPerm::Send | ChannelPerm::View;
        max & !state_gate(actor, State::Dead)
    }
}

impl IPermUpdatePolicy for TrialPolicy {
    fn fits(&self, _eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> bool {
        matches!(profile.ownership, ProfileOwnership::Single(_))
    }

    // A trial is public, so watching costs presence and nothing more. Speaking costs holding the
    // floor: the phase names which side may talk, a debate opens it to both unless it is being held
    // for a host, and voting closes it to everyone while the room stays visible.
    //
    // Which name holds the floor is decided by DISPLAY, against the ones the trial announced its
    // sides under. That is what lets a prosecutor be anonymous without anything about them reading
    // differently: their own name answers no here, exactly like every other spectator's, and the
    // mask they were announced under answers yes.
    //
    // A frozen trial has no floor at all. It is stopped rather than hidden, so everyone keeps their
    // view of it and nobody speaks into a room the trial is not running in.
    fn eval(&self, eng: &Engine, _channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        let Some(owner) = extract_single(profile) else {
            return ChannelPermSet::EMPTY;
        };
        let actor = get_actor(eng, owner).expect("Invalid actor key found within profile");
        let watching =
            ChannelPermSet::from(ChannelPerm::View) & !mod_gate(actor, Modifier::NoPresence);

        let Ok(prosecution) = get_prosecution(eng, self.prosecution_id) else {
            return watching;
        };
        if prosecution.frozen(eng) {
            return watching;
        }

        let prosecuting = profile.display == prosecution.prosecution.prosecutor_display;
        let defending = profile.display == prosecution.defense.defendant_display
            || prosecution
                .defense
                .lawyer
                .as_ref()
                .is_some_and(|lawyer| profile.display == ActorDisplay::Raw(lawyer.actor_id));

        let holds_floor = match &prosecution.phase {
            ProsecutionPhase::Trial { phase, .. } => match phase {
                TrialPhase::Prosecutor(_) => prosecuting,
                TrialPhase::Defense(_) => defending,
                // A debate held for a host is over in everything but the confirmation.
                TrialPhase::Debate { .. } => {
                    !prosecution.pending_advance && (prosecuting || defending)
                }
            },
            // Custody has no trial channel, and voting keeps the room open with the floor closed.
            ProsecutionPhase::Custody { .. } | ProsecutionPhase::Voting { .. } => false,
        };

        // Speaking is gated on presence too: the floor is no use to someone who is not there.
        if holds_floor && !watching.is_empty() {
            watching | ChannelPerm::Send
        } else {
            watching
        }
    }
}

// Static dispatch for performance.
// It is worth the bookkeeping. This runs after every action.
impl IPermUpdatePolicy for PermUpdatePolicy {
    fn fits(&self, eng: &Engine, channel: &Channel, profile: &ChannelProfile) -> bool {
        match self {
            PermUpdatePolicy::Fixed(pol) => pol.fits(eng, channel, profile),
            PermUpdatePolicy::Contact(pol) => pol.fits(eng, channel, profile),
            PermUpdatePolicy::News(pol) => pol.fits(eng, channel, profile),
            PermUpdatePolicy::Presence(pol) => pol.fits(eng, channel, profile),
            PermUpdatePolicy::Alive(pol) => pol.fits(eng, channel, profile),
            PermUpdatePolicy::Trial(pol) => pol.fits(eng, channel, profile),
        }
    }

    fn eval(&self, eng: &Engine, channel: &Channel, profile: &ChannelProfile) -> ChannelPermSet {
        match self {
            PermUpdatePolicy::Fixed(pol) => pol.eval(eng, channel, profile),
            PermUpdatePolicy::Contact(pol) => pol.eval(eng, channel, profile),
            PermUpdatePolicy::News(pol) => pol.eval(eng, channel, profile),
            PermUpdatePolicy::Presence(pol) => pol.eval(eng, channel, profile),
            PermUpdatePolicy::Alive(pol) => pol.eval(eng, channel, profile),
            PermUpdatePolicy::Trial(pol) => pol.eval(eng, channel, profile),
        }
    }
}
