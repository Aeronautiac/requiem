// The browser host. Builds the transport, wires it to the client, and mounts amane.
//
// This is the whole of what a host is: everything platform-shaped lives behind HostContext, and
// amane never reaches past it.
import { mount } from "svelte";
import App from "amane-core/ui/App.svelte";
import { createWebHost } from "amane-core/lib/host_web";
import { ClientState } from "amane-core/client.svelte";
import "amane-core/ui/app.css";

// Where yagami is. Override with VITE_YAGAMI_URL for a non-default bind.
const baseUrl = import.meta.env.VITE_YAGAMI_URL ?? "http://127.0.0.1:3000";

// Late-bound on purpose: the transport needs somewhere to report a dropped socket, and the thing
// that handles it is built FROM the transport. The closure only runs once a connection is live, by
// which point `client` is assigned.
let client: ClientState;

const host = createWebHost({
  baseUrl,
  onDropped: (reason) => client.dropped(reason),
});

client = new ClientState(host);

mount(App, {
  target: document.getElementById("app")!,
  props: { client },
});
