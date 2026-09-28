// The selected channel's organization, opened from a rail button: its abilities (via AbilityMenu),
// its roster, and — for System — membership controls. Renders nothing unless the selected channel
// backs an org this view was told about.
import { useState } from "react";
import type { Action } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { orgMembers, orgOfChannel } from "amane-client/queries/orgs.ts";
import { viewActor } from "amane-client/queries/session.ts";
import { orgDisplayName, playerLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Modal } from "../../kit/Modal.tsx";
import { orgColorVar, tint } from "../../style.ts";
import { useGameUi, useView } from "../game_ui.ts";
import { Entity } from "../Name.tsx";
import { AbilityMenu } from "../abilities/AbilityMenu.tsx";
import { RailButton } from "./RailButton.tsx";

export function OrgPanel() {
  const ui = useGameUi();
  const view = useView();
  const session = useSession();
  const flash = useFlash();
  const [open, setOpen] = useState(false);
  const [addLeader, setAddLeader] = useState(false);
  const [addOg, setAddOg] = useState(false);

  const channelId = ui.selected?.kind === "channel" ? ui.selected.id : null;
  const entry = orgOfChannel(view, channelId);
  if (!entry) return null;
  const [orgId, org] = entry;
  const accent = orgColorVar(org.name);
  const isAdmin = ui.viewer === "System";
  // The org's roster and abilities ride its own viewport. Frozen outranks membership below, since
  // membership reads the roster and the roster is one of the things that stopped updating — a view
  // that has lost the viewport can still be listed in an org it was thrown out of.
  const frozen = view.frozen(org.viewport);
  const isMember = isAdmin || org.members.has(ui.viewer);
  const members = orgMembers(view, orgId, org);
  const effectiveCount = members.filter((m) => m.effective).length;
  const candidates = [...view.players.keys()].filter((id) => !org.members.has(id));

  async function send(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: viewActor(ui.viewer), timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }
  function addMember(playerId: string) {
    send(
      { AddToOrg: { actor_id: slotKeyFromString(playerId), org_id: slotKeyFromString(orgId), leader: addLeader, og: addOg } },
      "Added.",
    );
  }
  function removeMember(playerId: string) {
    send({ RemoveFromOrg: { actor_id: slotKeyFromString(playerId), org_id: slotKeyFromString(orgId) } }, "Removed.");
  }
  function toggleOg(playerId: string, currentlyOg: boolean) {
    send({ SetOgStatus: { actor_id: slotKeyFromString(playerId), org_id: slotKeyFromString(orgId), og: !currentlyOg } }, "OG status updated.");
  }
  function toggleLeader(playerId: string, currentlyLeader: boolean) {
    send(
      { ChangeOrgLeader: { org_id: slotKeyFromString(orgId), new_leader: currentlyLeader ? null : slotKeyFromString(playerId) } },
      "Leader updated.",
    );
  }

  return (
    <>
      <RailButton active={open} onClick={() => setOpen(true)}>
        Organization
      </RailButton>

      <Modal
        open={open}
        onClose={() => setOpen(false)}
        width="32rem"
        title={
          <span className="flex items-center gap-2.5">
            <span className="h-3 w-3 shrink-0" style={{ backgroundColor: accent }} />
            <span style={{ color: accent }}>{orgDisplayName(org.name)}</span>
          </span>
        }
      >
        <div className="flex flex-col divide-y divide-edge">
          <div className="pb-4">
            {frozen ? (
              <p className="border-l-2 px-3 py-2 text-sm" style={tint("var(--color-event-alarm)")}>
                This organization no longer reaches you. Everything here is what you last saw.
              </p>
            ) : isMember ? (
              <AbilityMenu orgId={orgId} />
            ) : (
              <p className="border-l-2 border-edge bg-raised px-3 py-2 text-sm text-ink-dim">
                You are no longer in this organization. Everything here is what you last saw.
              </p>
            )}
          </div>

          <div className="flex flex-col gap-2 py-4">
            <div className="flex items-baseline justify-between gap-2">
              <span className="text-xs font-semibold uppercase tracking-widest text-ink-dim">Members</span>
              <span className="text-xs tabular-nums text-ink-dim">
                <span className="text-ink">{members.length}</span> total ·{" "}
                <span style={{ color: accent }}>{effectiveCount}</span> counted
              </span>
            </div>

            {members.length === 0 ? (
              <p className="py-1 text-sm text-ink-dim">No members.</p>
            ) : (
              <div className="flex flex-col border border-edge">
                {members.map((m) => (
                  <div key={m.id} className="flex flex-wrap items-center gap-2 border-b border-edge px-3 py-2 last:border-b-0">
                    <span className={`min-w-0 flex-1 truncate text-sm ${m.effective ? "text-ink" : "text-ink-dim"}`}>
                      {playerLabel(m.id, view.players)}
                    </span>
                    {!m.effective && (
                      <span
                        className="shrink-0 border border-edge px-1.5 py-0.5 text-xs font-medium uppercase tracking-wide text-ink-dim"
                        title="Not present, so not counted toward this org's ability member requirements. Still a full member."
                      >
                        not counted
                      </span>
                    )}
                    {m.leader && (
                      <span
                        className="shrink-0 border px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wide"
                        style={{ color: accent, borderColor: accent }}
                        title="The org's leader. Only they and the host are told who this is."
                      >
                        Leader
                      </span>
                    )}
                    {m.og && (
                      <span
                        className="shrink-0 border px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wide"
                        style={{ color: accent, borderColor: accent }}
                        title="An original member. Only they and the host know this."
                      >
                        OG
                      </span>
                    )}
                    {isAdmin && (
                      <span className="flex shrink-0 gap-1">
                        <Button variant="ghost" size="sm" title="Toggle leader" onClick={() => toggleLeader(m.id, m.leader)}>
                          leader
                        </Button>
                        <Button variant="ghost" size="sm" title="Toggle OG status" onClick={() => toggleOg(m.id, m.og)}>
                          og
                        </Button>
                        <Button variant="danger" size="sm" title="Remove from org" onClick={() => removeMember(m.id)}>
                          remove
                        </Button>
                      </span>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>

          {isAdmin && (
            <div className="flex flex-col gap-2 pt-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className="text-xs font-semibold uppercase tracking-widest text-ink-dim">Add member</span>
                <div className="flex gap-3 text-xs text-ink">
                  <label className="flex items-center gap-1.5">
                    <input type="checkbox" checked={addLeader} onChange={(e) => setAddLeader(e.target.checked)} /> leader
                  </label>
                  <label className="flex items-center gap-1.5">
                    <input type="checkbox" checked={addOg} onChange={(e) => setAddOg(e.target.checked)} /> og
                  </label>
                </div>
              </div>

              {candidates.length === 0 ? (
                <p className="py-1 text-sm text-ink-dim">No one to add.</p>
              ) : (
                <div className="flex flex-col border border-edge">
                  {candidates.map((id) => (
                    <button
                      key={id}
                      type="button"
                      className="flex min-h-8 w-full items-center justify-between border-b border-edge px-3 py-2 text-sm text-ink last:border-b-0 hover:bg-raised"
                      onClick={() => addMember(id)}
                    >
                      <Entity of={{ kind: "player", id }} view={view} menu={false} />
                      <span className="shrink-0 text-xs uppercase tracking-wide text-ink-dim">add</span>
                    </button>
                  ))}
                </div>
              )}
              <FlashLine flash={flash} />
            </div>
          )}
        </div>
      </Modal>
    </>
  );
}
