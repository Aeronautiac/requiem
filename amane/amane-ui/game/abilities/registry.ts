// The generic abilities menu. Abilities surfaced through a dedicated widget elsewhere (Contact,
// CreateGroupchat) are excluded in amane-client/queries/abilities.ts so they don't appear twice.
import type { ComponentType } from "react";
import type { AbilityName } from "amane-client/bindings.ts";
import { AnonymousAnnouncementAbility } from "./AnonymousAnnouncementAbility.tsx";
import { AnonymousProsecuteAbility } from "./AnonymousProsecuteAbility.tsx";
import { AutopsyAbility } from "./AutopsyAbility.tsx";
import { BlackoutAbility } from "./BlackoutAbility.tsx";
import { CivilianArrestAbility } from "./CivilianArrestAbility.tsx";
import { FabricateLoungeAbility } from "./FabricateLoungeAbility.tsx";
import { FalseAnonymousContactAbility } from "./FalseAnonymousContactAbility.tsx";
import { KidnapAbility } from "./KidnapAbility.tsx";
import { KiraConnectionAbility } from "./KiraConnectionAbility.tsx";
import { LeaderResignAbility } from "./LeaderResignAbility.tsx";
import { OutsourceAbility } from "./OutsourceAbility.tsx";
import { ProsecuteAbility } from "./ProsecuteAbility.tsx";
import { PseudocideAbility } from "./PseudocideAbility.tsx";
import { ShinigamiEyeDealAbility } from "./ShinigamiEyeDealAbility.tsx";
import { ShinigamiSacrificeAbility } from "./ShinigamiSacrificeAbility.tsx";
import { TapInAbility } from "./TapInAbility.tsx";
import { targetAbility } from "./TargetAbility.tsx";
import { TrueNameInviteAbility } from "./TrueNameInviteAbility.tsx";
import { TrueNameRerollAbility } from "./TrueNameRerollAbility.tsx";
import { UnderTheRadarAbility } from "./UnderTheRadarAbility.tsx";
import { UnlawfulArrestAbility } from "./UnlawfulArrestAbility.tsx";

// `orgId`, when set, means the ability belongs to that org: the same form is reused, so an org
// ability looks identical to a personal one, but the request dispatches as UseOrgAbility — which
// may open an org vote — instead of UseAbility.
export interface AbilityUiProps {
  abilityId: string;
  orgId?: string;
  onDone: () => void;
}

// Names absent here have no frontend UI yet; the menu lists them but leaves them un-usable rather
// than pretending they work.
export const ABILITY_UIS: Partial<Record<AbilityName, ComponentType<AbilityUiProps>>> = {
  Gun: targetAbility("Fire", (target_id) => ({ Gun: { target_id } }), true),
  AnonymousContact: targetAbility("Contact anonymously", (target) => ({ AnonymousContact: { target } })),
  AnonymousAnnouncement: AnonymousAnnouncementAbility,
  Pseudocide: PseudocideAbility,
  FabricateLounge: FabricateLoungeAbility,
  FalseAnonymousContact: FalseAnonymousContactAbility,
  Ipp: targetAbility("Grant IPP", (target) => ({ Ipp: { target } })),
  Prosecute: ProsecuteAbility,
  AnonymousProsecute: AnonymousProsecuteAbility,
  BackgroundCheck: targetAbility("Run background check", (target) => ({ BackgroundCheck: { target } })),
  TrueNameReveal: targetAbility("Reveal true name", (target) => ({ TrueNameReveal: { target } })),
  NotebookReveal: targetAbility("Check for notebook", (target) => ({ NotebookReveal: { target } })),
  CivilianArrest: CivilianArrestAbility,
  Bug: targetAbility("Plant bug", (target) => ({ Bug: { target } })),
  PublicKidnap: KidnapAbility,
  AnonymousKidnap: KidnapAbility,
  UnlawfulArrest: UnlawfulArrestAbility,
  UnderTheRadar: UnderTheRadarAbility,
  ShinigamiSacrifice: ShinigamiSacrificeAbility,
  ShinigamiEyeDeal: ShinigamiEyeDealAbility,
  KiraConnection: KiraConnectionAbility,
  TrueNameReroll: TrueNameRerollAbility,
  TapIn: TapInAbility,
  SilentProsecute: targetAbility("Accuse", (target) => ({ SilentProsecute: { target } }), true),
  ForceInvite: targetAbility("Force invite", (target) => ({ ForceInvite: { target } })),
  Blackout: BlackoutAbility,
  Autopsy: AutopsyAbility,
  TrueNameInvite: TrueNameInviteAbility,
  Outsource: OutsourceAbility,
  LeaderResign: LeaderResignAbility,
};
