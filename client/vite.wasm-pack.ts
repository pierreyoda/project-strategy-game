import { spawn } from 'node:child_process';
import { type FSWatcher, watch } from 'node:fs';
import { resolve } from 'node:path';
import type { Plugin } from 'vite';

/** Cargo workspace crates compiled into the WASM bridge. */
const GAME_RUST_CRATES = ['core', 'procgen', 'wasm_bridge'];
const RUST_SOURCE_REGEX = /\.rs$|Cargo\.toml$/;
const WASM_OUT_NAME = 'project-x-wasm-pkg';

/**
 * Builds the `wasm_bridge` crate with wasm-pack into `../wasm-pkg`, where Vite's WebAssembly ES module
 * integration picks it up. The Rust build matches Vite's mode (debug or release build), and in watch mode
 * the Rust crates are watched to rebuild the WASM package, which in turn triggers a Vite rebuild.
 */
export function wasmPack(): Plugin {
  let workspaceDir: string;
  let isProduction: boolean;
  let isWasmBuilt = false;
  let rebuild = Promise.resolve();
  let rebuildTimeout: NodeJS.Timeout | undefined;
  const watchers: FSWatcher[] = [];

  const buildWasm = () =>
    new Promise<void>((resolveBuild, rejectBuild) => {
      const args = [
        'build',
        isProduction ? '--release' : '--dev',
        ...['--target', 'bundler', '--out-dir', resolve(workspaceDir, 'wasm-pkg'), '--out-name', WASM_OUT_NAME],
        resolve(workspaceDir, 'wasm_bridge'),
      ];
      spawn('wasm-pack', args, { stdio: 'inherit' })
        .on('error', rejectBuild)
        .on('exit', code => (code === 0 ? resolveBuild() : rejectBuild(new Error(`wasm-pack exited with code ${code}`))));
    }).then(() => {
      isWasmBuilt = true;
    });

  const scheduleRebuild = () => {
    clearTimeout(rebuildTimeout);
    rebuildTimeout = setTimeout(() => {
      rebuild = rebuild.then(buildWasm).catch(err => console.error('[wasm-pack]', err));
    }, 100);
  };

  return {
    name: 'project-x:wasm-pack',
    configResolved(config) {
      workspaceDir = resolve(config.root, '..');
      isProduction = config.isProduction;
    },
    async buildStart() {
      if (this.meta.watchMode && watchers.length === 0) {
        for (const rustCrate of GAME_RUST_CRATES) {
          const watcher = watch(resolve(workspaceDir, rustCrate), { recursive: true }, (_, filename) => {
            if (filename && RUST_SOURCE_REGEX.test(filename)) scheduleRebuild();
          });
          watchers.push(watcher);
        }
      }
      // Watch mode rebuilds go through `scheduleRebuild`: only build here if no WASM package was built yet.
      if (!isWasmBuilt) await buildWasm();
    },
    closeWatcher() {
      clearTimeout(rebuildTimeout);
      watchers.forEach(watcher => watcher.close());
    },
  };
}
