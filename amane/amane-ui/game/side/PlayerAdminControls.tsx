// Everything here dispatches as the Admin actor, which the engine accepts for these.
import { useState } from "react";
import type { Action, Role } from "amane-client/bindings.ts";
import { ROLES, slotKeyFromString } from "amane-client/bindings.ts";
import { nameLabel, roleLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Input, Select } from "../../kit/Input.tsx";
import { useView } from "../game_ui.ts";

export function PlayerAdminControls({ id }: { id: string }) {
  const view = useView();
  const session = useSession();
  const flash = useFlash();
  const target = slotKeyFromString(id);
  // Only System is ever told these, and this panel only renders for System.
  const info = view.players.get(id);

  // For picking a NEW role; the current one is shown in the inspector above.
  const [role, setRole] = useState<Role>("Civilian");
  const [trueName, setTrueName] = useState("");
  const [displayName, setDisplayName] = useState("");

  // Sub-menu state for the kill / revive actions. Each option below is a real field of the engine
  // action, surfaced as a control rather than hardcoded in the dispatch. A blank message sends null,
  // meaning "use the engine default".
  const [killOpen, setKillOpen] = useState(false);
  const [killMessage, setKillMessage] = useState("");
  const [killSilent, setKillSilent] = useState(false);
  const [killAllowChaining, setKillAllowChaining] = useState(true);
  const [killSeverLinks, setKillSeverLinks] = useState(true);
  const [killBooksDormant, setKillBooksDormant] = useState(false);

  const [reviveOpen, setReviveOpen] = useState(false);
  const [reviveMessage, setReviveMessage] = useState("");
  const [reviveSilent, setReviveSilent] = useState(false);
  const [reviveIgnoreLinks, setReviveIgnoreLinks] = useState(false);

  async function run(payload: Action, ok: string) {
    const reply = await session.submit_action({ actor: "Admin", timestamp: Date.now(), payload });
    flash.reply(reply, ok);
  }

  function setRoleAction() {
    run({ GiveRole: { target_id: target, role } }, `Role set to ${roleLabel(role)}.`);
  }
  function setTrueNameAction() {
    const name = trueName.trim();
    if (!name) {
      flash.error("Enter a name.");
      return;
    }
    run({ SetTrueName: { target_id: target, true_name: name } }, "True name set.");
  }
  // A CONTROL, not an action: a profile is the server's record of who is playing the slot, and the
  // engine has no concept of it. True name above is the opposite — a mechanic, secret, and the thing
  // you write in a notebook.
  async function setProfileAction() {
    const name = displayName.trim();
    if (!name) {
      flash.error("Enter a name.");
      return;
    }
    const reply = await session.submit_control({
      Sim: { time: 0, data: { SetProfile: { actor: target, profile: { display_name: name } } } },
    });
    flash.reply(reply, "Profile updated.");
  }
  // News anchor is a status, not a role — handing over the anchor's kit rather than changing who the
  // player is. null vacates the post entirely rather than handing it on.
  function makeNewsAnchor() {
    run({ SetNewsAnchor: { target_id: target } }, "Set as news anchor.");
  }
  function clearNewsAnchor() {
    run({ SetNewsAnchor: { target_id: null } }, "News anchor post vacated.");
  }
  function kill() {
    run(
      {
        Kill: {
          target_id: target,
          killer_id: null,
          death_message: killMessage.trim() || null,
          silent: killSilent,
          allow_link_chaining: killAllowChaining,
          sever_links: killSeverLinks,
          set_books_dormant: killBooksDormant,
        },
      },
      "Player killed.",
    );
  }
  function revive() {
    run(
      {
        Revive: {
          target_id: target,
          ignore_links: reviveIgnoreLinks,
          silent: reviveSilent,
          revival_message: reviveMessage.trim() || null,
        },
      },
      "Player revived.",
    );
  }

  return (
    <div className="flex flex-col gap-2 py-1 text-sm">
      <div className="text-ink-dim">
        <div>Role: {info?.role ? roleLabel(info.role) : "—"}</div>
        <div>True name: {info?.true_name ? nameLabel(info.true_name) : "—"}</div>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <Select
          value={role}
          onChange={(e) => setRole(e.target.value as Role)}
          options={ROLES.map((r) => ({ value: r, label: roleLabel(r) }))}
          className="min-w-0 flex-1"
        />
        <Button variant="ghost" size="sm" onClick={setRoleAction}>
          Set role
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <Button variant="ghost" size="sm" className="flex-1" onClick={makeNewsAnchor}>
          Set as news anchor
        </Button>
        <Button variant="ghost" size="sm" onClick={clearNewsAnchor}>
          Vacate
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <Input value={trueName} onChange={(e) => setTrueName(e.target.value)} placeholder="New true name" className="min-w-0 flex-1" />
        <Button variant="ghost" size="sm" onClick={setTrueNameAction}>
          Set true name
        </Button>
      </div>

      <div className="flex flex-wrap items-center gap-1.5">
        <Input
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          placeholder="New display name"
          className="min-w-0 flex-1"
        />
        <Button variant="ghost" size="sm" onClick={setProfileAction}>
          Set display name
        </Button>
      </div>

      <div className="flex flex-col gap-1.5 border-t border-edge pt-1.5">
        <div className="flex flex-col gap-1">
          <Button variant="danger" size="sm" className="justify-start" onClick={() => setKillOpen(!killOpen)}>
            <span className="inline-block w-3 text-center text-xs">{killOpen ? "▾" : "▸"}</span>
            Kill
          </Button>
          {killOpen && (
            <div className="flex flex-col gap-1.5 pl-3">
              <Input value={killMessage} onChange={(e) => setKillMessage(e.target.value)} placeholder="Death message (blank = default)" />
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={killSilent} onChange={(e) => setKillSilent(e.target.checked)} /> Silent
              </label>
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={killAllowChaining} onChange={(e) => setKillAllowChaining(e.target.checked)} /> Allow
                link chaining
              </label>
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={killSeverLinks} onChange={(e) => setKillSeverLinks(e.target.checked)} /> Sever links
              </label>
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={killBooksDormant} onChange={(e) => setKillBooksDormant(e.target.checked)} /> Set books
                dormant
              </label>
              <Button variant="danger" size="sm" onClick={kill}>
                Kill
              </Button>
            </div>
          )}
        </div>

        <div className="flex flex-col gap-1">
          <Button variant="ghost" size="sm" className="justify-start text-ok-text" onClick={() => setReviveOpen(!reviveOpen)}>
            <span className="inline-block w-3 text-center text-xs">{reviveOpen ? "▾" : "▸"}</span>
            Revive
          </Button>
          {reviveOpen && (
            <div className="flex flex-col gap-1.5 pl-3">
              <Input
                value={reviveMessage}
                onChange={(e) => setReviveMessage(e.target.value)}
                placeholder="Revival message (blank = default)"
              />
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={reviveSilent} onChange={(e) => setReviveSilent(e.target.checked)} /> Silent
              </label>
              <label className="flex items-center gap-2 text-ink">
                <input type="checkbox" checked={reviveIgnoreLinks} onChange={(e) => setReviveIgnoreLinks(e.target.checked)} /> Ignore
                links
              </label>
              <Button variant="ghost" size="sm" className="text-ok-text" onClick={revive}>
                Revive
              </Button>
            </div>
          )}
        </div>
      </div>

      <FlashLine flash={flash} />
    </div>
  );
}
