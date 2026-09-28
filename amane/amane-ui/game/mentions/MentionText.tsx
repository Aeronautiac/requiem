// Renders message text with its embedded mentions turned into coloured chips. The `@` is dropped;
// an id becomes a real name, resolved against the view being rendered. Every mention kind — player
// included — renders through the shared Entity component, so a name picks up the same colour and
// chip look it has everywhere else.
import type { View } from "amane-client/game/view.ts";
import { parseMentions } from "amane-client/text.ts";
import { Entity } from "../Name.tsx";

export function MentionText({ content, view }: { content: string; view: View }) {
  const segments = parseMentions(content);
  return (
    <>
      {segments.map((seg, i) => {
        if ("text" in seg) return seg.text;
        return <Entity key={i} of={seg.mention} view={view} chip />;
      })}
    </>
  );
}
