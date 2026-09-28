// Which commands are worth an OS notification, and what it says. The core only reports each
// delivery; this is where it is judged. A command with no rule raises nothing, and making a silent
// one notify is adding a rule here, with no core change.
//
// A rule's `toast` runs after the command has folded into the view, so it reads the new state. A
// rule that needs the state from BEFORE (a transition: "you just gained read access") takes it in
// `before`, which must copy values out, never hold references: the object it reads will hold the
// new state by the time `toast` runs.
import type { ProsecutionPhaseView } from "amane-client/bindings.ts";
import { slotKeyToString } from "amane-client/bindings.ts";
import type { Delivered, Observer } from "amane-client/game/game.ts";
import type { CommandName, PayloadOf } from "amane-client/game/handlers/index.ts";
import { phaseViewEqual } from "amane-client/game/prosecution.ts";
import type { View } from "amane-client/game/view.ts";
import { NOTIF_CHANNEL } from "amane-client/game/view.ts";
import type { Toast } from "amane-client/host.ts";
import {
  formatDuration,
  mentionsViewer,
  nameLabel,
  orgDisplayName,
  phaseAnnouncementKey,
  playerLabel,
  refFromDisplay,
  refLabel,
  roleLabel,
  t,
} from "amane-client/text.ts";

type Rule<K extends CommandName> = {
  before?: (view: View, payload: PayloadOf<K>) => unknown;
  toast: (view: View, payload: PayloadOf<K>, memo: unknown, delivered: Delivered) => Toast | null;
};

type Rules = { [K in CommandName]?: Rule<K> };

const RULES: Rules = {
  Death: {
    toast: (view, p) => ({
      title: t("toast_death_title"),
      body: t("toast_death_body", { name: playerLabel(slotKeyToString(p.target_id), view.players) }),
    }),
  },

  // "You lost the anchor" needs who held it before this command.
  NewsAnchor: {
    before: (view) => view.news_anchor,
    toast: (view, p, previous) => {
      const target = p.target_id ? slotKeyToString(p.target_id) : null;
      const title = t("toast_news_anchor_title");
      if (target === view.own_key) return { title, body: t("toast_news_anchor_you_gained") };
      if (previous === view.own_key) return { title, body: t("toast_news_anchor_you_lost") };
      return {
        title,
        body: target
          ? t("toast_news_anchor_named", { name: playerLabel(target, view.players) })
          : t("toast_news_anchor_vacated"),
      };
    },
  },

  PressConfStatus: {
    toast: (view, p) => {
      const name = playerLabel(slotKeyToString(p.target_id), view.players);
      return {
        title: t("toast_press_conf_title"),
        body: p.has_access ? t("toast_press_conf_joined", { name }) : t("toast_press_conf_left", { name }),
      };
    },
  },

  AnonymousAnnouncement: {
    toast: (_view, p) => ({ title: t("toast_announcement_title"), body: p.content }),
  },

  EyeDealTaken: {
    toast: (view, p) => ({
      title: t("toast_eye_deal_title"),
      body: t("toast_eye_deal_body", { who: refLabel(refFromDisplay(p.user, view.orgs), view.players) }),
    }),
  },

  FailedSilentProsecution: {
    toast: (view, p) => ({
      title: t("toast_false_accusation_title"),
      body: t("toast_false_accusation_body", {
        name: playerLabel(slotKeyToString(p.accuser_id), view.players),
        true_name: nameLabel(p.true_name),
        org: orgDisplayName(p.org),
      }),
    }),
  },

  Revival: {
    toast: (view, p) => ({
      title: t("toast_revival_title"),
      body: t("toast_revival_body", { name: playerLabel(slotKeyToString(p.target_id), view.players) }),
    }),
  },

  NewIteration: {
    toast: (_view, p) =>
      p.iteration === 1
        ? { title: t("toast_game_begins_title"), body: t("toast_game_begins_body") }
        : { title: t("toast_new_day_title"), body: t("toast_new_day_body", { day: p.iteration }) },
  },

  Blackout: {
    toast: (_view, p) =>
      p.active
        ? { title: t("blackout_begun_label"), body: t("blackout_begun") }
        : { title: t("blackout_over_label"), body: t("blackout_over") },
  },

  Kidnapping: {
    toast: (view, p) => ({
      title: t("toast_kidnapping_title"),
      body: t("toast_kidnapping_body", { name: playerLabel(slotKeyToString(p.target_id), view.players) }),
    }),
  },

  KidnapReveal: {
    toast: (view, p) => {
      const tracked = view.kidnappings.get(slotKeyToString(p.kidnapping_id));
      const victim = tracked ? playerLabel(tracked.victim, view.players) : t("toast_kidnap_reveal_unknown_victim");
      return {
        title: t("toast_kidnap_reveal_title"),
        body: p.kidnapper
          ? t("toast_kidnap_reveal_named", {
              victim,
              kidnapper: playerLabel(slotKeyToString(p.kidnapper), view.players),
            })
          : t("toast_kidnap_reveal_anonymous", { victim }),
      };
    },
  },

  Incarceration: {
    toast: (view, p) => {
      const name = playerLabel(slotKeyToString(p.victim_id), view.players);
      return {
        title: t("toast_incarceration_title"),
        body: p.duration
          ? t("toast_incarceration_timed", { name, duration: formatDuration(p.duration) })
          : t("toast_incarceration_body", { name }),
      };
    },
  },

  IncarcerationReleased: {
    toast: (view, p) => {
      const tracked = view.incarcerations.get(slotKeyToString(p.incarceration_id));
      return {
        title: t("toast_release_title"),
        body: t("toast_release_body", {
          name: tracked ? playerLabel(tracked.victim, view.players) : t("toast_release_unknown"),
        }),
      };
    },
  },

  // The System copy feeds the admin inspector and raises nothing.
  RoleUpdate: {
    toast: (view, p) =>
      view.own_key === "System"
        ? null
        : { title: t("toast_role_title"), body: t("toast_role_body", { role: roleLabel(p.role) }) },
  },

  TrueNameUpdate: {
    toast: (view, p) =>
      view.own_key === "System"
        ? null
        : { title: t("toast_true_name_title"), body: t("toast_true_name_body", { name: nameLabel(p.true_name) }) },
  },

  // An org's copy lands in the org channel, not in anyone's personal feed, and raises nothing.
  RevealTrueName: {
    toast: (view, p, _memo, delivered) =>
      view.org_at(delivered.viewport)
        ? null
        : {
            title: t("toast_reveal_name_title"),
            body: t("toast_reveal_name_body", {
              name: playerLabel(slotKeyToString(p.target_id), view.players),
              true_name: nameLabel(p.true_name),
            }),
          },
  },

  RevealNotebookHolding: {
    toast: (view, p) => {
      const name = playerLabel(slotKeyToString(p.target_id), view.players);
      return {
        title: t("toast_reveal_notebook_title"),
        body: p.holding ? t("toast_reveal_notebook_holding", { name }) : t("toast_reveal_notebook_empty", { name }),
      };
    },
  },

  EyeCount: {
    toast: (_view, p) => ({ title: t("toast_eyes_title"), body: t("toast_eyes_body", { count: p.count }) }),
  },

  FakeLoungeTapped: {
    toast: (view, p) => {
      const who = refLabel(refFromDisplay(p.display, view.orgs), view.players);
      return view.own_key === "System"
        ? { title: t("toast_fake_lounge_admin_title"), body: t("toast_fake_lounge_admin_body", { who }) }
        : { title: t("toast_fake_lounge_title"), body: t("toast_fake_lounge_body", { who }) };
    },
  },

  Bugged: {
    toast: (_view, p) => ({
      title: t("toast_bugged_title"),
      body: p.context === "Custody" ? t("toast_bugged_custody") : t("toast_bugged_explicit"),
    }),
  },

  TapInResult: {
    toast: (view, p, _memo, delivered) => {
      if (view.org_at(delivered.viewport)) return null;
      const body =
        p.outcome === "NoSuchContact"
          ? t("toast_tap_in_no_contact", { id: p.contact_id })
          : p.outcome === "NotLoggable"
            ? t("toast_tap_in_not_loggable", { id: p.contact_id })
            : t("toast_tap_in_found", { id: p.contact_id });
      return { title: t("toast_tap_in_title"), body };
    },
  },

  LeaderStatus: {
    toast: (view, p) => {
      const org = view.orgs.get(slotKeyToString(p.org_id));
      const name = refLabel(org ? { kind: "org", org: org.name } : { kind: "unknown" }, view.players);
      return {
        title: t("toast_leader_title"),
        body: p.leader ? t("toast_leader_gained", { org: name }) : t("toast_leader_lost", { org: name }),
      };
    },
  },

  // The core decides when a notebook has arrived and writes NotebookReceived to the notifications
  // feed; this only raises what that delivery wrote.
  ProfileAccess: {
    before: (view) => view.channels.get(NOTIF_CHANNEL)?.events.length ?? 0,
    toast: (view, _p, count) => {
      const pushed = view.channels.get(NOTIF_CHANNEL)?.events.slice(count as number) ?? [];
      if (!pushed.some((e) => "NotebookReceived" in e.data)) return null;
      return { title: t("toast_notebook_received_title"), body: t("toast_notebook_received_body") };
    },
  },

  // A message that names this viewer: by key, role, or an org they are in.
  AddMessage: {
    toast: (view, p) => {
      if (!mentionsViewer(view, p.content)) return null;
      const channel = view.channels.get(slotKeyToString(p.channel_id));
      return {
        title: t("toast_mention_title"),
        body: t("toast_mention_body", {
          sender: refLabel(refFromDisplay(p.sender_display, view.orgs), view.players),
          channel: channel?.name ?? "",
        }),
      };
    },
  },

  // News only when the PHASE moves; a signal of readiness changes the snapshot without moving it.
  UpdateProsecution: {
    before: (view, p) => {
      const prev = view.prosecutions.get(slotKeyToString(p.prosecution_id));
      return prev ? structuredClone(prev.phase) : null;
    },
    toast: (view, p, prev_phase) => {
      if (prev_phase && phaseViewEqual(prev_phase as ProsecutionPhaseView, p.phase)) return null;
      return {
        title: t("toast_prosecution_title"),
        body: t(phaseAnnouncementKey(p.phase, false), {
          prosecutor: refLabel(refFromDisplay(p.prosecutor_display, view.orgs), view.players),
          defendant: refLabel(refFromDisplay(p.defendant_display, view.orgs), view.players),
        }),
      };
    },
  },

  // The prosecution is gone from the view after the fold, so the parties are read before it.
  CloseProsecution: {
    before: (view, p) => {
      const prev = view.prosecutions.get(slotKeyToString(p.prosecution_id));
      if (!prev) return null;
      return {
        phase: structuredClone(prev.phase),
        prosecutor: refLabel(refFromDisplay(prev.prosecutor_display, view.orgs), view.players),
        defendant: refLabel(refFromDisplay(prev.defendant_display, view.orgs), view.players),
      };
    },
    toast: (_view, p, memo) => {
      if (!memo) return null;
      const prev = memo as { phase: ProsecutionPhaseView; prosecutor: string; defendant: string };
      return {
        title: t("toast_prosecution_ended_title"),
        body: t(phaseAnnouncementKey(prev.phase, true, p.verdict), { prosecutor: prev.prosecutor, defendant: prev.defendant }),
      };
    },
  },
};

// The observer the game screen registers. `wanted` decides whether this delivery may raise
// anything at all (live, not muted, the view on screen); `raise` hands the toast to the host.
export function notificationObserver(
  wanted: (delivered: Delivered) => boolean,
  raise: (toast: Toast) => void,
): Observer {
  // The table is typed per command; looking a rule up by a runtime name needs this one widening.
  const rule = (name: CommandName) =>
    RULES[name] as Rule<CommandName> | undefined;
  const payload = (delivered: Delivered) =>
    (delivered.command as Record<string, unknown>)[delivered.name] as PayloadOf<CommandName>;

  return {
    before(view, delivered) {
      const found = rule(delivered.name);
      if (!found?.before || !wanted(delivered)) return undefined;
      return found.before(view, payload(delivered));
    },
    after(view, delivered, memo) {
      const found = rule(delivered.name);
      if (!found || !wanted(delivered)) return;
      const toast = found.toast(view, payload(delivered), memo, delivered);
      if (toast) raise(toast);
    },
  };
}
