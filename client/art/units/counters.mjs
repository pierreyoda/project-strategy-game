/**
 * Generates the unit counters: NATO-style (APP-6 / MIL-STD-2525) friendly frame and icon on a card with
 * the kit's bevelled corners, the echelon on a tab above. Glyphs as milsymbol draws them: infantry = X,
 * motorized infantry = X + vertical centre line (`S-G-UCIM--`), light armour = ellipse + "L" (`S-G-UCATL-`).
 *
 * Two frames per counter, like every entity (see `domains/map/entities.ts`): the base is the white card and
 * the dark echelon tab; the mark is the card again, in white with the frame, the icon and the echelon cut out,
 * so that tinting the mark with the nation's colour leaves them white. The strength row is not part of the
 * sprite: it changes every turn, and belongs to the renderer, in the strip left free at the card's foot.
 *
 * Usage, from `client/`:
 *   node art/units/counters.mjs art/units/svg
 *   for f in art/units/svg/*_base.svg; do rsvg-convert -w 1110 -h 1280 "$f" -o "raw-assets/entities/units{m}{tps}/$(basename "${f%_base.svg}").png"; done
 *   (and the same for `*_mark.svg` -> `<name>_mark.png`)
 */
import { mkdirSync, writeFileSync } from "node:fs";

const K = { ground: "#090e14", line: "#405670" };
const r = (n) => Math.round(n * 10) / 10;

let out = [];
const el = (tag, a) =>
  out.push(
    `<${tag} ${Object.entries(a)
      .map(([k, v]) => `${k}="${typeof v === "number" ? r(v) : v}"`)
      .join(" ")}/>`,
  );
const stroke = (d, color, w, extra = {}) =>
  el("path", {
    d,
    stroke: color,
    "stroke-width": w,
    fill: "none",
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    ...extra,
  });
/** A card with bevelled top-left and bottom-right corners, the kit's panel shape. */
const card = (x, y, w, h, c) =>
  `M${x + c} ${y} L${x + w} ${y} L${x + w} ${y + h - c} L${x + w - c} ${y + h} L${x} ${y + h} L${x} ${y + c} Z`;

const ICONS = {
  infantry: (cx, cy, hw, hh, ink, w) =>
    stroke(`M${cx - hw} ${cy - hh} L${cx + hw} ${cy + hh} M${cx - hw} ${cy + hh} L${cx + hw} ${cy - hh}`, ink, w),
  motorized: (cx, cy, hw, hh, ink, w) =>
    stroke(
      `M${cx - hw} ${cy - hh} L${cx + hw} ${cy + hh} M${cx - hw} ${cy + hh} L${cx + hw} ${cy - hh} M${cx} ${cy - hh - 14} L${cx} ${cy + hh + 14}`,
      ink,
      w,
    ),
  light_tank: (cx, cy, hw, hh, ink, w) => {
    el("ellipse", { cx, cy: cy - 18, rx: hw * 0.78, ry: hh * 0.56, fill: "none", stroke: ink, "stroke-width": w });
    // the "L" size modifier, as strokes rather than a font
    stroke(`M${cx - 18} ${cy + hh - 46} L${cx - 18} ${cy + hh + 4} L${cx + 22} ${cy + hh + 4}`, ink, w * 0.75);
  },
};

/** Echelon marks above the frame: one X per step, division = XX. */
const ECHELONS = { brigade: 1, division: 2, corps: 3 };

const GEOM = (cx, cy) => {
  const w = 640,
    h = 426,
    x = cx - w / 2,
    y = cy - h / 2;
  return { w, h, x, y, c: 40, inset: 22 };
};

/**
 * Draw one counter. `layer`: "full" draws it as it looks tinted; "base" the white card and the tab;
 * "mark" the white card with the ink cut out, through a mask.
 */
function counter(cx, cy, kind, echelon, layer, nation = "#ffffff") {
  const { w, h, x, y, c, inset } = GEOM(cx, cy);
  const steps = ECHELONS[echelon];
  const tw = 60 + steps * 48,
    th = 58,
    tx = cx - tw / 2,
    ty = y - th + 10;
  const echelonPath = Array.from({ length: steps }, (_, i) => {
    const ex = tx + 30 + i * 48 + 20;
    return `M${ex - 20} ${ty + 14} L${ex + 20} ${ty + 40} M${ex - 20} ${ty + 40} L${ex + 20} ${ty + 14}`;
  }).join(" ");

  if (layer !== "mark") {
    el("path", { d: card(x + 10, y + 14, w, h, c), fill: "#000", opacity: 0.32 });
    el("path", { d: card(x, y, w, h, c), fill: "#ffffff" });
    el("path", { d: card(tx, ty, tw, th, 14), fill: K.ground, opacity: 0.92 });
    el("path", { d: card(tx, ty, tw, th, 14), fill: "none", stroke: K.line, "stroke-width": 2 });
    stroke(`M${tx + 14} ${ty + 2} L${tx + tw - 2} ${ty + 2}`, "#fff", 2, { opacity: 0.12 });
  }
  // the ink: inner frame, icon, the echelon on its tab, and a lit top edge
  const ink = (color, wrap) => {
    if (wrap) out.push(wrap);
    el("path", {
      d: card(x + inset, y + inset, w - 2 * inset, h - 2 * inset, c * 0.55),
      fill: "none",
      stroke: color,
      "stroke-width": 7,
    });
    ICONS[kind](cx, cy - 10, 220, 128, color, 15);
    stroke(`M${x + c} ${y + 3} L${x + w - 3} ${y + 3}`, color, 4, { opacity: 0.35 });
    if (wrap) out.push("</g>");
  };
  if (layer === "full") {
    out.push(`<g>`);
    el("path", { d: card(x, y, w, h, c), fill: nation });
    out.push("</g>");
    ink("#ffffff");
    stroke(echelonPath, nation, 8);
  } else if (layer === "mark") {
    const id = `cut-${kind}`;
    out.push(`<mask id="${id}" maskUnits="userSpaceOnUse" x="0" y="0" width="1110" height="1280">`);
    el("path", { d: card(x, y, w, h, c), fill: "#fff" });
    ink("#000");
    out.push("</mask>");
    el("path", { d: card(x, y, w, h, c), fill: "#ffffff", mask: `url(#${id})` });
    stroke(echelonPath, "#ffffff", 8);
  }
}

const frame = (body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1110 1280" width="1110" height="1280">\n${body}\n</svg>\n`;
const OUT = process.argv[2] || "svg";
// the "full" layer's nation colour: a CSS variable when the output feeds the design canvas
const NATION = process.argv[3] || "#4f9ddb";
mkdirSync(OUT, { recursive: true });
for (const kind of ["infantry", "motorized", "light_tank"]) {
  const name = `${kind}_division`;
  for (const layer of ["full", "base", "mark"]) {
    out = [];
    counter(555, 640, kind, "division", layer, NATION);
    writeFileSync(`${OUT}/${name}_${layer}.svg`, frame(out.join("\n")));
  }
}
console.log("ok");
