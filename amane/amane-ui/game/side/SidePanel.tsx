// The right rail's content: the org/poll/prosecution controls, the group chat owner's controls (if
// any), and who's around. Placed inside an already-scrolling container (rail or drawer), so this
// sets no overflow or height of its own.
import { ErrorBoundary } from "../../kit/ErrorBoundary.tsx";
import { GcControls } from "./GcControls.tsx";
import { Players } from "./Players.tsx";
import { RightControls } from "./RightControls.tsx";

export function SidePanel() {
  return (
    <div className="flex flex-col">
      <ErrorBoundary name="Right controls">
        <RightControls />
      </ErrorBoundary>
      <ErrorBoundary name="Group chat controls">
        <GcControls />
      </ErrorBoundary>
      <ErrorBoundary name="Players">
        <Players />
      </ErrorBoundary>
    </div>
  );
}
