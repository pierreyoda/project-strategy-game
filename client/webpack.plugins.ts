import { type resolve as INodePathResolve } from 'path';
import type IWasmPackPlugin from '@wasm-tool/wasm-pack-plugin';
import type IForkTsCheckerWebpackPlugin from 'fork-ts-checker-webpack-plugin';

/* eslint-disable @typescript-eslint/no-var-requires */
const resolve: typeof INodePathResolve = require('path').resolve;
const WasmPackPlugin: typeof IWasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const ForkTsCheckerWebpackPlugin: typeof IForkTsCheckerWebpackPlugin = require('fork-ts-checker-webpack-plugin');

/** @typeof readonly string[] */
const GAME_RUST_CRATES = ['core', 'procedural', 'scripting', 'wasm-bridge'];
/** @typeof readonly string[] */
const RUST_WATCHED_DIRS_RELATIVE = GAME_RUST_CRATES.map(rustCrate => resolve(__dirname, '../', rustCrate));
const WASM_OUT_NAME = 'project-x-wasm-pkg';

export const plugins = [
  // Rust WASM build will match webpack mode (debug or release build)
  new WasmPackPlugin({
    extraArgs: '--target bundler',
    crateDirectory: resolve(__dirname, '../wasm_bridge/'),
    watchDirectories: RUST_WATCHED_DIRS_RELATIVE,
    forceWatch: true, // align with webpack's watch mode
    outDir: resolve(__dirname, '../wasm-pkg/'),
    outName: WASM_OUT_NAME,
    pluginLogLevel: 'info',
  }),
  new ForkTsCheckerWebpackPlugin({
    logger: 'webpack-infrastructure',
  }),
];
