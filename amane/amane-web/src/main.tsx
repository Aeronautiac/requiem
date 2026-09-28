// The browser host's entry: build the Host, build the Client, mount the UI.
import { Client } from "amane-client/client.ts";
import { App } from "amane-ui/App.tsx";
import "amane-ui/app.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { createWebHost } from "./host.ts";

// Where yagami is. Must share a hostname with this page (see host.ts). Override with
// VITE_YAGAMI_URL.
const base_url = import.meta.env.VITE_YAGAMI_URL ?? "http://localhost:3000";

const client = new Client(createWebHost(base_url));
void client.refresh_account();

createRoot(document.getElementById("app")!).render(
  <StrictMode>
    <App client={client} />
  </StrictMode>,
);
