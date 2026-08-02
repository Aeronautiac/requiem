// Every piece of user-facing copy the core produces. Nothing outside this file may contain a
// sentence: state code deals in values, and the value becomes words here.
//
// `{name}` placeholders are filled by `t()` in game/helpers.svelte.ts. Keys are typed, so a typo is a
// build error rather than a blank on screen.
export const STRINGS = {
  // ---- prosecution phase, short labels for the panel ----
  prosecution_label_awaiting_host: "Awaiting the host",
  prosecution_label_verdict_vote: "Verdict vote",
  prosecution_label_custody: "In custody",
  prosecution_label_debate: "Trial · debate",
  prosecution_label_to_begin: "Trial · {side} to begin",
  prosecution_label_speaking: "Trial · {side} speaking",
  prosecution_side_prosecution: "prosecution",
  prosecution_side_defense: "defense",

  // ---- prosecution phase, the announcing sentence (news feed and toast share it) ----
  prosecution_found_guilty: "{defendant} has been found guilty.",
  prosecution_acquitted: "{defendant} has been acquitted.",
  prosecution_ended: "The prosecution of {defendant} has ended.",
  prosecution_verdict_vote_begun: "The verdict vote for {defendant} has begun.",
  prosecution_started: "{prosecutor} is prosecuting {defendant}.",
  prosecution_entered_debate: "The trial of {defendant} has entered debate.",
  prosecution_trial_begun: "The trial of {defendant} has begun — the prosecution has the floor.",
  prosecution_presents: "In the trial of {defendant}, the prosecution presents.",
  prosecution_defense_floor: "In the trial of {defendant}, the defense has the floor.",
  prosecution_defense_presents: "In the trial of {defendant}, the defense presents.",

  // ---- how a refused or failed submission reads ----
  //
  // These exist because the state layer hands back a VALUE (Denied, Crashed, an ActionError) and
  // the render site turns it into this. Nothing in the pipeline holds the sentence.
  exec_denied: "You are not permitted to do that.",
  exec_crashed: "The engine has crashed.",
  exec_desync: "The client and server are out of step. Please reconnect.",
  exec_lost_state: "The client lost track of the game state and cannot continue. Please reconnect.",
  exec_left_game: "Left the game.",

  control_Denied: "You are not an administrator of this game.",
  control_KeyNotFound: "That key does not exist.",
  control_CannotActOnSelf: "You cannot change your own key.",
  control_RequiresSupervise: "Only a supervisor can change another administrator's key.",
  control_CannotGrantSupervise: "Only a supervisor can grant the supervisor capability.",

  // ---- OS toasts ----
  //
  // One fires for every world event and everything that lands in the personal log, so a player
  // away from the screen still learns what happened to them. The wording is a touch terser than the
  // feed's — a toast is a glance, the feed is the full record — but nothing is withheld for privacy.
  toast_death_title: "Death",
  toast_death_body: "{name} has died.",
  toast_announcement_title: "Anonymous Announcement",
  toast_kidnapping_title: "Kidnapping",
  toast_kidnapping_body: "{name} has been kidnapped.",
  toast_kidnap_reveal_title: "Kidnap Reveal",
  toast_kidnap_reveal_named: "Authorities have recovered {victim}, and {kidnapper} was revealed as the kidnapper.",
  toast_kidnap_reveal_anonymous: "Authorities have recovered {victim}, but the kidnapper stayed anonymous.",
  toast_kidnap_reveal_unknown_victim: "the victim",
  toast_incarceration_title: "Imprisonment",
  toast_incarceration_timed: "{name} has been imprisoned for {duration}.",
  toast_incarceration_body: "{name} has been imprisoned.",
  toast_release_title: "Release",
  toast_release_body: "{name} has been released.",
  toast_release_unknown: "A prisoner",
  toast_revival_title: "Revival",
  toast_revival_body: "{name} is alive.",
  toast_role_title: "Role",
  toast_role_body: "Your role is now {role}.",
  toast_true_name_title: "True Name",
  toast_true_name_body: "Your true name is now {name}.",
  toast_false_accusation_title: "False Accusation",
  toast_false_accusation_body: "{name} (real name: {true_name}) named an innocent person as wanted. {org} has expelled and barred them.",
  toast_prosecution_title: "Prosecution",
  toast_prosecution_ended_title: "Prosecution Ended",
  toast_game_begins_title: "The Game Begins",
  toast_game_begins_body: "Day 1. Abilities and notebooks are live.",
  toast_new_day_title: "New Day",
  toast_new_day_body: "Day {day}.",
  toast_reveal_name_title: "Name Reveal",
  toast_reveal_name_body: "{name}'s true name is {true_name}.",
  toast_reveal_notebook_title: "Notebook Check",
  toast_reveal_notebook_holding: "{name} is holding a notebook.",
  toast_reveal_notebook_empty: "{name} is not holding a notebook.",
  toast_bugged_title: "Surveillance",
  toast_bugged_explicit: "You have been bugged. Your messages are being monitored.",
  toast_bugged_custody: "You are being monitored while in custody.",
  toast_notebook_received_title: "Notebook",
  toast_notebook_received_body: "A notebook has come into your possession.",
  toast_tap_in_title: "Tap In",
  toast_tap_in_found: "Contact {id} tapped — reading its record.",
  toast_tap_in_no_contact: "Contact {id} does not exist.",
  toast_tap_in_not_loggable: "Contact {id} has logging off — nothing was written down.",

  // ---- blackout, which is the only world event that announces its own silence ----
  blackout_begun_label: "Blackout",
  blackout_begun:
    "The world has gone dark. Nothing that happens will be announced until it lifts, and the news is off the air.",
  blackout_over_label: "Blackout Over",
  blackout_over: "The lights are back. Everything that happened in the dark follows.",

  // ---- organizations, whose config codes are terse on purpose ----
  org_name_NULL: "Null",
  org_name_KK: "Kira's Kingdom",
  org_name_TF: "Task Force",
  org_name_SPK: "SPK",

  // ---- unnamed objects, which the engine never names for us ----
  player_unnamed: "player-{idx}v{version}",
  display_mysterious: "???",
  display_system: "System",
  display_org_unknown: "Org",
  display_unknown: "Unknown",
} as const;

export type StringKey = keyof typeof STRINGS;
