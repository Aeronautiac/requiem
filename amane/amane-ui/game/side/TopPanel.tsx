// The strip under the channel header showing live polls or prosecutions, one at a time — the button
// that opens a panel closes it. Capped in height and scrolls its own row, so a long list never grows
// the page.
import { livePolls } from "amane-client/queries/polls.ts";
import { useGameUi, useView } from "../game_ui.ts";
import { PollCard } from "./PollCard.tsx";
import { ProsecutionCard } from "./ProsecutionCard.tsx";

export function TopPanel() {
  const ui = useGameUi();
  const view = useView();
  if (!ui.top_panel) return null;
  const panel = ui.top_panel;

  const polls = panel === "polls" ? livePolls(view) : [];
  const prosecutions = panel === "prosecutions" ? [...view.prosecutions.entries()] : [];

  return (
    <div className="flex max-h-[40dvh] flex-col border-b border-edge bg-surface">
      <div className="flex shrink-0 items-center gap-2 px-3 py-1">
        <span className="text-xs font-medium uppercase tracking-wide text-ink-dim">
          {panel === "polls" ? "Polls" : "Prosecutions"}
        </span>
        <span className="text-xs text-ink-dim">{panel === "polls" ? polls.length : prosecutions.length}</span>
        <button
          type="button"
          className="ml-auto px-1.5 text-ink-dim hover:text-ink"
          aria-label="Close panel"
          onClick={() => ui.togglePanel(panel)}
        >
          ✕
        </button>
      </div>

      <div className="flex min-h-0 flex-1 gap-2 overflow-auto px-3 pb-2">
        {panel === "polls" ? (
          polls.length === 0 ? (
            <p className="py-2 text-sm text-ink-dim">No active votes.</p>
          ) : (
            polls.map((p) => <PollCard key={p.id} id={p.id} data={p.data} pollView={p.pollView} frozen={p.frozen} variant="panel" />)
          )
        ) : prosecutions.length === 0 ? (
          <p className="py-2 text-sm text-ink-dim">No active prosecutions.</p>
        ) : (
          prosecutions.map(([id, data]) => <ProsecutionCard key={id} id={id} data={data} />)
        )}
      </div>
    </div>
  );
}
