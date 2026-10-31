import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

// Of the renderer's plugins, only Svelte is needed: for the runes of `.svelte.ts` modules, not Tailwind nor AssetPack
export default defineConfig({
  plugins: [svelte()],
  test: {
    include: ["src/**/*.test.ts"],
    // a DOM, for Svelte to compile modules as the renderer runs them and for the browser APIs they use
    environment: "happy-dom",
  },
});
