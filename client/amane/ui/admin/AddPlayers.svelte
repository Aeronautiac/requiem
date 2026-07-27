<script lang="ts">
  import Dialog from "../kit/Dialog.svelte";
  import Input from "../kit/Input.svelte";
  import Select from "../kit/Select.svelte";
  import Button from "../kit/Button.svelte";
  import { getContext } from "svelte";
  import type { ActionRequest, Role } from "../../bindings";
  import { ROLES } from "../../constants";
  import { now } from "../../time.svelte.ts";
  import { GAME_STATE_KEY, GameState } from "../../game_state.svelte.ts";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const game_state = getContext<GameState>(GAME_STATE_KEY);

  const client = getContext<ClientState>(CLIENT_KEY);

  let true_name: string = $state("");
  let role: Role = $state("Civilian");

  let open = $state(false);
  const flash = new Flash();
</script>

<Button size="sm" onclick={() => (open = true)}>Add Players</Button>

<Dialog bind:open title="Add Players">
  <!-- No display name here: creating the SLOT and saying who is on it are separate facts with
       separate lifetimes. The new player appears as `player-<slot>` and is named from the
       player list, which is also where a rename happens later. -->
  <Input bind:value={true_name} placeholder="True Name" />

  <Select bind:value={role} options={ROLES.map((r) => ({ value: r, label: r }))} />

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

      const err = await client.dispatch(request);
      if (err) {
        flash.set_error(`Action Failed: ${err}`);
      } else {
        flash.set_success(`Added ${true_name}. Name them from the player list.`);
      }
    }}>Add</Button
  >

  <FlashDisplay {flash} />
</Dialog>
