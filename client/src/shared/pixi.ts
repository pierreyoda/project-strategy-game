import type { Application, Container } from "pixi.js";

/** What components drawing with Pixi need from the view hosting them. */
export type PixiContext = {
  app: Application;
  /** Container in world coordinates, moved around by the camera. */
  world: Container;
};

/** A color token of the design system (`index.css`), as PixiJS takes colors. */
export const tokenColor = (name: string): number =>
  parseInt(getComputedStyle(document.documentElement).getPropertyValue(name).trim().replace("#", ""), 16);
