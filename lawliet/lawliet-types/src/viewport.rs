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

// What kind of object a viewport belongs to, stated on MapViewport when the viewport is
// allocated.
//
// A fact about the viewport, and one a recipient may act on. It reaches a client only on
// admission, like everything else addressed there. The frontend server acts on exactly one
// variant — it never forwards a Log viewport to a client — and that is the only branch anyone is
// expected to need: every other kind of object announces itself through the content it sends.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ViewportKind {
    Channel,
    Bug,
    Poll,
    Passive,
    // The world-level singleton every present player has access to. Replaces both the old
    // BasePlayer stream and the deferred-command queue.
    Presence,
    // One per player, naming them as the sender of everything logged from them. The only viewport
    // that is an identity rather than an audience: nobody is ever granted access to it, and the
    // engine never delivers through it. yagami keys its message store by it, which is how an
    // autopsy answers who really sent something that was said under a borrowed display.
    Log,
}
