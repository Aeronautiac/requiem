// The host's tools, as a row of buttons beside the rest of the bottom bar. Mounted only when this
// key administers the game (GameScreen gates on `administers(session)`).
import { useSession } from "../../hooks.ts";
import { Button } from "../../kit/Button.tsx";
import { FlashLine, useFlash } from "../../kit/Flash.tsx";
import { AddPlayers } from "./AddPlayers.tsx";
import { GoToTime } from "./GoToTime.tsx";
import { KeyManager } from "./KeyManager.tsx";
import { TimelineViewer } from "./TimelineViewer.tsx";

export function AdminPanel() {
  const session = useSession();
  const flash = useFlash();

  async function startGame() {
    // Rejected once the game is running, so this is safe to leave in place rather than gate on a
    // phase the client is not told.
    const reply = await session.submit_action({ actor: "Admin", timestamp: Date.now(), payload: { StartGame: {} } });
    flash.reply(reply, "Day 1. Abilities and notebooks are live.");
  }

  async function nextDay() {
    const reply = await session.submit_action({ actor: "Admin", timestamp: Date.now(), payload: { NextIteration: {} } });
    flash.reply(reply, "Day progressed.");
  }

  async function crash() {
    // A crash comes back as the "engine has crashed" string; the runtime is respawned and
    // resaturated behind the scenes, so surface it to make the crash visible at all.
    const reply = await session.submit_action({ actor: "Admin", timestamp: Date.now(), payload: { Crash: {} } });
    flash.reply(reply, "No crash?");
  }

  return (
    <div className="flex flex-wrap items-center gap-2">
      <AddPlayers />
      <KeyManager />
      <GoToTime flash={flash} />
      <TimelineViewer />
      <Button variant="ghost" size="sm" onClick={startGame}>
        Start Game
      </Button>
      <Button variant="ghost" size="sm" onClick={nextDay}>
        Next Day
      </Button>
      <Button variant="danger" size="sm" onClick={crash}>
        Crash
      </Button>
      <FlashLine flash={flash} />
    </div>
  );
}
