/**
 * What the map draws of the deposits under its tiles.
 *
 * Like `entities.ts`, nothing here knows what a deposit holds in figures: a deposit arrives as the
 * resource it is and the level the simulation reads it at, and the renderer stays a renderer.
 */

import { Assets, type Spritesheet, type Texture } from "pixi.js";

import { initializeAssets } from "./assets";

/** The resources found in the ground, which the `deposits` set has art for. */
export const DEPOSIT_RESOURCES = ["water", "oil", "metals", "uranium"] as const;
export type DepositResource = (typeof DEPOSIT_RESOURCES)[number];

/** Trace, deposit, rich: the levels the simulation reads a deposit at (`DEPOSIT_LEVELS` in the protocol). */
export type DepositLevel = 1 | 2 | 3;

/** A deposit to draw: which tile it lies on, what it holds, how rich it is. */
export type MapDeposit = {
  /** Row-major tile index. */
  tile: number;
  resource: DepositResource;
  level: DepositLevel;
};

export const isDepositResource = (resource: string): resource is DepositResource =>
  (DEPOSIT_RESOURCES as readonly string[]).includes(resource);

/** Built by AssetPack from `raw-assets/deposits{m}{tps}/`, the art of `art/deposits/`. */
const DEPOSITS_BUNDLE = "deposits";

/** The frame of a deposit in the sprite sheet. */
export const depositFrame = ({ resource, level }: Pick<MapDeposit, "resource" | "level">): string =>
  `${resource}_${level}.png`;

/** The frames of the deposits, by frame name. */
export type DepositTextures = Record<string, Texture>;

/** Load the deposits' sprite sheet. */
export const loadDepositTextures = async (): Promise<DepositTextures> => {
  await initializeAssets();
  await Assets.loadBundle(DEPOSITS_BUNDLE);
  return Assets.get<Spritesheet>(DEPOSITS_BUNDLE).textures;
};
