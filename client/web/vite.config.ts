import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// No path aliases. `amane` resolves through the npm workspace link, and amane's own imports are
// all relative — a host used to have to declare amane's internal `$lib` alias for it to build at
// all, which is exactly the dependency this package must not have.
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  server: {
    port: 5173,
    // Two browser profiles pointed at one game is the point of this host, so bind it where a
    // second machine on the LAN can reach it too.
    host: true,
  },
});
