import { useState } from "react";
import { abilityRequest } from "amane-client/queries/abilities.ts";
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { useGameUi } from "../game_ui.ts";
import { MentionInput } from "../mentions/MentionInput.tsx";
import type { AbilityUiProps } from "./registry.ts";

export function AnonymousAnnouncementAbility({ abilityId, orgId, onDone }: AbilityUiProps) {
  const session = useSession();
  const ui = useGameUi();
  const [content, setContent] = useState("");
  const flash = useFlash();

  async function announce() {
    if (!content.trim()) {
      flash.error("Write something to announce.");
      return;
    }
    const reply = await session.submit_action(
      abilityRequest(ui.viewer, abilityId, orgId, { AnonymousAnnouncement: { content } }, Date.now()),
    );
    if (flash.reply(reply)) onDone();
  }

  return (
    <div className="flex flex-col gap-3">
      <MentionInput value={content} onChange={setContent} placeholder="Announcement…" onSubmit={announce} />
      <Button onClick={announce}>Announce</Button>
      <FlashLine flash={flash} />
    </div>
  );
}
