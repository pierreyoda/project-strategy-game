import type { Application, Container } from "pixi.js";

/** What components drawing with Pixi need from the view hosting them. */
export type PixiContext = {
  app: Application;
  /** Container in world coordinates, moved around by the camera. */
  world: Container;
};
