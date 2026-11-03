/**
 * The map's lenses: other ways of looking at the same map, each stepping the terrain back to bring
 * one thing forward.
 *
 * A lens is the player's view and nothing more: the simulation never hears of it, and switching one
 * on changes what is drawn, never what is known.
 */

import type { DepositResource, MapDeposit } from "../../map/deposits";
import { NO_NATION, type NationMeta, TileVisibility } from "../session/protocol";
import type { WorldState } from "./worldState";

export const LENS_KINDS = ["resources", "political"] as const;
export type LensKind = (typeof LENS_KINDS)[number];

/** Which deposits the resource lens shows: all of them, or those of one resource. */
export type ResourceFilter = "all" | DepositResource;

export type Lens = { kind: "none" } | { kind: "resources"; filter: ResourceFilter } | { kind: "political" };

export const NO_LENS: Lens = { kind: "none" };

export const LENS_NAMES: Record<LensKind, string> = {
  resources: "Resources",
  political: "Political",
};

/** The key switching each lens on and off, as the tooltips show it. */
export const LENS_HOTKEYS: Record<LensKind, string> = {
  resources: "R",
  political: "P",
};

/** The lens a click on `kind`'s button leaves: that one, or none when it was already on. */
export const lensToggled = (lens: Lens, kind: LensKind): Lens => {
  if (lens.kind === kind) {
    return NO_LENS;
  }
  return kind === "resources" ? { kind, filter: "all" } : { kind };
};

/** The resource lens narrowed to `filter`. Any other lens is left as it is. */
export const lensFiltered = (lens: Lens, filter: ResourceFilter): Lens =>
  lens.kind === "resources" ? { kind: "resources", filter } : lens;

/**
 * The lens a key press switches, or `null` for any other key.
 *
 * Read from the character typed rather than the physical key, so that the hotkeys are the letters
 * their tooltips show whatever the keyboard's layout. Not while a modifier is held: those shortcuts
 * belong to the application.
 */
export const lensForKey = (event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "altKey">): LensKind | null => {
  if (event.ctrlKey || event.metaKey || event.altKey) {
    return null;
  }
  const key = event.key.toUpperCase();
  return LENS_KINDS.find((kind) => LENS_HOTKEYS[kind] === key) ?? null;
};

/** The deposits the resource lens shows under `filter`: never one run dry, which has nothing left to show. */
export const filteredDeposits = (deposits: readonly MapDeposit[], filter: ResourceFilter): readonly MapDeposit[] =>
  deposits.filter(({ resource, exhausted }) => !exhausted && (filter === "all" || resource === filter));

/**
 * The nations the political lens names in its legend: the viewer's own first, then every nation
 * holding a tile the viewer has explored or reaching into view with its sphere, by id.
 *
 * Nobody is listed the viewer has seen nothing of: the legend would give rivals away before the map does.
 */
export const knownNations = (world: WorldState, viewer: number): NationMeta[] => {
  const known = new Set<number>([viewer]);
  for (let index = 0; index < world.tileOwner.length; index++) {
    const owner = world.tileOwner[index];
    if (owner !== NO_NATION && world.visibility[index] !== TileVisibility.Unexplored) {
      known.add(owner);
    }
    const sphere = world.tileSphere[index];
    if (sphere !== NO_NATION) {
      known.add(sphere);
    }
  }
  return world.meta.nations
    .filter(({ id }) => known.has(id))
    .sort((a, b) => (a.id === viewer ? -1 : b.id === viewer ? 1 : a.id - b.id));
};
