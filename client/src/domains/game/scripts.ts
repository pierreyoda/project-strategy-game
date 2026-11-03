/**
 * The scripts a new game is started with, bundled from the repository's `scripts/` folder.
 *
 * The Rust tests `include_str!` the very same files. The bridge compiles what it is sent and embeds
 * nothing, so modding and hot reload will change who fills this list, never the bridge.
 */

import defaultAi from "../../../../scripts/ai/default.rhai?raw";
import defaultScenario from "../../../../scripts/scenarios/default.rhai?raw";

import type { ScriptSource } from "./session/protocol";

/** The default scenario, and the default AI playing every nation but the player's: an economy, and scouts. */
export const DEFAULT_SCRIPTS: readonly ScriptSource[] = [
  { name: "default", kind: "scenario", source: defaultScenario },
  { name: "default_ai", kind: "ai", source: defaultAi },
];
