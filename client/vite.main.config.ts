import { defineConfig } from 'vite';

import { wasmPack } from './vite.wasm-pack.ts';

// https://vitejs.dev/config
export default defineConfig({
  build: {
    // Vite's WebAssembly ES module integration relies on top-level await, which Forge's default CommonJS
    // output does not support: the main process is built as an ES module instead.
    lib: {
      entry: 'src/index.ts',
      formats: ['es'],
      fileName: () => '[name].js',
    },
    rolldownOptions: {
      // Rolldown only infers a Node.js target for CommonJS output, and bundled CommonJS dependencies
      // (like `electron-squirrel-startup`) need a working `require` for Node.js built-ins.
      platform: 'node',
    },
  },
  plugins: [wasmPack()],
});
