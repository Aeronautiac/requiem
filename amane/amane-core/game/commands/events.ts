// Normalize a raw wire Command into the feed event shape the channel renderer already displays.
//
// A channel log stores the same *observed* commands a live view is handed (a message, a Kira
// connection attempt, an announcement, a tap event), so a log record reads back exactly what was
// written down. Every command that has a single display maps here; commands with no feed display
// (registrations, rosters, viewport bookkeeping) return null and are omitted — they were never
// something a channel shows.
import { slotKeyToString } from "../../bindings";
import type { Command } from "../../bindings";
import type { GameEvent } from "../types";

export function commandToEvent(cmd: Command, timestamp: number): GameEvent | null {
  const ts = timestamp;

  if ("AddMessage" in cmd) {
    return {
      timestamp: ts,
      data: { Message: { sender_display: cmd.AddMessage.sender_display, content: cmd.AddMessage.content } },
    };
  }

  if ("KiraConnectionAttempt" in cmd) {
    return {
      timestamp: ts,
      data: {
        KiraConnectionAttempt: {
          user: slotKeyToString(cmd.KiraConnectionAttempt.user),
          success: cmd.KiraConnectionAttempt.success,
        },
      },
    };
  }

  if ("ChannelTapped" in cmd) {
    return { timestamp: ts, data: { ChannelTapped: {} } };
  }

  if ("AddContactLog" in cmd) {
    return { timestamp: ts, data: { ContactLogEntry: cmd.AddContactLog.log } };
  }

  if ("AnonymousAnnouncement" in cmd) {
    return { timestamp: ts, data: { AnonymousAnnouncement: { content: cmd.AnonymousAnnouncement.content } } };
  }

  if ("EyeDealTaken" in cmd) {
    return { timestamp: ts, data: { EyeDealTaken: { user: cmd.EyeDealTaken.user } } };
  }

  if ("NewsAnchor" in cmd) {
    return {
      timestamp: ts,
      data: { NewsAnchor: { target_id: cmd.NewsAnchor.target_id ? slotKeyToString(cmd.NewsAnchor.target_id) : null } },
    };
  }

  if ("PressConfStatus" in cmd) {
    return {
      timestamp: ts,
      data: {
        PressConfStatus: {
          target_id: slotKeyToString(cmd.PressConfStatus.target_id),
          has_access: cmd.PressConfStatus.has_access,
        },
      },
    };
  }

  if ("FailedSilentProsecution" in cmd) {
    return {
      timestamp: ts,
      data: {
        FailedSilentProsecution: {
          accuser_id: slotKeyToString(cmd.FailedSilentProsecution.accuser_id),
          true_name: cmd.FailedSilentProsecution.true_name,
          org: cmd.FailedSilentProsecution.org,
        },
      },
    };
  }

  if ("PseudocideRevival" in cmd) {
    return {
      timestamp: ts,
      data: { PseudocideRevival: { target_id: slotKeyToString(cmd.PseudocideRevival.target_id) } },
    };
  }

  if ("NewIteration" in cmd) {
    return { timestamp: ts, data: { NewIteration: { iteration: cmd.NewIteration.iteration } } };
  }

  if ("Blackout" in cmd) {
    return { timestamp: ts, data: { Blackout: { active: cmd.Blackout.active } } };
  }

  if ("Kidnapping" in cmd) {
    return {
      timestamp: ts,
      data: {
        Kidnapping: {
          kidnapping_id: slotKeyToString(cmd.Kidnapping.kidnapping_id),
          target_id: slotKeyToString(cmd.Kidnapping.target_id),
          duration: cmd.Kidnapping.duration,
        },
      },
    };
  }

  if ("KidnapReveal" in cmd) {
    return {
      timestamp: ts,
      data: {
        KidnapReveal: {
          kidnapping_id: slotKeyToString(cmd.KidnapReveal.kidnapping_id),
          victim: null,
          kidnapper: cmd.KidnapReveal.kidnapper ? slotKeyToString(cmd.KidnapReveal.kidnapper) : null,
        },
      },
    };
  }

  if ("Incarceration" in cmd) {
    return {
      timestamp: ts,
      data: {
        Incarceration: {
          incarceration_id: slotKeyToString(cmd.Incarceration.incarceration_id),
          victim_id: slotKeyToString(cmd.Incarceration.victim_id),
          duration: cmd.Incarceration.duration,
        },
      },
    };
  }

  if ("IncarcerationReleased" in cmd) {
    return {
      timestamp: ts,
      data: {
        IncarcerationReleased: {
          incarceration_id: slotKeyToString(cmd.IncarcerationReleased.incarceration_id),
          victim: null,
        },
      },
    };
  }

  if ("Death" in cmd) {
    const d = cmd.Death;
    return {
      timestamp: ts,
      data: {
        Death: {
          target_id: slotKeyToString(d.target_id),
          true_name: d.true_name,
          death_message: d.death_message,
          role: d.role,
          notebook_transferred: d.notebook_transferred,
          ability_transferred: d.ability_transferred,
        },
      },
    };
  }

  if ("NotebookWrite" in cmd) {
    const w = cmd.NotebookWrite;
    return {
      timestamp: ts,
      data: {
        Write: {
          user_id: slotKeyToString(w.user_id),
          notebook_id: slotKeyToString(w.notebook_id),
          message: w.message ?? "",
          true_name: w.true_name,
          delay: w.delay,
          successes_remaining: w.successes_remaining,
          attempts_remaining: w.attempts_remaining,
          success: w.success,
          target_saved: w.target_saved,
        },
      },
    };
  }

  if ("RoleUpdate" in cmd) {
    return { timestamp: ts, data: { RoleUpdate: { role: cmd.RoleUpdate.role } } };
  }

  if ("TrueNameUpdate" in cmd) {
    return { timestamp: ts, data: { TrueNameUpdate: { true_name: cmd.TrueNameUpdate.true_name } } };
  }

  if ("EyeCount" in cmd) {
    return { timestamp: ts, data: { EyeCount: { count: cmd.EyeCount.count } } };
  }

  if ("Bugged" in cmd) {
    return { timestamp: ts, data: { Bugged: { context: cmd.Bugged.context } } };
  }

  if ("RevealTrueName" in cmd) {
    return {
      timestamp: ts,
      data: {
        RevealTrueName: {
          target_id: slotKeyToString(cmd.RevealTrueName.target_id),
          true_name: cmd.RevealTrueName.true_name,
        },
      },
    };
  }

  if ("RevealNotebookHolding" in cmd) {
    return {
      timestamp: ts,
      data: {
        RevealNotebookHolding: {
          target_id: slotKeyToString(cmd.RevealNotebookHolding.target_id),
          holding: cmd.RevealNotebookHolding.holding,
        },
      },
    };
  }

  // Everything else — channel/actor/object registration, rosters, viewport bookkeeping — has no
  // single feed display and is not part of a written-down record.
  return null;
}
