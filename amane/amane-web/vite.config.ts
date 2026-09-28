import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), react()],
  server: {
    port: 5173,
    // Reachable from a second machine on the LAN. Open it on `localhost` though, not an IP: the
    // session cookie only rides requests to the same site as the page.
    host: true,
  },
});
