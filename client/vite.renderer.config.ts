import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// https://vitejs.dev/config
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  server: {
    fs: {
      // the WASM bridge is built outside of the client (see `vite.wasm-pack.ts`)
      allow: [".", "../wasm-pkg"],
    },
  },
  worker: {
    // Vite's WebAssembly ES module integration relies on top-level await, unsupported by IIFE workers
    format: "es",
  },
});
