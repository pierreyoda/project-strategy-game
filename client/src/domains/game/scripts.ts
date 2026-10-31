/**
 * The scripts a new game is started with, bundled from the repository's `scripts/` folder.
 *
 * The Rust tests `include_str!` the very same files. The bridge compiles what it is sent and embeds
 * nothing, so modding and hot reload will change who fills this list, never the bridge.
 */

import scoutAi from "../../../../scripts/ai/scout.rhai?raw";
import defaultScenario from "../../../../scripts/scenarios/default.rhai?raw";

import type { ScriptSource } from "./session/protocol";

/** The default scenario, and the scout AI playing every nation but the player's. */
export const DEFAULT_SCRIPTS: readonly ScriptSource[] = [
  { name: "default", kind: "scenario", source: defaultScenario },
  { name: "scout", kind: "ai", source: scoutAi },
];
