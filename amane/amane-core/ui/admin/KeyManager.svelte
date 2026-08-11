<script lang="ts">
  // Minting the keys that let anyone else into the game, and managing the ones that exist.
  //
  // Creating a game mints exactly one key — the admin's — so every other participant's credential
  // has to come from here. A key is not a person and not a connection: it is a privilege set, and
  // "associating it with an actor" means naming the slots its holder may play as.
  //
  // The set that already exists arrives on the KeyRoster, a whole-set server command gated to
  // admins, and is rendered below as one list: what each key permits, and how to change or revoke
  // it. Authority to touch a given key is the server's to decide — a supervisor key, or the full
  // set, or one's own key — and a refused control surfaces its error here, so the table offers
  // everything and the server denies what it must. A freshly created key lands in the same list
  // the moment the roster refreshes, no separate handling. Secret tokens are shown plainly: keys
  // are how someone gets in, handed over out of band (see copy), not worth treating as the crown
  // jewels.
  import { getContext } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { GAME_STATE_KEY, type GameState } from "../../game/state.svelte";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { execErrorText, playerLabel } from "../../game/helpers.svelte";
  import { slotKeyFromString, slotKeyToString } from "../../bindings";
  import type { ActorScope, Capability, PrivilegeSet } from "../../bindings";
  import Button from "../kit/Button.svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Select from "../kit/Select.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game = getContext<GameState>(GAME_STATE_KEY);
  const session = getContext<SessionState>(SESSION_KEY);
  const flash = new Flash();

  // What a key may do beyond acting as its actors. Supervise never appears without Administer
  // because a key holding it alone is inert — every control is gated on Administer first, and
  // Supervise only widens which OTHER keys an administrator may touch.
  type Grant = "player" | "admin" | "supervisor";

  const GRANTS: { value: Grant; label: string }[] = [
    { value: "player", label: "Player — no administration" },
    { value: "admin", label: "Administrator — manages ordinary keys" },
    { value: "supervisor", label: "Supervisor — manages administrators too" },
  ];

  const CAPABILITIES: Record<Grant, Capability[]> = {
    player: [],
    admin: ["Administer"],
    supervisor: ["Administer", "Supervise"],
  };

  // ---- minting ----

  let open = $state(false);
  let grant = $state<Grant>("player");
  let every_actor = $state(false);
  const chosen = new SvelteSet<string>();
  let busy = $state(false);

  // The System view, never the selected one: an admin reading a player's view must still be able to
  // mint for every slot. Orgs are excluded deliberately — no connection may act as an org (the
  // engine instantiates org actions from a member's own), so an org in a key's scope grants nothing.
  const players = $derived.by(() => {
    const all = game.system_view().players;
    return [...all.keys()]
      .map((id) => ({ id, label: playerLabel(id, all) }))
      .sort((a, b) => a.label.localeCompare(b.label));
  });

  // ---- the ledger ----

  // Every key on the roster, in display order. The whole set, delivered a fresh copy each change.
  const keys = $derived.by(() => {
    const sys = game.system_view();
    return [...sys.keys.entries()].sort((a, b) => a[0].localeCompare(b[0]));
  });

  function toggle(id: string) {
    if (chosen.has(id)) chosen.delete(id);
    else chosen.add(id);
  }

  function scope(): ActorScope {
    return every_actor ? "All" : { Only: [...chosen].map(slotKeyFromString) };
  }

  // The capabilities a set amounts to, pinned to the three grant bins the UI offers.
  function grantFor(caps: Capability[]): Grant {
    if (caps.includes("Supervise")) return "supervisor";
    if (caps.includes("Administer")) return "admin";
    return "player";
  }

  function grantLabel(privileges: PrivilegeSet): string {
    return GRANTS.find((g) => g.value === grantFor(privileges.capabilities))!.label;
  }

  function scopeLabel(privileges: PrivilegeSet): string {
    if (privileges.actors === "All") return "Every actor";
    return (
      privileges.actors.Only.map((a) => playerLabel(slotKeyToString(a), game.system_view().players))
        .filter(Boolean)
        .join(", ") || "No actors"
    );
  }

  function reset_mint() {
    chosen.clear();
    every_actor = false;
    grant = "player";
  }

  async function create() {
    // A key with neither actors nor administration can do nothing at all, and would only ever be
    // discovered as such by whoever was handed it.
    if (!every_actor && chosen.size === 0 && grant === "player") {
      flash.set_error("Pick at least one player, or grant administration.");
      return;
    }

    busy = true;
    const reply = await session.submit_control({
      CreateKey: { actors: scope(), capabilities: CAPABILITIES[grant] },
    });
    busy = false;

    if (!reply.ok) {
      flash.set_error(execErrorText(reply.error));
      return;
    }
    // Every other control answers with a bare tag, so a response without the key means the reply
    // stream is not what this client thinks it is — say so rather than showing a key that isn't
    // about to appear in the ledger.
    if (typeof reply.value === "string" || !("KeyCreated" in reply.value)) {
      flash.set_error("The server answered something other than a key.");
      return;
    }

    // The new key lands in the ledger (and thus the list below) on the roster refresh that follows
    // the mint; nothing special is done with the key from the reply alone.
    reset_mint();
    flash.set_success("Key created — it's in the list below.");
  }

  async function copy(key: string) {
    // Absent on an insecure origin, so this is a real case rather than defensive padding.
    if (!navigator.clipboard) {
      flash.set_error("No clipboard here — select the key and copy it by hand.");
      return;
    }
    try {
      await navigator.clipboard.writeText(key);
      flash.set_success("Copied.");
    } catch {
      flash.set_error("Could not copy — select the key and copy it by hand.");
    }
  }

  // ---- editing an existing key ----

  // The key currently being edited, and the drafts of its capabilities and scope. Editing is
  // draft-then-commit: nothing reaches the server until Save, and the drafts start from the key's
  // current set, so an untouched Save is a no-op re-statement.
  let editing = $state<string | null>(null);
  let edit_grant = $state<Grant>("player");
  let edit_every = $state(false);
  const edit_chosen = new SvelteSet<string>();
  let saving = $state(false);

  function begin_edit(key: string) {
    const privileges = game.system_view().keys.get(key);
    if (!privileges) return;
    editing = key;
    edit_grant = grantFor(privileges.capabilities);
    edit_every = privileges.actors === "All";
    edit_chosen.clear();
    if (privileges.actors !== "All") {
      for (const actor of privileges.actors.Only) edit_chosen.add(slotKeyToString(actor));
    }
  }

  function toggle_edit(id: string) {
    if (edit_chosen.has(id)) edit_chosen.delete(id);
    else edit_chosen.add(id);
  }

  function edit_scope(): ActorScope {
    return edit_every ? "All" : { Only: [...edit_chosen].map(slotKeyFromString) };
  }

  async function save() {
    const key = editing;
    if (!key) return;
    saving = true;
    const set_caps = await session.submit_control({
      SetCapabilities: { key, capabilities: CAPABILITIES[edit_grant] },
    });
    if (!set_caps.ok) {
      flash.set_error(execErrorText(set_caps.error));
      saving = false;
      return;
    }
    const set_scope = await session.submit_control({
      SetActorScope: { key, actors: edit_scope() },
    });
    saving = false;
    if (!set_scope.ok) {
      flash.set_error(execErrorText(set_scope.error));
      return;
    }
    flash.set_success("Key updated.");
    editing = null;
  }

  async function revoke(key: string) {
    if (!confirm("Revoke this key? Its holder is disconnected immediately.")) return;
    const reply = await session.submit_control({ RevokeKey: { key } });
    if (!reply.ok) flash.set_error(execErrorText(reply.error));
    else {
      flash.set_success("Key revoked.");
      if (editing === key) editing = null;
    }
  }
</script>

<Button variant="ghost" size="sm" onclick={() => (open = true)}>Keys</Button>

<Dialog bind:open title="Keys">
  <p class="text-xs text-ink-dim">
    A key is how someone gets in. Choose the players its holder may act as, hand it over out of
    band, and they join with it. Every existing key shows below with what it may do.
  </p>

  <!-- mint -->
  <label class="flex items-center gap-2 text-sm">
    <input type="checkbox" bind:checked={every_actor} />
    Every actor, including ones added later
  </label>

  <div
    class="max-h-48 overflow-y-auto rounded-md border border-edge"
    class:opacity-50={every_actor}
  >
    {#each players as player (player.id)}
      <label class="flex items-center gap-2 px-2 py-1 text-sm hover:bg-raised">
        <input
          type="checkbox"
          disabled={every_actor}
          checked={chosen.has(player.id)}
          onchange={() => toggle(player.id)}
        />
        {player.label}
      </label>
    {:else}
      <p class="px-2 py-1 text-xs text-ink-dim">No players yet — add some first.</p>
    {/each}
  </div>

  <Select bind:value={grant} options={GRANTS} class="w-full" />

  <Button disabled={busy} onclick={create}>Create key</Button>

  <!-- the ledger: the same list a fresh key lands in -->
  <div class="flex flex-col gap-2 border-t border-edge pt-3">
    <p class="text-xs text-ink-dim">Keys, as the server holds them.</p>
    {#each keys as [key, privileges] (key)}
      <div class="rounded-md border border-edge p-2">
        {#if editing === key}
          <label class="flex items-center gap-2 text-sm">
            <input type="checkbox" bind:checked={edit_every} />
            Every actor, including ones added later
          </label>
          <div
            class="max-h-32 overflow-y-auto rounded-md border border-edge"
            class:opacity-50={edit_every}
          >
            {#each players as player (player.id)}
              <label class="flex items-center gap-1 px-2 py-1 text-sm hover:bg-raised">
                <input
                  type="checkbox"
                  disabled={edit_every}
                  checked={edit_chosen.has(player.id)}
                  onchange={() => toggle_edit(player.id)}
                />
                {player.label}
              </label>
            {/each}
          </div>
          <Select bind:value={edit_grant} options={GRANTS} class="w-full" />
          <div class="flex gap-2">
            <Button size="sm" disabled={saving} onclick={save}>Save</Button>
            <Button size="sm" variant="ghost" onclick={() => (editing = null)}>Cancel</Button>
          </div>
        {:else}
          <div class="flex items-center justify-between gap-2">
            <div class="min-w-0">
              <code class="block truncate font-mono text-xs">{key}</code>
              <p class="text-xs text-ink-dim">{grantLabel(privileges)}</p>
              <p class="text-xs text-ink-dim">{scopeLabel(privileges)}</p>
            </div>
            <div class="flex shrink-0 gap-2">
              <Button size="sm" variant="ghost" onclick={() => copy(key)}>Copy</Button>
              <Button size="sm" variant="ghost" onclick={() => begin_edit(key)}>Edit</Button>
              <Button size="sm" variant="danger" onclick={() => revoke(key)}>Revoke</Button>
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <p class="text-xs text-ink-dim">No keys yet — create one above.</p>
    {/each}
  </div>

  <FlashDisplay {flash} />
</Dialog>