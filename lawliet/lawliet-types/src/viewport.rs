use serde::{Deserialize, Serialize};

// A viewport is an opaque, engine-allocated identity that commands can be addressed to.
//
// That is all it is. Two separate relations hang off it and neither one defines it: actors are
// granted and denied access to it, and commands are directed at it. It is a routing primitive
// carrying no notion of *why* anyone has access — the object it belongs to is the authority on
// that and gates the viewport from its own visibility rule, never the other way round.
//
// Everything an object gates is addressed to the object's viewport, including the command that
// introduces the object itself (a channel's SetChannelLoggable, a bug's NewBug). Since gaining
// access backfills everything previously addressed there, a client always learns which object a
// viewport belongs to from the content it receives through it.
//
// Not everything owns one. An object whose audience is exactly some other object's rides that
// object's viewport instead of keeping a copy in step with it — polls do this, and are addressed
// to whatever they were put to.

// What kind of object a viewport belongs to, stated on MapViewport when the viewport is
// allocated.
//
// A fact about the viewport, and one a recipient may act on. It reaches a client only on
// admission, like everything else addressed there. No branch on it is required of anyone: every
// kind of object announces itself through the content it sends.
//
// Every variant here is an audience. The record is not one, and is not a viewport at all — see
// LogID.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ViewportKind {
    Channel,
    Bug,
    Passive,
    // Everything that HAPPENED, announced to everyone present. Emptied wholesale by a blackout,
    // which is what a blackout is — nobody exits presence, the world simply stops announcing. What
    // happened still happens, and is handed over in order when the blackout lifts.
    WorldEvents,
    // The world's structural facts: who exists, what day it is. Every present player holds it and
    // a blackout does not touch it, because a game that cannot tell you a player joined is broken
    // rather than dark.
    WorldData,
}
