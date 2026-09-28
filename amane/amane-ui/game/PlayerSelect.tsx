// A player picker for anything that takes a target. Offers only the players this view has been
// told about: a target you have never heard of is not one you get to name. The engine is the
// authority on which targets are valid.
import { playerLabel } from "amane-client/text.ts";
import { useView } from "./game_ui.ts";

export function PlayerSelect({
  value,
  onChange,
  placeholder = "Select a player",
  ids,
}: {
  value: string;
  onChange: (id: string) => void;
  placeholder?: string;
  // Only these players (e.g. an org's members). Omitted: every player the view knows.
  ids?: Iterable<string>;
}) {
  const view = useView();
  const allowed = ids ? new Set(ids) : null;
  const players = [...view.players.keys()].filter((id) => !allowed || allowed.has(id));
  return (
    <select
      value={value}
      onChange={(event) => onChange(event.target.value)}
      className="h-9 w-full min-w-0 border border-edge bg-panel px-2 text-sm text-ink"
    >
      <option value="" disabled>
        {placeholder}
      </option>
      {players.map((id) => (
        <option key={id} value={id}>
          {playerLabel(id, view.players)}
        </option>
      ))}
    </select>
  );
}
