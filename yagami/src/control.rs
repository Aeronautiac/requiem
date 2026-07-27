// Game administration: key management, and the invariant that keeps a game administrable.
//
// `may_manage` and the Supervise rules in `manage` are two halves of one guarantee -- the last
// Supervise holder can be neither revoked nor demoted, so at least one administrator always
// exists. They must change together or the invariant breaks, which is why they share a file.

use std::collections::HashSet;

use tokio_util::sync::CancellationToken;

use crate::{
    auth::{Capability, Key, KeyData, Privileges, Ticket, to_flags},
    delivery::narrow,
    game::GameEvent,
    state::{GameHandle, GameId, WrappedServerState, lock_state},
    wire::{ControlError, ControlOutcome, ControlResponse, GameControl},
};

// may the caller act on this target key? the single authority rule for every key-management control,
// kept in one place so a control added later cannot quietly skip it.
//
// the two rules below combine into the property that keeps a game administrable: a key holding
// Administer is reachable ONLY from a Supervise holder, and a Supervise holder cannot reach its own
// key -- so the LAST Supervise holder can be neither revoked nor demoted, by anyone. there is always
// at least one key holding Administer. changing either rule breaks that, so change them together.
//
// note this is not a count-the-admins guard: a lone PLAIN admin may still revoke itself and cut
// itself off. that is deliberate. the case being prevented is nobody holding admin, and the
// unreachable supervisor is what prevents it.
pub fn may_manage(
    game: &GameHandle,
    caller_key: &Key,
    supervises: bool,
    target: &Key,
) -> Result<(), ControlError> {
    let Some(target_data) = game.keys.get(target) else {
        return Err(ControlError::KeyNotFound);
    };

    if target == caller_key {
        // a plain admin has full authority over itself, up to and including revoking its own key and
        // cutting itself off. a supervisor deliberately does not: authority over admins sits above
        // admins, and that has to include the holder's own key or it is just a self-granted crown.
        return if supervises {
            Err(ControlError::CannotActOnSelf)
        } else {
            Ok(())
        };
    }

    // another administrator's key is reachable only from above.
    if target_data
        .privileges
        .capabilities
        .contains(Capability::Administer)
        && !supervises
    {
        return Err(ControlError::RequiresSupervise);
    }

    Ok(())
}

// withdraw a key and everything standing on it.
pub fn revoke_key(game: &mut GameHandle, key: &Key) {
    let Some(key_data) = game.keys.remove(key) else {
        return;
    };

    // ends every socket opened with this key at once -- their tokens are children of this one.
    key_data.cancel.cancel();

    for ticket in key_data.tickets {
        // the ledger entry has to go with the key: an unclaimed ticket that resolves to a key which
        // no longer exists is a live panic path in establish_ws_connection.
        game.tickets.remove(&ticket);

        // a claimed ticket still has its ConnHandle until the connection task's guard runs. mark it
        // so fan-out skips it in that window: it can no longer be resolved to a privilege set, and
        // both dispatch and attach treat an unresolvable LIVE connection as a broken invariant.
        if let Some(conn) = game.connections.get_mut(&ticket) {
            conn.dropped = true;
        }
    }
}

// Bring every live connection on a key into line with privileges that just changed.
//
// A connection's viewport cursor is not a cache of the current rules -- it is the ACCUMULATED
// result of walking the log under the rules in force at the time. A change makes it wrong, but the
// two directions are not symmetric and must not be handled the same way.
//
// NARROWING is an exit at the meta level. It removes future access and says nothing about the
// past, so it is applied in place, here, with no log and no batch. Nothing already delivered is
// retracted: client state is monotonic and the connection keeps the history it legitimately
// received. Without this the cursor's `access` would simply never drop the lost actors -- access
// is built from Enter/Exit commands in the log, and the Exit that would close it is now filtered
// out by the very change that should have ended it, so the connection would keep reading those
// viewports forever.
//
// WIDENING owes history, which means it needs the log -- and only the game task has that. So it
// goes out as an event carrying the PREVIOUS privilege set, and the delivery is the difference
// between old and new. See delivery::widen; it is emphatically not a replay.
//
// Both run on every change rather than being predicated on which direction it went: each is a
// no-op when there was nothing to do, and deciding here would mean re-deriving the comparison the
// two of them already make.
fn apply_privilege_change(game: &mut GameHandle, key: &Key, before: Privileges) {
    // split the borrow by field: cursors are edited through `connections` while `keys` is still
    // read for the ticket list and the new privilege set.
    let GameHandle {
        keys,
        connections,
        inbox,
        ..
    } = game;

    let Some(key_data) = keys.get(key) else {
        return;
    };

    for ticket in &key_data.tickets {
        if let Some(conn) = connections.get_mut(ticket)
            && let Some(cursor) = conn.cursor.as_mut()
        {
            narrow(cursor, &key_data.privileges);
        }

        // an unattached connection needs neither: its replay walks the whole log under the new
        // privileges and arrives at the same place.
        let _ = inbox.send(GameEvent::Widen {
            ticket: ticket.clone(),
            before: before.clone(),
        });
    }
}

// carry out one control. authority over the target is checked per-control rather than up front,
// because CreateKey has no target and EndGame's target is the game itself.
pub fn manage(
    game: &mut GameHandle,
    caller_key: &Key,
    supervises: bool,
    control: &GameControl,
    cancel: &CancellationToken,
) -> Result<ControlResponse, ControlError> {
    match control {
        GameControl::EndGame => {
            cancel.cancel();
            Ok(ControlResponse::Ended)
        }

        GameControl::CreateKey {
            actors,
            capabilities,
        } => {
            let capabilities = to_flags(capabilities);
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            let key = Key::generate();
            game.keys.insert(
                key.clone(),
                KeyData {
                    // child of the game token, so teardown takes this key's connections with it.
                    cancel: game.cancel.child_token(),
                    tickets: HashSet::new(),
                    privileges: Privileges {
                        actors: actors.clone(),
                        capabilities,
                    },
                },
            );

            Ok(ControlResponse::KeyCreated { key })
        }

        GameControl::RevokeKey { key } => {
            may_manage(game, caller_key, supervises, key)?;
            revoke_key(game, key);
            Ok(ControlResponse::KeyRevoked)
        }

        GameControl::SetCapabilities { key, capabilities } => {
            may_manage(game, caller_key, supervises, key)?;

            let capabilities = to_flags(capabilities);
            // gating on the grant rather than on holding it already, so a supervisor may still strip
            // Supervise from a key that has it.
            if capabilities.contains(Capability::Supervise) && !supervises {
                return Err(ControlError::CannotGrantSupervise);
            }

            // may_manage already established the key exists.
            let key_data = game
                .keys
                .get_mut(key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.capabilities = capabilities;

            // Granting or stripping Administer changes which viewports this key reads, so its
            // connections' cursors no longer describe reality. See apply_privilege_change.
            apply_privilege_change(game, key, before);

            Ok(ControlResponse::CapabilitiesSet)
        }

        GameControl::SetActorScope { key, actors } => {
            may_manage(game, caller_key, supervises, key)?;

            let key_data = game
                .keys
                .get_mut(key)
                .expect("target key resolved by may_manage");
            let before = key_data.privileges.clone();
            key_data.privileges.actors = actors.clone();

            // The cursor was built by walking the log under the OLD scope, so it holds no access
            // for viewports the newly-added actors are already in -- and still holds access for
            // ones the removed actors unlocked. See apply_privilege_change.
            apply_privilege_change(game, key, before);

            Ok(ControlResponse::ActorScopeSet)
        }
    }
}

// resolve the caller, gate on being an administrator at all, then hand off. the lock is held across
// the whole control so two admins cannot interleave halfway through each other's authority checks.
pub fn handle_control(
    state: &WrappedServerState,
    game_id: GameId,
    ticket: &Ticket,
    control: &GameControl,
    cancel: &CancellationToken,
) -> ControlOutcome {
    let mut server_state = lock_state(state);
    let Some(game) = server_state.games.get_mut(&game_id) else {
        return ControlOutcome::Denied; // game is gone
    };

    let Some(caller_key) = game.tickets.get(ticket).cloned() else {
        return ControlOutcome::Denied;
    };
    let Some(caller) = game.keys.get(&caller_key) else {
        return ControlOutcome::Denied;
    };

    // copied out so the caller's borrow ends before the target is mutated -- a caller acting on its
    // own key would otherwise be aliasing.
    let capabilities = caller.privileges.capabilities;

    if !capabilities.contains(Capability::Administer) {
        return ControlOutcome::Denied;
    }

    match manage(
        game,
        &caller_key,
        capabilities.contains(Capability::Supervise),
        control,
        cancel,
    ) {
        Ok(response) => ControlOutcome::Ok(response),
        Err(error) => ControlOutcome::Err(error),
    }
}
