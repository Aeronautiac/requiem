<script lang="ts">
  import { execErrorText } from "../../game/helpers.svelte";
  import Dialog from "../kit/Dialog.svelte";
  import Input from "../kit/Input.svelte";
  import Select from "../kit/Select.svelte";
  import Button from "../kit/Button.svelte";
  import { getContext } from "svelte";
  import type { ActionRequest, Role } from "../../bindings";
  import { ROLES } from "../../constants";
  import { now } from "../../time.svelte.ts";
  import { SESSION_KEY, type SessionState } from "../../session.svelte.ts";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const session = getContext<SessionState>(SESSION_KEY);

  let true_name: string = $state("");
  let role: Role = $state("Civilian");

  let open = $state(false);
  const flash = new Flash();
</script>

<Button variant="ghost" size="sm" onclick={() => (open = true)}
  >Add Players</Button
>

<Dialog bind:open title="Add Players">
  <!-- No display name here: creating the SLOT and saying who is on it are separate facts with
       separate lifetimes. The new player appears as `player-<slot>` and is named from the
       player list, which is also where a rename happens later. -->
  <!-- Blank is how you ask the server for a drawn name: it fills an empty true_name from the
       reservoir and keeps anything you type. -->
  <Input bind:value={true_name} placeholder="True Name (blank to draw one)" />

  <Select
    bind:value={role}
    options={ROLES.map((r) => ({ value: r, label: r }))}
  />

  <Button
    onclick={async () => {
      const request: ActionRequest = {
        actor: "Admin", // later enforce who is allowed to do this on the server
        timestamp: now(),
        payload: {
          AddPlayer: {
            true_name: true_name,
            starting_role: role,
          },
        },
      };

      const reply = await session.submit_action(request);
      if (!reply.ok) {
        flash.set_error(`Action Failed: ${execErrorText(reply.error)}`);
      } else {
        // The drawn name never comes back on the response, so a blank submission cannot be
        // echoed here — the player's own Notifications log is where the name lands.
        flash.set_success(
          true_name
            ? `Added ${true_name}.`
            : "Added a player with a drawn name.",
        );
      }
    }}>Add</Button
  >

  <FlashDisplay {flash} />
</Dialog>
