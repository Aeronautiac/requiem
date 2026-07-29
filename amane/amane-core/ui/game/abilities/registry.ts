// The generic abilities menu. Abilities surfaced through a dedicated widget elsewhere are
// excluded so they don't appear twice.
import type { Component } from "svelte";
import type {
  AbilityBehaviour,
  AbilityName,
  ActionRequest,
} from "../../../bindings";
import { slotKeyFromString } from "../../../bindings";
import { viewerToActor } from "../../../types";
import { now } from "../../../time.svelte.ts";

import GunAbility from "./GunAbility.svelte";
import AnonymousContactAbility from "./AnonymousContactAbility.svelte";
import AnonymousAnnouncementAbility from "./AnonymousAnnouncementAbility.svelte";
import PseudocideAbility from "./PseudocideAbility.svelte";
import FabricateLoungeAbility from "./FabricateLoungeAbility.svelte";
import FalseAnonymousContactAbility from "./FalseAnonymousContactAbility.svelte";
import IppAbility from "./IppAbility.svelte";
import ProsecuteAbility from "./ProsecuteAbility.svelte";
import AnonymousProsecuteAbility from "./AnonymousProsecuteAbility.svelte";
import BackgroundCheckAbility from "./BackgroundCheckAbility.svelte";
import TrueNameRevealAbility from "./TrueNameRevealAbility.svelte";
import NotebookRevealAbility from "./NotebookRevealAbility.svelte";
import CivilianArrestAbility from "./CivilianArrestAbility.svelte";
import BugAbility from "./BugAbility.svelte";
import KidnapAbility from "./KidnapAbility.svelte";
import UnlawfulArrestAbility from "./UnlawfulArrestAbility.svelte";
import UnderTheRadarAbility from "./UnderTheRadarAbility.svelte";
import ShinigamiSacrificeAbility from "./ShinigamiSacrificeAbility.svelte";
import KiraConnectionAbility from "./KiraConnectionAbility.svelte";
import TrueNameRerollAbility from "./TrueNameRerollAbility.svelte";
import TapInAbility from "./TapInAbility.svelte";
import SilentProsecuteAbility from "./SilentProsecuteAbility.svelte";

// `orgId`, when set, means the ability belongs to that org: the same form is reused, so an org
// ability looks identical to a personal one, but the request is dispatched as UseOrgAbility —
// which may open an org vote — instead of UseAbility.
export interface AbilityUiProps {
  abilityId: string;
  onDone: () => void;
  orgId?: string;
}

export const EXCLUDED_ABILITIES: ReadonlySet<AbilityName> = new Set<AbilityName>([
  "Contact", // Players widget
  "CreateGroupchat", // Channels widget (Group Chats category)
]);

// Names absent here have no frontend UI yet; the menu lists them but leaves them un-usable rather
// than pretending they work.
export const ABILITY_UIS: Partial<
  Record<AbilityName, Component<AbilityUiProps>>
> = {
  Gun: GunAbility,
  AnonymousContact: AnonymousContactAbility,
  AnonymousAnnouncement: AnonymousAnnouncementAbility,
  Pseudocide: PseudocideAbility,
  FabricateLounge: FabricateLoungeAbility,
  FalseAnonymousContact: FalseAnonymousContactAbility,
  Ipp: IppAbility,
  Prosecute: ProsecuteAbility,
  AnonymousProsecute: AnonymousProsecuteAbility,
  BackgroundCheck: BackgroundCheckAbility,
  TrueNameReveal: TrueNameRevealAbility,
  NotebookReveal: NotebookRevealAbility,
  CivilianArrest: CivilianArrestAbility,
  Bug: BugAbility,
  PublicKidnap: KidnapAbility,
  AnonymousKidnap: KidnapAbility,
  UnlawfulArrest: UnlawfulArrestAbility,
  UnderTheRadar: UnderTheRadarAbility,
  ShinigamiSacrifice: ShinigamiSacrificeAbility,
  KiraConnection: KiraConnectionAbility,
  TrueNameReroll: TrueNameRerollAbility,
  TapIn: TapInAbility,
  SilentProsecute: SilentProsecuteAbility,
};

// "AnonymousContact" -> "Anonymous Contact"
export function prettyAbility(name: AbilityName): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2");
}

// The engine decides whether an org ability fires or opens a vote.
export function useAbilityRequest(
  viewer: string,
  abilityId: string,
  orgId: string | undefined,
  behaviour: AbilityBehaviour,
): ActionRequest {
  return {
    actor: viewerToActor(viewer),
    timestamp: now(),
    payload: orgId
      ? {
          UseOrgAbility: {
            org_id: slotKeyFromString(orgId),
            ability_id: slotKeyFromString(abilityId),
            ability_args: behaviour,
          },
        }
      : {
          UseAbility: {
            ability_id: slotKeyFromString(abilityId),
            ability_args: behaviour,
          },
        },
  };
}
