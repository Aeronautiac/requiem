<script lang="ts">
  import DialogContent from "$lib/components/ui/dialog/dialog-content.svelte";
  import DialogHeader from "$lib/components/ui/dialog/dialog-header.svelte";
  import DialogTitle from "$lib/components/ui/dialog/dialog-title.svelte";
  import Dialog from "$lib/components/ui/dialog/dialog.svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import * as Select from "$lib/components/ui/select";
  import { getContext } from "svelte";
  import type { ActionRequest, Role } from "../../bindings";
  import { ROLES } from "../../constants";
  import type { Router } from "$lib/router";
  import { ROUTER_KEY } from "$lib/router";
  import { Button } from "$lib/components/ui/button";
  import { GAME_STATE_KEY, GameState } from "../../game_state.svelte.ts";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";

  const router = getContext<Router>(ROUTER_KEY);
  const game_state = getContext<GameState>(GAME_STATE_KEY);

  let display_name: string = $state("");
  let true_name: string = $state("");
  let role: Role = $state("Civilian");

  let open = $state(false);
  const flash = new Flash();
</script>

<Button size="sm" onclick={() => (open = true)}>Add Players</Button>

<Dialog bind:open>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Add Players</DialogTitle>
    </DialogHeader>

    <Input bind:value={display_name} placeholder="Display Name" />
    <Input bind:value={true_name} placeholder="True Name" />

    <Select.Root type="single" bind:value={role}>
      <Select.Trigger>{role}</Select.Trigger>
      <Select.Content>
        {#each ROLES as r}
          <Select.Item value={r}>{r}</Select.Item>
        {/each}
      </Select.Content>
    </Select.Root>

    <Button
      onclick={async () => {
        const request: ActionRequest = {
          actor: "Admin", // later enforce who is allowed to do this on the server
          timestamp: Date.now(),
          payload: {
            AddPlayer: {
              true_name: true_name,
              starting_role: role,
            },
          },
        };

        const err = game_state.process_response(
          await router.sendAction(request),
          { display_name },
        );
        if (err) {
          flash.set_error(`Action Failed: ${err}`);
        } else {
          flash.set_success(`Added ${display_name}.`);
        }
      }}>Add</Button
    >

    <FlashDisplay {flash} />
  </DialogContent>
</Dialog>
