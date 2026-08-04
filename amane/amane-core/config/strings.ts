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
  toast_news_anchor_title: "News Anchor",
  toast_news_anchor_you_gained: "You are now the news anchor.",
  toast_news_anchor_you_lost: "You are no longer the news anchor.",
  toast_news_anchor_named: "{name} is now the news anchor.",
  toast_news_anchor_vacated: "The news anchor post is now vacant.",
  news_anchor_label: "News Anchor",
  press_conference_label: "Press Conference",
  toast_press_conf_title: "Press Conference",
  toast_press_conf_joined: "{name} joined the press conference.",
  toast_press_conf_left: "{name} left the press conference.",
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
  toast_mention_title: "Mention",
  toast_mention_body: "{sender} mentioned you in {channel}.",
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

  // ---- roles: the engine enum on the left, what a player reads on the right ----
  role_name_Kira: "Kira",
  role_name_SecondKira: "2nd Kira",
  role_name_L: "L",
  role_name_Watari: "Watari",
  role_name_BeyondBirthday: "Beyond Birthday",
  role_name_PrivateInvestigator: "Private Investigator",
  role_name_Civilian: "Civilian",
  role_name_RogueCivilian: "Rogue Civilian",
  role_name_Poser: "Poser",
  role_name_ConArtist: "Con Artist",
  role_name_WantedCivilian: "Wanted Civilian",
  role_name_Near: "Near",
  role_name_Mello: "Mello",

  // ---- organizations, whose config codes are terse on purpose ----
  org_name_NULL: "Null",
  org_name_KK: "Kira's Kingdom",
  org_name_TF: "Task Force",
  org_name_SPK: "SPK",

  // ---- fixed channels the engine names with a code. Dynamic channels (lounges, group chats,
  // notebooks) carry their own names and simply fall through to what they were given. ----
  channel_name_LAndWatari: "L & Watari",

  // ---- unnamed objects, which the engine never names for us ----
  player_unnamed: "player-{idx}v{version}",
  display_mysterious: "???",
  display_system: "System",
  display_org_unknown: "Org",
  display_unknown: "Unknown",

  // ---- ability descriptions ----
  //
  // The single source for what each ability does. The card's description toggle and the drill-in
  // form both read these, so the sentence lives in exactly one place. Keyed by the raw AbilityName;
  // per-side warnings and one-off usage notes stay in the ability's own form, since they are not
  // this description.
  ability_desc_Gun: "Shoot a player of your choosing.",
  ability_desc_AnonymousContact:
    "Open an anonymous lounge with a player — they won't see who you are.",
  ability_desc_AnonymousAnnouncement: "Broadcast an anonymous announcement to the news feed.",
  ability_desc_Pseudocide:
    "Fake a target's death (yourself included). They're revived after a delay; the fields you set are announced as the death reveal.",
  ability_desc_FabricateLounge:
    "Fabricate a private lounge between two players. Only you can see it, and you hold both of their identities — letting you author a conversation that never happened.",
  ability_desc_FalseAnonymousContact:
    "Open an anonymous lounge with a player — they won't see who you are, and the role you show them is one you choose to pose as.",
  ability_desc_Ipp: "Grant IPP to a player — strengthened presence and immunity from being written.",
  ability_desc_Prosecute:
    "Openly prosecute a player — they'll be put into custody and the trial is filed under your real identity.",
  ability_desc_AnonymousProsecute:
    "Prosecute a player anonymously — they'll be put into custody and the trial is filed under your role, not your identity.",
  ability_desc_BackgroundCheck:
    "Look up a player's true name. The result appears privately in your Info channel.",
  ability_desc_TrueNameReveal:
    "Reveal a player's true name. The result appears privately in your Info channel.",
  ability_desc_NotebookReveal:
    "Check whether a player is currently holding a notebook. The result appears privately in your Info channel.",
  ability_desc_CivilianArrest:
    "Call a public arrest vote against a player. Any present player may vote; if a majority agrees, they're jailed for a while and then released.",
  ability_desc_Bug:
    "Plant a bug on a player. Their messages in loggable channels are relayed to a private surveillance feed only you can see. They're told they've been bugged, but not by whom.",
  ability_desc_PublicKidnap:
    "Kidnap a player: they're pulled into a private channel until released. When it ends, the kidnapper is revealed.",
  ability_desc_AnonymousKidnap:
    "Kidnap a player: they're pulled into a private channel until released. The kidnapping is anonymous — the kidnapper stays hidden on release.",
  ability_desc_UnlawfulArrest:
    "Jail a player immediately, with no vote. The world sees the imprisonment but never learns who ordered it.",
  ability_desc_UnderTheRadar:
    "Go off the record for the rest of the iteration. Nothing you say is logged, no bug relays you, and the contacts you open leave no trace in anyone's contact log. It does not make you inaudible: the people in a room still hear what you say there.",
  ability_desc_ShinigamiSacrifice:
    "Trade one of your own to a shinigami for another player's true name. The sacrifice dies, and the world is told what they were spent on. The name goes to the org.",
  ability_desc_KiraConnection:
    "Reach for Kira down a line you already have. Only a direct, non-anonymous line can establish who is really on the other end. The attempt lands in that lounge either way, naming you and saying whether it worked.",
  ability_desc_TrueNameReroll:
    "Give a player a new true name, drawn by the server. Anyone holding the old one is holding something worthless — and you will not be told the new one. Single use.",
  ability_desc_TapIn:
    "Read a contact channel's record by guessing its number. Lounges and group chats are numbered in one running sequence — you tap what you can work out from what you already know. Wrong guesses are limited, and the channel is told it was read — though never by whom.",
  ability_desc_SilentProsecute:
    "Name a player as wanted, with no trial and no vote. If they are wanted they die immediately and nothing is spent.",
  ability_desc_ForceInvite:
    "Put a player into the organization immediately. No true name is needed and they are not asked — they are simply in, and they see the organization's channel from the moment it lands. Someone the organization has blacklisted cannot be brought back this way.",
  ability_desc_Blackout:
    "Take the world dark. Nothing that happens is announced to anyone while it lasts, and the news goes off the air. Nothing is lost — everything held back arrives at once when it lifts. Players are still told who joins and what day it is, so an absence can be worked out from a roster; what they cannot learn is what became of anybody.",
  ability_desc_Autopsy:
    "Examine a dead player's record. Everything they said — including under a name that was not theirs — is laid out privately to you, named as them.",
  ability_desc_TrueNameInvite:
    "Invite a player by guessing their true name. Get it right and they join the org and their name is revealed to the members. Get it wrong and you have spent the attempt for nothing.",
  ability_desc_Outsource:
    "Delegate a prosecution: pull a player into the org and set them prosecuting someone on the org's behalf. Draws the org's invite and prosecution pools.",
  ability_desc_LeaderResign:
    "Step down as leader. Leadership passes on per the org's policy — some orgs require you to name your successor, others decide for themselves.",

  // ---- ability warnings ----
  //
  // The price of an ability that a player must weigh before firing it, rendered apart from the
  // description and in the danger colour. Only the abilities that carry an irreversible cost have
  // one; the rest have no key and show no warning.
  ability_warn_Blackout:
    "Everyone in the organization right now is permanently marked as wanted, and leaving later does not undo it. The organization itself is marked too, which exposes anyone who joins afterwards for as long as they stay.",
  ability_warn_SilentProsecute:
    "If they are not wanted, you are expelled from the organization and permanently barred from it, and the world is told your true name and which organization threw you out.",

  // ---- passive descriptions ----
  //
  // Passives are observed, never used. Keyed by the raw variant name; the ones carrying data
  // (a multiplier, a log kind) still read the same base description.
  passive_desc_Wanted:
    "You are marked as wanted. A silent prosecution against you lands without a trial, and orgs that hunt the wanted can act on you.",
  passive_desc_VoteAmplification: "Your vote counts for more than one, by the shown multiplier.",
  passive_desc_VolatileEyes:
    "Your Shinigami Eyes are fragile: a failed notebook check burns one of them.",
  passive_desc_ContactLogs:
    "Contacts you are party to are written down, building a record that can later be read back.",
  passive_desc_OwnedNotebookBlock:
    "A notebook you own cannot be used against you — writes naming you from it do nothing.",
  passive_desc_CustodyBugReceiver:
    "While a player is in custody, their monitored messages are relayed to you.",
} as const;

export type StringKey = keyof typeof STRINGS;
