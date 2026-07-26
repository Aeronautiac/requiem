use serde::{Deserialize, Serialize};

// A viewport is an opaque, engine-allocated identity that commands can be addressed to.
//
// That is all it is. Two separate relations hang off it and neither one defines it: actors are
// granted and denied access to it, and commands are directed at it. It is a routing primitive
// carrying no notion of *why* anyone has access — the object it belongs to is the authority on
// that and gates the viewport from its own visibility rule, never the other way round.
//
// Everything an object gates is addressed to the object's viewport, including the command that
// introduces the object itself (a channel's SetChannelLoggable, a poll's UpdatePoll, a bug's
// NewBug). Since gaining access backfills everything previously addressed there, a client
// always learns which object a viewport belongs to from the content it receives through it.

// What kind of object a viewport belongs to. Display only — it exists so an opaque key is
// legible in a log. NEITHER the server nor the client may branch on it; if either needs to,
// something upstream has gone wrong.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ViewportKind {
    Channel,
    Bug,
    Poll,
    Passive,
    // The world-level singleton every present player has access to. Replaces both the old
    // BasePlayer stream and the deferred-command queue.
    Presence,
}
