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

use lawliet_types::command::{Command, CommandRecipient};
use smallvec::SmallVec;

use crate::{
    action::ActionContext,
    actor::{ActorType, modifier::Modifier},
    common::{ActorKey, ProsecutionKey},
    engine::Engine,
    helpers::{cmd_all_deferred, get_prosecution, get_prosecution_mut},
};

// Broadcast a prosecution's client-facing snapshot. The snapshot goes to System + BasePlayer
// immediately and to every player deferred on presence (absent players replay the full ordered
// timeline when presence returns — updates are never dropped, unlike polls). Separately, any
// player we last sent a live update to (the dirty set) who has since lost presence gets a
// FreezeProsecutionView notice so their UI marks the state frozen. Only runs on the mutate pass.
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

    cmd_all_deferred(
        eng,
        ctx,
        Command::UpdateProsecution {
            prosecution_id,
            prosecutor_display,
            defendant_display,
            phase,
            trial_channel,
        },
        Modifier::NoPresence.into(),
        true,
        true,
        mutate,
    );

    let present: SmallVec<[ActorKey; 16]> = eng
        .world
        .actors
        .iter()
        .filter_map(|(id, a)| {
            (matches!(a.actor_type, ActorType::Player(_)) && !a.has_modifier(Modifier::NoPresence))
                .then_some(id)
        })
        .collect();

    let prosecution = get_prosecution(eng, prosecution_id).expect("just read above");
    let frozen: SmallVec<[ActorKey; 8]> = prosecution
        .dirty
        .iter()
        .filter(|id| !present.contains(id))
        .copied()
        .collect();
    for id in &frozen {
        ctx.push_cmd(
            Command::FreezeProsecutionView { prosecution_id },
            CommandRecipient::Actor(*id),
            eng.time,
        );
    }

    let prosecution = get_prosecution_mut(eng, prosecution_id).expect("just read above");
    prosecution.dirty = present.into_iter().collect();
}

// Tell everyone a prosecution has ended: System + BasePlayer immediately, players deferred so the
// close lands in order after any pending updates. The prosecution (and its dirty set) is removed
// by the caller.
pub(crate) fn broadcast_prosecution_close(
    eng: &mut Engine,
    ctx: &mut ActionContext,
    prosecution_id: ProsecutionKey,
    mutate: bool,
) {
    cmd_all_deferred(
        eng,
        ctx,
        Command::CloseProsecution { prosecution_id },
        Modifier::NoPresence.into(),
        true,
        true,
        mutate,
    );
}
