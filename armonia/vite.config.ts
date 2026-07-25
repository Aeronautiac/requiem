import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: {
      // amane is the client core, consumed as a library. Imports only ever go
      // armonia -> amane; nothing in amane may reach back into armonia.
      amane: path.resolve("../amane"),
      // $lib is amane's OWN internal convention (its shadcn primitives import
      // $lib/utils), so it has to resolve inside amane, not into armonia's src.
      $lib: path.resolve("../amane/lib"),
    },
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1430 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
