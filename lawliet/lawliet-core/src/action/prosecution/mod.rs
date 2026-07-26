pub mod advance_prosecution;
pub mod cull_prosecutions;
pub mod prosecution_vote_res;
pub mod select_lawyer;
pub mod set_custody;
pub mod signal_ready;
pub mod start_prosecution;
pub mod terminate_prosecution;
pub mod update_prosecution_channels;
pub mod update_prosecutions;

use lawliet_types::command::Command;

use crate::{
    action::ActionContext,
    common::ProsecutionKey,
    engine::Engine,
    helpers::{cmd_world_event, get_prosecution},
};

// Broadcast a prosecution's client-facing snapshot to everyone present, plus the System mirror.
//
// The ordered timeline is what matters here, and the presence viewport preserves it for free: a
// player who loses presence exits and stops receiving updates, and re-entry replays every one
// they missed in order. That is what the old deferred queue and the "frozen view" notice were
// both approximating — the queue held the updates, the notice told the client its state was
// stale. Neither is needed now: absence is stated by the exit, and the client already knows
// nothing more will arrive until it re-enters.
pub(crate) fn broadcast_prosecution(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    let Ok(prosecution) = get_prosecution(eng, prosecution_id) else {
        return;
    };
    let (phase, trial_channel) = prosecution.phase_view();
    let prosecutor_display = prosecution.prosecution.prosecutor_display;
    let defendant_display = prosecution.defense.defendant_display;

    cmd_world_event(
        eng,
        ctx,
        Command::UpdateProsecution {
            prosecution_id,
            prosecutor_display,
            defendant_display,
            phase,
            trial_channel,
        },
    );
}

// Tell everyone a prosecution has ended. Addressed the same way as the snapshot, so for an
// absent player it lands after any updates they have yet to receive.
pub(crate) fn broadcast_prosecution_close(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    mutate: bool,
) {
    if !mutate {
        return;
    }
    cmd_world_event(eng, ctx, Command::CloseProsecution { prosecution_id });
}
