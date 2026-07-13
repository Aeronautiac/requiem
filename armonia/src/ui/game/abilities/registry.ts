// Registry for the generic abilities menu. Each ability that has its own use UI maps
// its AbilityName to a component; abilities surfaced through a dedicated widget
// elsewhere are excluded so they don't appear twice.
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

// Every ability UI component takes the ability instance's id and a callback to run
// once the ability has been used (closes the menu). `orgId`, when set, means the ability
// belongs to that org: the same form is reused (so an org ability looks identical to a
// personal one) but the request is dispatched as UseOrgAbility — which may open an org
// vote — instead of a personal UseAbility.
export interface AbilityUiProps {
  abilityId: string;
  onDone: () => void;
  orgId?: string;
}

// Abilities with a dedicated surface elsewhere — kept out of the generic menu.
export const EXCLUDED_ABILITIES: ReadonlySet<AbilityName> = new Set<AbilityName>([
  "Contact", // Players widget
  "CreateGroupchat", // Channels widget (Group Chats category)
]);

// AbilityName -> its unique use UI. Names absent here have no frontend UI yet; the
// menu lists them but leaves them un-usable rather than pretending they work.
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
};

// "AnonymousContact" -> "Anonymous Contact"
export function prettyAbility(name: AbilityName): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2");
}

// Build the request to use a given ability instance with the collected behaviour args.
// A personal ability (orgId undefined) becomes UseAbility; an org ability becomes
// UseOrgAbility against that org (the engine decides whether it fires or opens a vote).
// The caller dispatches it through game.dispatch so it goes on the ordered pipe.
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
