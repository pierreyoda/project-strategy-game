/**
 * Generates the unit counters: a WW2 staff-map sign on the kit's bevelled card, the card itself the
 * unit's frame - a white border inset from its edge, the sign inside it at one stroke weight and
 * clipped to it. Infantry = the frame's diagonals, motorized = the same cross on two wheels, light
 * tank = the track oval. No echelon mark: every counter is a division.
 *
 * Two frames per counter, like every entity (see `domains/map/entities.ts`): the base is the white
 * card and its shadow; the mark is the card again, in white with the border and the sign cut out, so
 * that tinting the mark with the nation's colour leaves them white. The companies are not part of the
 * sprite: they change every turn, and belong to the renderer, which draws them as circles across the
 * card's bottom edge (`COMPANY_ROW` in `MapEntityRenderer.svelte`, at y 835 in the frame).
 *
 * Usage, from `client/`:
 *   node art/units/counters.mjs art/units/svg
 *   for f in art/units/svg/*_base.svg; do rsvg-convert -w 1110 -h 1280 "$f" -o "raw-assets/entities/units{m}{tps}/$(basename "${f%_base.svg}").png"; done
 *   (and the same for `*_mark.svg` -> `<name>_mark.png`)
 */
import { mkdirSync, writeFileSync } from "node:fs";

const r = (n) => Math.round(n * 10) / 10;

/** A card with bevelled top-left and bottom-right corners, the kit's panel shape. */
const card = (x, y, w, h, c) =>
  `M${r(x + c)} ${r(y)} L${r(x + w)} ${r(y)} L${r(x + w)} ${r(y + h - c)} L${r(x + w - c)} ${r(y + h)} L${r(x)} ${r(y + h)} L${r(x)} ${r(y + c)} Z`;

/**
 * The card in the 1110x1280 frame: 3:2, centred a little above the hex centre so that the companies
 * across its bottom edge stay within the hex.
 */
const CARD = { w: 640, h: 426, c: 40, cx: 555, cy: 622 };
/** The white border, inset from the card's edge. */
const BORDER = { inset: 16, width: 9 };
/** One stroke weight for every sign. */
const SIGN = 21;

const x = CARD.cx - CARD.w / 2;
const y = CARD.cy - CARD.h / 2;
/** The border's box, which the signs run to and are clipped by. */
const inner = {
  x0: x + BORDER.inset,
  y0: y + BORDER.inset,
  x1: x + CARD.w - BORDER.inset,
  y1: y + CARD.h - BORDER.inset,
};
const innerPath = card(inner.x0, inner.y0, inner.x1 - inner.x0, inner.y1 - inner.y0, CARD.c * 0.6);

const cross = (ink) =>
  `<path d="M${inner.x0} ${inner.y0} L${inner.x1} ${inner.y1} M${inner.x0} ${inner.y1} L${inner.x1} ${inner.y0}" stroke="${ink}" stroke-width="${SIGN}" fill="none"/>`;
const SIGNS = {
  infantry: cross,
  // infantry on wheels: hollow, without the companies' discs, so that they are never read as one
  motorized: (ink) =>
    cross(ink) +
    [-1, 1]
      .map(
        (side) =>
          `<circle cx="${r(CARD.cx + side * 58)}" cy="${r(inner.y1 - 92)}" r="27" fill="none" stroke="${ink}" stroke-width="${SIGN - 5}"/>`,
      )
      .join(""),
  // armour: its track, a flat oval
  light_tank: (ink) =>
    `<rect x="${r(CARD.cx - 196)}" y="${r((inner.y0 + inner.y1) / 2 - 110)}" width="392" height="192" rx="96" fill="none" stroke="${ink}" stroke-width="${SIGN}"/>`,
};

/** The ink of a counter: its border and its sign, clipped to the border. */
const ink = (kind, color) =>
  `<clipPath id="inside-${kind}"><path d="${innerPath}"/></clipPath>` +
  `<g clip-path="url(#inside-${kind})">${SIGNS[kind](color)}</g>` +
  `<path d="${innerPath}" fill="none" stroke="${color}" stroke-width="${BORDER.width}"/>`;
/** The lit top edge, as on every raised surface of the HUD. */
const litEdge = (color, opacity) =>
  `<path d="M${x + CARD.c} ${y + 2.5} L${x + CARD.w - 3} ${y + 2.5}" stroke="${color}" stroke-width="4" fill="none" opacity="${opacity}"/>`;

/**
 * Draw one counter. `layer`: "full" draws it as it looks tinted; "base" the white card and its
 * shadow; "mark" the white card with the ink cut out, through a mask.
 */
function counter(kind, layer, nation) {
  const cardPath = card(x, y, CARD.w, CARD.h, CARD.c);
  if (layer === "base") {
    return `<path d="${card(x + 10, y + 14, CARD.w, CARD.h, CARD.c)}" fill="#000" opacity="0.32"/><path d="${cardPath}" fill="#ffffff"/>`;
  }
  if (layer === "mark") {
    return (
      `<mask id="cut-${kind}" maskUnits="userSpaceOnUse" x="0" y="0" width="1110" height="1280">` +
      `<path d="${cardPath}" fill="#fff"/>${ink(kind, "#000")}${litEdge("#000", 0.35)}</mask>` +
      `<path d="${cardPath}" fill="#ffffff" mask="url(#cut-${kind})"/>`
    );
  }
  return (
    `<path d="${card(x + 10, y + 14, CARD.w, CARD.h, CARD.c)}" fill="#000" opacity="0.32"/>` +
    `<path d="${cardPath}" fill="${nation}"/>${ink(kind, "#ffffff")}${litEdge("#ffffff", 0.35)}`
  );
}

const frame = (body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1110 1280" width="1110" height="1280">\n${body}\n</svg>\n`;
const OUT = process.argv[2] || "svg";
// the "full" layer's nation colour: a CSS variable when the output feeds the design canvas
const NATION = process.argv[3] || "#4f9ddb";
mkdirSync(OUT, { recursive: true });
for (const kind of ["infantry", "motorized", "light_tank"]) {
  for (const layer of ["full", "base", "mark"]) {
    writeFileSync(`${OUT}/${kind}_division_${layer}.svg`, frame(counter(kind, layer, NATION)));
  }
}
console.log("ok");
