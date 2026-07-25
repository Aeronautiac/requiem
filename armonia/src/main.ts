// armonia's entry point. Its whole job is to build the host context and hand it to amane —
// everything the user sees is amane's.
import { mount } from "svelte";
import App from "amane/ui/App.svelte";
import "amane/ui/app.css";
import { createTauriHost } from "./transport_tauri";

const app = mount(App, {
  target: document.getElementById("app")!,
  props: { host: createTauriHost() },
});

export default app;
