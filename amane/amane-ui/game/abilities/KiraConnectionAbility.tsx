import { useState } from "react";
import { slotKeyFromString } from "amane-client/bindings.ts";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { channelLabel } from "amane-client/text.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { Select } from "../../kit/Input.tsx";
import { useGameUi, useView } from "../game_ui.ts";
import type { AbilityUiProps } from "./registry.ts";

export function KiraConnectionAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const view = useView();

  const [lounge, setLounge] = useState("");
  const flash = useFlash();

  // Every lounge the viewer is in, by the channel it shows up as. Only a Basic lounge actually
  // qualifies, but the client is not told which variant a lounge is — an anonymous line looks like
  // any other from here — so all of them are offered and the engine rejects the rest.
  const lounges: { loungeId: string; name: string }[] = [];
  for (const channel of view.channels.values()) {
    if (channel.category !== "Lounge") continue;
    const link = channel.link;
    if (link && "lounge" in link) lounges.push({ loungeId: link.lounge, name: channel.name });
  }

  async function connect() {
    if (!lounge) {
      flash.error("Pick a line to reach through.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { KiraConnection: { lounge: slotKeyFromString(lounge) } }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <Select
        value={lounge}
        onChange={(e) => setLounge(e.target.value)}
        options={[{ value: "", label: "Select a line" }, ...lounges.map((l) => ({ value: l.loungeId, label: channelLabel(l.name) }))]}
      />
      <Button variant="danger" onClick={connect}>
        Reach for Kira
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
