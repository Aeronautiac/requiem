<script lang="ts">
  import DialogContent from "$lib/components/ui/dialog/dialog-content.svelte";
  import DialogHeader from "$lib/components/ui/dialog/dialog-header.svelte";
  import DialogTitle from "$lib/components/ui/dialog/dialog-title.svelte";
  import Dialog from "$lib/components/ui/dialog/dialog.svelte";
  import Input from "$lib/components/ui/input/input.svelte";
  import * as Select from "$lib/components/ui/select";
  import { getContext } from "svelte";
  import type { AddPlayer, ActionRequest, Role } from "./bindings";
  import { ROLES } from "./constants";
  import type { Router } from "$lib/router";
  import { ROUTER_KEY } from "$lib/router";
  import { Button } from "$lib/components/ui/button";

  const router = getContext<Router>(ROUTER_KEY);

  let display_name: string = $state("");
  let true_name: string = $state("");
  let role: Role = $state("Civilian");

  let open = $state(false);

  let error: string | null = $state(null);
  let error_timer: ReturnType<typeof setTimeout> | null = null;

  function set_error(msg: string) {
    if (error_timer !== null) clearTimeout(error_timer);
    error = msg;
    error_timer = setTimeout(() => {
      error = null;
    }, 3000);
  }
</script>

<Button onclick={() => (open = true)}>Add Players</Button>

<Dialog bind:open>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Add Players</DialogTitle>
    </DialogHeader>

    <Input bind:value={display_name} placeholder="Display Name" />
    <Input bind:value={true_name} placeholder="True Name" />

    <Select.Root bind:value={role}>
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
        const { exec_result } = await router.sendAction(request);
        console.log(exec_result);

        if (exec_result === "Crashed") {
          set_error("Action Failed: The engine has crashed.");
        } else {
          const { Ok, Err } = exec_result.Standard;
          if (Err != null) {
            set_error(`Action Failed: ${Err}`);
          } else if (Ok != null) {
            // call upon game state
          }
        }
      }}>Add</Button
    >

    {#if error !== null}
      <p class="text-red-500">{error}</p>
    {/if}
  </DialogContent>
</Dialog>
