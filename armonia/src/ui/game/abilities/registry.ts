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
import BackgroundCheckAbility from "./BackgroundCheckAbility.svelte";
import TrueNameRevealAbility from "./TrueNameRevealAbility.svelte";
import NotebookRevealAbility from "./NotebookRevealAbility.svelte";

// Every ability UI component takes the ability instance's id and a callback to run
// once the ability has been used (closes the menu).
export interface AbilityUiProps {
  abilityId: string;
  onDone: () => void;
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
  BackgroundCheck: BackgroundCheckAbility,
  TrueNameReveal: TrueNameRevealAbility,
  NotebookReveal: NotebookRevealAbility,
};

// "AnonymousContact" -> "Anonymous Contact"
export function prettyAbility(name: AbilityName): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2");
}

// Build the UseAbility request for a given ability instance + behaviour args. The
// caller dispatches it through game.dispatch so it goes on the ordered pipe.
export function useAbilityRequest(
  viewer: string,
  abilityId: string,
  behaviour: AbilityBehaviour,
): ActionRequest {
  return {
    actor: viewerToActor(viewer),
    timestamp: now(),
    payload: {
      UseAbility: {
        ability_id: slotKeyFromString(abilityId),
        ability_args: behaviour,
      },
    },
  };
}
