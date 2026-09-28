// The rail's action row: the org panel entry, plus the two toggles that raise the widget strip
// under the channel header.
import { livePolls } from "amane-client/queries/polls.ts";
import type { TopPanel } from "../game_ui.ts";
import { useGameUi, useView } from "../game_ui.ts";
import { OrgPanel } from "./OrgPanel.tsx";
import { RailButton } from "./RailButton.tsx";

const TOGGLES: { panel: TopPanel; label: string }[] = [
  { panel: "polls", label: "Polls" },
  { panel: "prosecutions", label: "Prosecutions" },
];

export function RightControls() {
  const ui = useGameUi();
  const view = useView();
  const counts: Record<TopPanel, number> = {
    polls: livePolls(view).length,
    prosecutions: view.prosecutions.size,
  };

  return (
    <div className="flex flex-col gap-1 border-b border-edge p-2">
      <OrgPanel />

      {TOGGLES.map((t) => (
        <RailButton
          key={t.panel}
          active={ui.top_panel === t.panel}
          count={counts[t.panel]}
          onClick={() => ui.togglePanel(t.panel)}
        >
          {t.label}
        </RailButton>
      ))}
    </div>
  );
}
