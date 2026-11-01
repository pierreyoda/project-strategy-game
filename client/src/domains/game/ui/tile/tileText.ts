import { TileVisibility } from "../../session/protocol";
import type { TileDetails } from "../../state/tileDetails";

/** A colour of the nation table, as CSS takes it. */
export const cssColor = (color: number): string => `#${color.toString(16).padStart(6, "0")}`;

export const VISIBILITY_NAMES: Record<TileVisibility, string> = {
  [TileVisibility.Unexplored]: "Unexplored",
  [TileVisibility.Fogged]: "Out of sight",
  [TileVisibility.Visible]: "In sight",
};

/** Whose the tile is, from the viewer's side: theirs, someone's, or within someone's reach. */
export const ownershipText = (details: TileDetails, viewer: number): string | null => {
  if (details.owner) {
    return details.owner.nation === viewer ? "Your territory" : `Territory of ${details.owner.name}`;
  }
  if (details.sphere) {
    return details.sphere.nation === viewer ? "Within your sphere" : `Sphere of ${details.sphere.name}`;
  }
  return null;
};

/** The colour of the nation the tile's ownership line names, if any. */
export const ownershipColor = (details: TileDetails): string | null => {
  const nation = details.owner ?? details.sphere;
  return nation ? cssColor(nation.color) : null;
};
