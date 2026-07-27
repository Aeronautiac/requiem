import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// Toolchain config, and it belongs to whoever BUILDS — amane ships source, so the compiler that
// turns `lang="ts"` into JS is the host's to choose. That is a different thing from the `$lib`
// alias amane used to make hosts declare, which was amane's own internal module resolution
// leaking outwards.
export default {
  preprocess: vitePreprocess(),
};
