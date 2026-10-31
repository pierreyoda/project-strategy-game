import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

import { assetPack } from "./vite.assetpack.ts";

// https://vitejs.dev/config
export default defineConfig({
  plugins: [svelte(), tailwindcss(), assetPack()],
  server: {
    fs: {
      // the WASM bridge is built outside of the client (see `vite.wasm-pack.ts`), and the game's
      // scripts live at the repository's root, shared with the Rust tests
      allow: [".", "../wasm-pkg", "../scripts"],
    },
  },
  worker: {
    // Vite's WebAssembly ES module integration relies on top-level await, unsupported by IIFE workers
    format: "es",
  },
});
