<script lang="ts">
  import { getContext } from "svelte";
  import { CLIENT_KEY, type ClientState } from "../../client.svelte.ts";
  import Button from "../kit/Button.svelte";
  import { Flash } from "../../flash.svelte.ts";
  import FlashDisplay from "../Flash.svelte";
  import { now } from "../../time.svelte.ts";
  import AddPlayers from "./AddPlayers.svelte";

  const client = getContext<ClientState>(CLIENT_KEY);
  const flash = new Flash();

  // Initialize Engine, Update and Offset lived here and are gone:
  //   - the server issues InitializeEngine itself on first boot, so an admin could only ever have
  //     sent a redundant second one;
  //   - Update was a manual Null to force catchup, which the null tick now drives on the clock;
  //   - Offset shifted the client's sense of "now" to time-travel an engine hosted in-process.
  //     yagami overwrites the timestamp on arrival, so against a server it did nothing at all.
</script>

<div class="flex items-center gap-2">
  <AddPlayers />

  <Button
    variant="danger"
    onclick={async () => {
      const err = await client.dispatch({
        actor: "Admin",
        timestamp: now(),
        payload: { Crash: {} },
      });
      // A crash comes back as the "engine has crashed" string (the runtime is respawned
      // and resaturated behind the scenes); surface it so the crash is actually visible.
      if (err) flash.set_error(err);
      else flash.set_success("No crash?");
    }}>Crash</Button
  >

  <FlashDisplay {flash} />
</div>
