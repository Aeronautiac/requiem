// One component for both PublicKidnap and AnonymousKidnap — they differ only in the behaviour key,
// so the variant is resolved from the ability's own name.
//
// Public kidnap has one wrinkle: an ORG designates which of its own is shown as the kidnapper,
// while a player is always themselves, and the engine forbids a player from setting a performer.
// So the picker only appears for a public org ability.
import { useState } from "react";
import type { AbilityBehaviour } from "amane-client/bindings.ts";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import { PlayerSelect } from "../PlayerSelect.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function KidnapAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();

  // The set this instance lives in — an org's shared set, or the viewer's own.
  const source = orgId ? view.orgs.get(orgId)?.abilities : view.abilities;
  const isPublic = source?.get(abilityId)?.name === "PublicKidnap";
  // Only a public org kidnap chooses its public face; a player is always themselves.
  const choosesPerformer = isPublic && orgId != null;

  const [target, setTarget] = useState("");
  const [performer, setPerformer] = useState(""); // only used when choosesPerformer
  const flash = useFlash();

  async function run() {
    if (!target) {
      flash.error("Pick a target.");
      return;
    }
    const t = slotKeyFromString(target);
    // A player's performer must be null; an org sends the chosen face, or null to default to the
    // acting member.
    const behaviour: AbilityBehaviour = isPublic
      ? { PublicKidnap: { target: t, performer: choosesPerformer && performer ? slotKeyFromString(performer) : null } }
      : { AnonymousKidnap: { target: t } };
    const reply = await session.submit_action(abilityRequest(ui.viewer, abilityId, orgId, behaviour, Date.now()));
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <label className="text-xs text-ink-dim">Target</label>
      <PlayerSelect value={target} onChange={setTarget} placeholder="Who to kidnap" />

      {choosesPerformer && (
        <>
          <label className="text-xs text-ink-dim">Shown as the kidnapper (defaults to whoever acts)</label>
          {/* The engine also requires they be present. */}
          <PlayerSelect
            value={performer}
            onChange={setPerformer}
            placeholder="Public face (optional)"
            ids={view.orgs.get(orgId ?? "")?.members ?? []}
          />
        </>
      )}

      <Button onClick={run}>Kidnap</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
