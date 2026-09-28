// Generic top-level display for one ability: name, remaining usages, reset timing, and a Use
// button. It knows nothing ability-specific — clicking Use hands off to the ability's own
// configuration form (see AbilityMenu / the registry).
import { useState } from "react";
import type { AbilityName, OrgAbility } from "amane-client/bindings.ts";
import { OrgAbilityPolicyFlag } from "amane-client/bindings.ts";
import { abilityUsable, prettyAbility } from "amane-client/queries/abilities.ts";
import { abilityDescription, abilityWarning, roleLabel } from "amane-client/text.ts";
import { Button } from "../../kit/Button.tsx";

export function AbilityCard({
  name,
  successUsages,
  failureUsages,
  resets,
  baseReset,
  unlimited,
  hasUi,
  requirements,
  onUse,
}: {
  name: AbilityName;
  successUsages: number;
  failureUsages: number;
  resets: number;
  // Recharge period (config); shown up front so the cadence is known before a use. `resets` is the
  // live countdown, only meaningful once a use has armed it.
  baseReset: number;
  // No charge pools: unlimited use. When set, the counts and reset values are meaningless.
  unlimited: boolean;
  hasUi: boolean;
  // Org abilities only: the static gates on firing this. Undefined for a personal ability.
  requirements?: OrgAbility;
  onUse: () => void;
}) {
  const usable = hasUi && abilityUsable({ unlimited, successUsages, failureUsages });
  // Collapse to a single number when both outcomes agree (the common case), else break them out so
  // the asymmetry is visible.
  const same = successUsages === failureUsages;

  // The gates as short chips. These state what firing needs — they do NOT reflect whether it is
  // met right now (the org can't see members' secret roles or presence), so a use can still be
  // refused; the engine says why when it is.
  const gates: string[] = [];
  if (requirements) {
    if (requirements.require_members > 0) gates.push(`${requirements.require_members} members`);
    for (const role of requirements.require_roles) gates.push(`needs ${roleLabel(role)}`);
    if (requirements.usage_policies & OrgAbilityPolicyFlag.RequireLeader) gates.push("leader only");
    if (requirements.usage_policies & OrgAbilityPolicyFlag.RequireVote) gates.push("vote");
  }

  const description = abilityDescription(name);
  const warning = abilityWarning(name);
  const [showDesc, setShowDesc] = useState(false);

  return (
    <div className="flex flex-col gap-2 border border-edge bg-raised px-3 py-2">
      <div className="flex items-center justify-between gap-3">
        <div className="flex min-w-0 flex-col gap-1">
          <span className="text-sm text-ink">{prettyAbility(name)}</span>
          <span className="text-xs text-ink-dim">
            {unlimited ? (
              "Unlimited uses"
            ) : same ? (
              `${successUsages} use${successUsages === 1 ? "" : "s"} left`
            ) : (
              `${successUsages} on success · ${failureUsages} on failure`
            )}
            {!unlimited && baseReset > 0 ? ` · recharges every ${baseReset}` : ""}
            {!unlimited && resets > 0 ? ` · resets in ${resets}` : ""}
            {!hasUi ? " · no UI yet" : ""}
          </span>
          {gates.length > 0 && (
            <span className="flex flex-wrap gap-1">
              {gates.map((gate) => (
                <span key={gate} className="bg-panel px-1.5 py-px text-xs text-ink-dim">
                  {gate}
                </span>
              ))}
            </span>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-1.5">
          {(description || warning) && (
            <Button
              variant={showDesc ? "default" : "ghost"}
              size="sm"
              className="aspect-square px-0"
              aria-label="Toggle description"
              aria-pressed={showDesc}
              title="What this does"
              onClick={() => setShowDesc(!showDesc)}
            >
              ?
            </Button>
          )}
          <Button size="sm" disabled={!usable} onClick={onUse}>
            Use
          </Button>
        </div>
      </div>

      {showDesc && (description || warning) && (
        <div className="flex flex-col gap-1.5 border-t border-edge pt-2">
          {description && <p className="text-sm text-ink-dim">{description}</p>}
          {warning && <p className="text-sm text-danger-text">{warning}</p>}
        </div>
      )}
    </div>
  );
}
