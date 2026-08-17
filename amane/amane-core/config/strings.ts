// Every piece of user-facing copy the core produces. Nothing outside this file may contain a
// sentence: state code deals in values, and the value becomes words here.
//
// `{name}` placeholders are filled by `t()` in game/helpers.svelte.ts. Keys are typed, so a typo is a
// build error rather than a blank on screen.
export const STRINGS = {
  // ---- prosecution phase, short labels for the panel ----
  prosecution_label_awaiting_host: "Awaiting the host",
  prosecution_label_verdict_vote: "Trial vote",
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
  prosecution_verdict_vote_begun: "The trial vote for {defendant} has begun.",
  prosecution_started: "{prosecutor} is prosecuting {defendant}.",
  prosecution_entered_debate: "The trial of {defendant} has entered debate.",
  prosecution_trial_begun: "The trial of {defendant} has begun, the prosecution has the floor.",
  prosecution_presents: "In the trial of {defendant}, the prosecution presents.",
  prosecution_defense_floor: "In the trial of {defendant}, the defense has the floor.",
  prosecution_defense_presents: "In the trial of {defendant}, the defense presents.",

  // ---- how a refused or failed submission reads ----
  //
  // These exist because the state layer hands back a VALUE (Denied, Crashed, an ActionError) and
  // the render site turns it into this. Nothing in the pipeline holds the sentence.
  exec_denied: "You are not permitted to do that.",
  exec_crashed: "Your request caused a crash. The engine has been rebooted and rehydrated.",
  exec_desync: "The client and server are out of step. Please reconnect.",
  exec_lost_state: "The client lost track of the game state and cannot continue. Please reconnect.",
  exec_left_game: "Left the game.",

  control_Denied: "You are not an administrator of this game.",
  control_KeyNotFound: "That key does not exist.",
  control_CannotActOnSelf: "Supervisors cannot change their own key.",
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
  toast_kidnap_reveal_title: "Kidnap Recovery",
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
  toast_leader_title: "Leadership",
  toast_leader_gained: "You are now the leader of {org}.",
  toast_leader_lost: "You are no longer the leader of {org}.",
  toast_eye_deal_title: "Shinigami Eye Deal",
  toast_eye_deal_body: "{who} has taken the shinigami eye deal.",
  toast_false_accusation_title: "False Accusation",
  toast_false_accusation_body: "{name} (real name: {true_name}) has attempted to silently execute an innocent person. As a result, they have been permanently blacklisted from {org}.",
  toast_prosecution_title: "Prosecution",
  toast_prosecution_ended_title: "Prosecution Ended",
  toast_game_begins_title: "Let The Games Begin",
  toast_game_begins_body: "Day 1\nVarious interactions have now been unlocked.",
  toast_new_day_title: "New Day",
  toast_new_day_body: "Day {day}",
  toast_reveal_name_title: "Name Reveal",
  toast_reveal_name_body: "{name}'s true name is {true_name}.",
  toast_eyes_title: "Shinigami Eyes",
  toast_eyes_body: "You have {count} eye(s) remaining.",
  toast_reveal_notebook_title: "Notebook Check",
  toast_reveal_notebook_holding: "{name} is holding a notebook.",
  toast_reveal_notebook_empty: "{name} is not holding a notebook.",
  toast_bugged_title: "Surveillance",
  toast_bugged_explicit: "You have been bugged. Your messages are being monitored.",
  toast_bugged_custody: "Your messages are being monitored while in custody.",
  toast_notebook_received_title: "Notebook",
  toast_notebook_received_body: "A notebook has come into your possession.",
  toast_mention_title: "Mention",
  toast_mention_body: "{sender} mentioned you in {channel}.",
  toast_tap_in_title: "Tap In",
  toast_tap_in_found: "Successfully tapped into contact {id}.",
  toast_tap_in_no_contact: "Contact {id} does not exist.",
  toast_tap_in_not_loggable: "Contact {id} has logging off. Nothing happened.",
  toast_fake_lounge_title: "Fake Lounge Read",
  toast_fake_lounge_body: "Your fabricated lounge was read by {who}.",
  toast_fake_lounge_admin_title: "Fake Lounge Read",
  toast_fake_lounge_admin_body: "A fabricated lounge was read by {who}.",

  // ---- blackout, which is the only world event that announces its own silence ----
  blackout_begun_label: "Blackout",
  blackout_begun:
    "The world has gone dark...",
  blackout_over_label: "Blackout Over",
  blackout_over: "The light returns...",

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
  display_system: "Admin",
  display_org_unknown: "Org",
  display_unknown: "Unknown",

  // ---- ability descriptions ----
  //
  // The single source for what each ability does. The card's description toggle and the drill-in
  // form both read these, so the sentence lives in exactly one place. Keyed by the raw AbilityName;
  // per-side warnings and one-off usage notes stay in the ability's own form, since they are not
  // this description.
  ability_desc_Gun: "Shoot a player. They will die...",
  ability_desc_AnonymousContact:
    "Open an anonymous lounge with a player. They won't see who you are.",
  ability_desc_AnonymousAnnouncement: "Send an anonymous announcement to the news feed.",
  ability_desc_Pseudocide:
    "Fake a target's death (yourself included). They're revived after a delay.",
  ability_desc_FabricateLounge:
    "Fabricate a lounge between two players. You are given two profiles within a private channel allowing you to send messages as either player.",
  ability_desc_FalseAnonymousContact:
    "Open an anonymous lounge with a player posing as a role of your choosing. They won't see who you are.",
  ability_desc_Ipp:
    "Grant IPP to a player. Strengthened presence (cannot be kidnapped, imprisoned, etc...) and immunity from having their name written in a Notebook. Not immunity from death. If your name was written earlier, you will still die.",
  ability_desc_Prosecute:
    "Prosecute a player. They will immediately be put into custody, and the prosecution will be announced with you as the prosecutor.",
  ability_desc_AnonymousProsecute:
    "Anonymously prosecute a player. They will immediately be put into custody, and the prosecution will be announced with your role as the prosecutor.",
  ability_desc_BackgroundCheck:
    "Reveal a player's true name. The result is sent only to the user (in the case of an organization, it is sent to the organization's channel).",
  ability_desc_TrueNameReveal:
    "Reveal a player's true name. The result is sent only to the user (in the case of an organization, it is sent to the organization's channel).",
  ability_desc_NotebookReveal:
    "Check whether a player is currently holding a notebook. The result is sent only to the user (in the case of an organization, it is sent to the organization's channel).",
  ability_desc_CivilianArrest:
    "Call a civilian arrest vote against a player. Any present player may vote (not kidnapped, dead, etc...). If a majority agrees, they're jailed for a while and then released.",
  ability_desc_Bug:
    "Bug a player. Their messages in loggable channels are relayed to a private feed only you can see. They're told they've been bugged, but not by who.",
  ability_desc_PublicKidnap:
    "Kidnap a player. They're put into a private channel until released. When it ends, the kidnapper is revealed. Speaking in a kidnapping channel while not being the primary kidnapper will also reveal you. Be careful.",
  ability_desc_AnonymousKidnap:
    "Kidnap a player. they're put into a private channel until released. The kidnapping is anonymous, and the kidnapper will not be revealed on release.",
  ability_desc_UnlawfulArrest:
    "Jail a player immediately. The world is told about the imprisonment, but not who did it.",
  ability_desc_UnderTheRadar:
    "Go under the radar for the rest of the day. Nothing you say is logged, no bug relays you, and none of your contacts show up in any logs. This only applies to what YOU do, not what others do.",
  ability_desc_ShinigamiSacrifice:
    "Trade the life of an OG org member to a shinigami for someone's true name. The sacrifice dies, the world is told they were sacrificed, and the name is sent to the org.",
  ability_desc_KiraConnection:
    "Attempt to connect with Kira within a non-anonymous lounge. The attempt is revealed in the lounge, revealing you and saying whether or not it worked. This can show up in things like tap ins. If you succeed, your notebook is unlocked.",
  ability_desc_TrueNameReroll:
    "Reroll a player's true name. You will not be told what it is unless it's yours. If someone's old name has already been written in a notebook, this will not save them.",
  ability_desc_TapIn:
    "Read a contact channel's contets by guessing its number. Lounges and group chats are numbered in one running sequence. You only get a few wrong guesses. The channel is notified that it was tapped into, but not told who did it.",
  ability_desc_SilentProsecute:
    "Attempt to silently execute a target (no trial, no vote). If they are wanted they die immediately. If they aren't, your true name is leaked and you are banned from the org you used it from (if any).",
  ability_desc_ForceInvite:
    "Force a player into the organization. Does not bypass blacklist.",
  ability_desc_Blackout:
    "Start a blackout. Nothing announced to the world is revealed until the blackout is over, trials and polls are frozen, and the news is locked down.",
  ability_desc_Autopsy:
    "Reveal a dead player's messages. Anything they said, even posing as someone they aren't, is revealed to you.",
  ability_desc_TrueNameInvite:
    "Invite a player using their true name. Their true name is revealed to the org on success.",
  ability_desc_Outsource:
    "Outsource a prosecution to another player. They are instantly invited, and their true name is not revealed. This uses the same invite pool as other invite methods.",
  ability_desc_LeaderResign:
    "Resign leadership. The new leader is decided by the org's policy. Some require you to pick a new leader while others reroll independently.",

  // ---- ability warnings ----
  //
  // The price of an ability that a player must weigh before firing it, rendered apart from the
  // description and in the danger colour. Only the abilities that carry an irreversible cost have
  // one; the rest have no key and show no warning.
  ability_warn_Blackout:
    "Everyone in the organization right now is permanently marked as wanted. Leaving doesn't fix it. The organization is also marked. New members will receive wanted status as long as they remain in the org.",
  ability_warn_SilentProsecute:
    "If they are not wanted, you are permanently banned from the organization, and your true name is leaked as well as the org you were banned from.",

  // ---- passive descriptions ----
  //
  // Passives are observed, never used. Keyed by the raw variant name; the ones carrying data
  // (a multiplier, a log kind) still read the same base description.
  passive_desc_Wanted:
    "You are marked as wanted. You can be silently prosecuted.",
  passive_desc_VoteAmplification: "Your vote's weight is multiplied by the shown value.",
  passive_desc_VolatileEyes:
    "Your Shinigami Eyes are fragile... You will lose one if you guess a notebook reveal wrong. If you lose both, you can't use them anymore.",
  passive_desc_ContactLogs:
    "You can see a log of contacts and group chat additions/removals.",
  passive_desc_OwnedNotebookBlock:
    "If you are the original owner of a notebook, it cannot be used. You can clear this by connecting with Kira.",
  passive_desc_CustodyBugReceiver:
    "While a player is in custody, their messages are relayed to you.",
} as const;

export type StringKey = keyof typeof STRINGS;
