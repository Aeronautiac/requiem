// The browser host. Builds the transport, wires it to the platform, and mounts amane.
//
// This is the whole of what a host is: everything platform-shaped lives behind HostContext, and
// amane never reaches past it.
import { mount } from "svelte";
import App from "amane/ui/App.svelte";
import { createWebHost } from "amane/lib/host_web";
import { PlatformState } from "amane/platform.svelte";
import "amane/ui/app.css";

// Where yagami is. Override with VITE_YAGAMI_URL for a non-default bind.
const baseUrl = import.meta.env.VITE_YAGAMI_URL ?? "http://127.0.0.1:3000";

// Late-bound on purpose: the transport needs somewhere to report a dropped socket, and the thing
// that handles it is built FROM the transport. The closure only runs once a connection is live, by
// which point `platform` is assigned.
let platform: PlatformState;

const host = createWebHost({
  baseUrl,
  onDropped: (reason) => platform.dropped(reason),
});

platform = new PlatformState(host);

mount(App, {
  target: document.getElementById("app")!,
  props: { platform },
});
