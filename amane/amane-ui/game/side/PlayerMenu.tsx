// The one profile menu, opened from any name via `ui.openPlayerMenu`. Everything it offers acts as
// the current viewer against the clicked player: contact, press-conference management (only with
// the NewsControl passive — the engine enforces it too), and the full admin controls when viewing
// as System.
import type { Action } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { statusLabels } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { statusAccent, tint } from "../../style.ts";
import { useGameUi, useView } from "../game_ui.ts";
import { Entity } from "../Name.tsx";
import { PlayerAdminControls } from "./PlayerAdminControls.tsx";

export function PlayerMenu() {
  const ui = useGameUi();
  const view = useView();
  const session = useSession();
  const flash = useFlash();

  const id = ui.player_menu;
  const isAdmin = ui.viewer === "System";
  const statuses = id !== null ? statusLabels(view.actor_statuses.get(id) ?? 0) : [];

  const contactAbilities = [...view.abilities.entries()].filter(([, av]) => av.name === "Contact");
  // The engine gates press-conference management on the NewsControl passive; mirror that so the
  // action only shows to someone who could actually use it.
  const canManageConf = [...view.passives.values()].some((p) => p.type === "NewsControl");
  const inConf = id !== null && view.press_conf.has(id);
  const hasActions = contactAbilities.length > 0 || canManageConf || isAdmin;

  async function run(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: viewActor(ui.viewer), timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }
  function contact() {
    const entry = contactAbilities[0];
    if (!entry || id === null) return;
    run(
      { UseAbility: { ability_id: slotKeyFromString(entry[0]), ability_args: { Contact: { target_id: slotKeyFromString(id) } } } },
      "Contact sent.",
    );
  }
  function toggleConf() {
    if (id === null) return;
    run(
      { PressConfAccess: { target_id: slotKeyFromString(id), has_access: !inConf } },
      inConf ? "Removed from press conference." : "Added to press conference.",
    );
  }

  return (
    <Modal
      open={id !== null}
      onClose={() => ui.openPlayerMenu(null)}
      title={
        id !== null ? (
          <span className="flex flex-wrap items-center gap-1.5">
            <Entity of={{ kind: "player", id }} view={view} chip menu={false} />
            {statuses.map((s) => (
              <span key={s} className="px-1 py-px text-xs uppercase tracking-wide" style={tint(statusAccent(s))}>
                {s}
              </span>
            ))}
          </span>
        ) : (
          ""
        )
      }
    >
      {id !== null && (
        <div className="flex flex-col gap-3 text-sm">
          {(contactAbilities.length > 0 || canManageConf) && (
            <div className="flex flex-col gap-2">
              {contactAbilities.length > 0 && (
                <Button variant="ghost" className="justify-start" onClick={contact}>
                  Contact
                </Button>
              )}
              {canManageConf && (
                <Button variant="ghost" className="justify-start" onClick={toggleConf}>
                  {inConf ? "Remove from press conference" : "Add to press conference"}
                </Button>
              )}
            </div>
          )}

          {isAdmin && (
            <div className="border-t border-edge pt-1">
              <PlayerAdminControls id={id} />
            </div>
          )}

          {!hasActions && <p className="py-1 text-ink-dim">No actions available.</p>}

          <FlashLine flash={flash} />
        </div>
      )}
    </Modal>
  );
}
