/**
 * Generates the building sprites (mine, solar plant; three levels each) in the settlements' grammar:
 * 1110x1280 frame, hex centre (555, 640), light from the upper left, iso slope ~0.53, top faces `st-hi`,
 * lower-left faces `st-lit`, lower-right faces `st-mid`, no outlines. The one nation-coloured surface per
 * level goes to a separate white `mark` frame, as for the settlements (see `domains/map/entities.ts`).
 *
 * Usage, from `client/`:
 *   node art/buildings/gen.mjs art/buildings/svg
 *   for f in art/buildings/svg/*_base.svg; do rsvg-convert -w 1110 -h 1280 "$f" -o "raw-assets/entities/buildings{m}{tps}/$(basename "${f%_base.svg}").png"; done
 *   (and the same for `*_mark.svg` -> `<name>_mark.png`)
 *
 * Emits, per building and level: `<name>_<level>_full.svg` (everything, nation colour included, for previews),
 * `_base.svg`, `_mark.svg`, and `<name>_<level>.frag.svg`, the drawing as a `<g id>` using the UI kit's CSS
 * variables, for the "Buildings" artboard of the design canvas.
 */
import { writeFileSync, mkdirSync } from "node:fs";

const SL = 0.53;
const CX = 555,
  CY = 640;

// ---------- palette modes ----------
const LIT = {
  "st-hi": "#bccdda",
  "st-lit": "#93a8b8",
  "st-mid": "#6b7e8e",
  "st-dark": "#3f4c58",
  "st-deep": "#2b353f",
  accent: "#3ec6e8",
  metals: "#a9b6c4",
  nation: "#e85d9a",
  black: "#000",
  wallA: "#273039",
  wallB: "#1f272e",
};
const col = (pal, k) =>
  pal.vars ? (k === "black" ? "#000" : k === "wallA" ? "#273039" : k === "wallB" ? "#1f272e" : `var(--${k})`) : pal[k];

// ---------- emitter ----------
class Draw {
  constructor(pal, mode) {
    this.pal = pal;
    this.mode = mode;
    this.out = [];
  }
  c(k) {
    return col(this.pal, k);
  }
  // mode: base = everything but nation; mark = only nation, white; full = all
  emit(s, nation = false) {
    if (this.mode === "base" && nation) return;
    if (this.mode === "mark" && !nation) return;
    this.out.push(s);
  }
  attr(o) {
    return Object.entries(o)
      .filter(([, v]) => v !== undefined && v !== null)
      .map(([k, v]) => `${k}="${v}"`)
      .join(" ");
  }
  path(d, o = {}, nation = false) {
    if (nation && this.mode === "mark")
      o = {
        ...o,
        fill: o.fill && o.fill !== "none" ? "#fff" : o.fill,
        stroke: o.stroke ? "#fff" : o.stroke,
        opacity: 1,
      };
    this.emit(`<path d="${d}" ${this.attr(o)}></path>`, nation);
  }
  ellipse(cx, cy, rx, ry, o = {}, nation = false) {
    if (nation && this.mode === "mark") o = { ...o, fill: "#fff", opacity: 1 };
    this.emit(`<ellipse cx="${r(cx)}" cy="${r(cy)}" rx="${r(rx)}" ry="${r(ry)}" ${this.attr(o)}></ellipse>`, nation);
  }
  circle(cx, cy, rad, o = {}, nation = false) {
    if (nation && this.mode === "mark") o = { ...o, fill: "#fff", opacity: 1 };
    this.emit(`<circle cx="${r(cx)}" cy="${r(cy)}" r="${r(rad)}" ${this.attr(o)}></circle>`, nation);
  }
  poly(pts, o = {}, nation = false) {
    this.path(pts.map((p, i) => `${i ? "L" : "M"}${r(p[0])} ${r(p[1])}`).join(" ") + " Z", o, nation);
  }
  line(a, b, o = {}, nation = false) {
    this.path(`M${r(a[0])} ${r(a[1])} L${r(b[0])} ${r(b[1])}`, o, nation);
  }
}
const r = (n) => Math.round(n * 10) / 10;

// iso: u runs lower-right, v runs lower-left, h up
const P = (u, v, h = 0) => [CX + u - v, CY + (u + v) * SL - h];

// ---------- primitives ----------
function box(d, u, v, w, dp, h, base = 0, opts = {}) {
  const c = d.c.bind(d);
  const A = P(u, v, base + h),
    B = P(u + w, v, base + h),
    C = P(u + w, v + dp, base + h),
    D = P(u, v + dp, base + h);
  const Db = P(u, v + dp, base),
    Cb = P(u + w, v + dp, base),
    Bb = P(u + w, v, base);
  d.poly([A, B, C, D], { fill: c(opts.top || "st-hi") });
  d.poly([D, C, Cb, Db], { fill: c(opts.left || "st-lit") });
  d.poly([C, B, Bb, Cb], { fill: c(opts.right || "st-mid") });
  // panel seams, like the settlement blocks
  if (opts.seams)
    for (let i = 1; i < opts.seams; i++) {
      const t = i / opts.seams;
      d.line(P(u, v + dp, base + h * t), P(u + w, v + dp, base + h * t), {
        stroke: c("st-deep"),
        "stroke-width": 3,
        opacity: 0.24,
      });
    }
  return { A, B, C, D };
}
// accent window strip on the lower-left face of a box
function stripL(d, u, v, w, dp, h0, h1, inset = 0.12, span = 0.76, o = {}) {
  const u0 = u + w * inset,
    u1 = u + w * (inset + span);
  d.poly(
    [P(u0, v + dp, h1), P(u1, v + dp, h1), P(u1, v + dp, h0), P(u0, v + dp, h0)],
    { fill: d.c(o.color || "accent"), opacity: o.opacity },
    o.nation,
  );
}
function stripR(d, u, v, w, dp, h0, h1, inset = 0.12, span = 0.76, o = {}) {
  const v0 = v + dp * inset,
    v1 = v + dp * (inset + span);
  d.poly(
    [P(u + w, v1, h1), P(u + w, v0, h1), P(u + w, v0, h0), P(u + w, v1, h0)],
    { fill: d.c(o.color || "accent"), opacity: o.opacity ?? 0.4 },
    o.nation,
  );
}
function cylinder(d, x, y, rx, h, opts = {}) {
  const c = d.c.bind(d),
    ry = rx * SL;
  const top = y - h;
  d.path(
    `M${r(x - rx)} ${r(top)} L${r(x - rx)} ${r(y)} A${r(rx)} ${r(ry)} 0 0 0 ${r(x + rx)} ${r(y)} L${r(x + rx)} ${r(top)} Z`,
    { fill: c(opts.body || "st-lit") },
  );
  const sx = x + rx * 0.3;
  d.path(
    `M${r(sx)} ${r(top + ry * 0.2)} L${r(sx)} ${r(y + ry * 0.2)} A${r(rx)} ${r(ry)} 0 0 0 ${r(x + rx)} ${r(y)} L${r(x + rx)} ${r(top)} Z`,
    { fill: c(opts.shade || "st-mid") },
  );
  d.ellipse(x, top, rx, ry, { fill: c(opts.top || "st-hi") });
  if (opts.bands)
    for (const t of opts.bands)
      d.line([x - rx, top + h * t], [x + rx, top + h * t], { stroke: c("st-deep"), "stroke-width": 7, opacity: 0.38 });
}
function shadow(d, x, y, rx, ry, o = 0.26) {
  d.ellipse(x, y, rx, ry, { fill: "#000", opacity: o });
}
function light(d, x, y, rad = 6, o = 0.8) {
  d.circle(x, y, rad, { fill: d.c("accent"), opacity: o });
}
function beacon(d, x, y, rad = 10) {
  d.circle(x, y, rad * 2.2, { fill: d.c("accent"), opacity: 0.22 });
  d.circle(x, y, rad, { fill: d.c("accent") });
}
// lights on the thirds of a ground diamond's near edges (and its back edges when asked)
function rimLights(d, cy, hw, back = false) {
  const [T, R, B, L] = diamond(CX, cy, hw);
  const at = (a, b, t) => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
  [at(L, B, 0.34), at(L, B, 0.67), at(B, R, 0.33), at(B, R, 0.66)].forEach(([x, y]) => light(d, x, y));
  if (back) [at(L, T, 0.5), at(T, R, 0.5)].forEach(([x, y]) => light(d, x, y, 6, 0.7));
}
function diamond(cx, cy, hw) {
  return [
    [cx, cy - hw * SL],
    [cx + hw, cy],
    [cx, cy + hw * SL],
    [cx - hw, cy],
  ];
}
// graded pad: translucent tint, retaining walls on the two near edges (town grammar)
function pad(d, hw, cy = CY, wall = 32, op = 0.42) {
  const c = d.c.bind(d);
  const [T, R, B, L] = diamond(CX, cy, hw);
  d.poly([T, R, B, L], { fill: c("st-deep"), opacity: op });
  d.line(T, B, { stroke: c("st-deep"), "stroke-width": 3, opacity: 0.18 });
  d.line(L, R, { stroke: c("st-deep"), "stroke-width": 3, opacity: 0.18 });
  d.poly([L, B, [B[0], B[1] + wall], [L[0], L[1] + wall]], { fill: c("wallA") });
  d.poly([B, R, [R[0], R[1] + wall], [B[0], B[1] + wall]], { fill: c("wallB") });
  d.line(L, B, { stroke: c("st-mid"), "stroke-width": 4, opacity: 0.45 });
}
function pennant(d, x, y, len = 76) {
  d.poly(
    [
      [x + 4, y],
      [x + len, y + 28],
      [x + 4, y + 56],
    ],
    { fill: d.c("nation") },
    true,
  );
  d.line([x + 4, y + 28], [x + len, y + 28], { stroke: "#000", "stroke-width": 3, opacity: 0.18 });
}
// lattice tower (derrick grammar): legs converge from base half-width wb to top half-width wt
function lattice(d, x, yb, yt, wb, wt, rungs = 6, legW = 13) {
  const c = d.c.bind(d);
  const H = yb - yt;
  const w = (t) => wb + (wt - wb) * t;
  // back legs, faint
  d.line([x, yt + 20], [x - wb * 0.9, yb + 12], { stroke: c("st-dark"), "stroke-width": 3, opacity: 0.42 });
  d.line([x, yt + 20], [x + wb * 0.9, yb + 12], { stroke: c("st-dark"), "stroke-width": 3, opacity: 0.42 });
  for (let i = 0; i < rungs - 1; i++) {
    const t0 = i / rungs,
      t1 = (i + 1) / rungs,
      y0 = yb - H * t0,
      y1 = yb - H * t1;
    d.line([x - w(t0), y0], [x + w(t1), y1], { stroke: c("st-mid"), "stroke-width": 5, opacity: 0.65 });
    d.line([x + w(t0), y0], [x - w(t1), y1], { stroke: c("st-mid"), "stroke-width": 5, opacity: 0.65 });
  }
  d.line([x - wb, yb], [x - wt, yt], { stroke: c("st-mid"), "stroke-width": legW, "stroke-linecap": "round" });
  d.line([x + wb, yb], [x + wt, yt], { stroke: c("st-mid"), "stroke-width": legW, "stroke-linecap": "round" });
  for (let i = 0; i < rungs; i++) {
    const t = i / rungs,
      y = yb - H * t;
    d.line([x - w(t), y], [x + w(t), y], { stroke: c("st-mid"), "stroke-width": 9 - i * 0.4 });
  }
  d.line([x, yt + 12], [x, yb - 16], { stroke: c("st-deep"), "stroke-width": 4, opacity: 0.55 });
}
// one solar panel: tilted toward the upper-left light; high edge at max u
function panel(d, u, v, w, dp, h0 = 10, tilt = 26) {
  const c = d.c.bind(d);
  const A = P(u, v, h0),
    B = P(u, v + dp, h0),
    C = P(u + w, v + dp, h0 + tilt),
    D = P(u + w, v, h0 + tilt);
  // contact shadow on the ground, cast down-right by the panel's height
  const sh = (h0 + tilt) * 0.45;
  d.poly(
    [
      P(u + sh, v + sh * 0.4, 0),
      P(u + sh, v + dp + sh * 0.4, 0),
      P(u + w + sh * 1.6, v + dp + sh * 0.4, 0),
      P(u + w + sh * 1.6, v + sh * 0.4, 0),
    ],
    { fill: "#000", opacity: 0.2 },
  );
  // posts and a ground rail under them
  const np = Math.max(1, Math.round(dp / 120));
  d.line(P(u + w * 0.85, v + 4, 0), P(u + w * 0.85, v + dp - 4, 0), {
    stroke: c("st-dark"),
    "stroke-width": 4,
    opacity: 0.7,
  });
  for (let k = 0; k < np; k++) {
    const vv = v + (dp * (k + 0.5)) / np;
    d.line(P(u + w * 0.85, vv, 0), P(u + w * 0.85, vv, h0 + tilt * 0.85), { stroke: c("st-dark"), "stroke-width": 7 });
    d.line(P(u + w * 0.15, vv, 0), P(u + w * 0.15, vv, h0 + tilt * 0.15), { stroke: c("st-dark"), "stroke-width": 6 });
  }
  d.poly([A, B, C, D], { fill: c("st-dark"), opacity: 0.9 });
  d.poly([A, B, C, D], { fill: c("accent"), opacity: 0.32 });
  d.poly([A, B, C, D], { fill: "none", stroke: c("st-lit"), "stroke-width": 5 });
  const n = Math.max(2, Math.round(dp / 44));
  for (let i = 1; i < n; i++) {
    const t = i / n;
    d.line(P(u, v + dp * t, h0), P(u + w, v + dp * t, h0 + tilt), {
      stroke: c("st-deep"),
      "stroke-width": 3,
      opacity: 0.45,
    });
  }
  d.line(P(u + w * 0.5, v, h0 + tilt * 0.5), P(u + w * 0.5, v + dp, h0 + tilt * 0.5), {
    stroke: c("st-deep"),
    "stroke-width": 3,
    opacity: 0.45,
  });
  // glint strip along the high edge
  d.line(D, C, { stroke: c("st-hi"), "stroke-width": 3, opacity: 0.5 });
}
// pylon / mast with cross arms
function mast(d, x, yb, yt, arms = [[36, 36]], o = {}) {
  const c = d.c.bind(d);
  d.line([x, yb], [x, yt], { stroke: c("st-mid"), "stroke-width": o.w || 8, "stroke-linecap": "round" });
  arms.forEach(([dy, hw], i) =>
    d.line([x - hw, yt + dy], [x + hw, yt + dy], {
      stroke: c("st-mid"),
      "stroke-width": 7 - i,
      "stroke-linecap": "round",
    }),
  );
}
function crates(d, u, v, n = 2, s = 26) {
  for (let i = 0; i < n; i++) {
    box(d, u + i * (s + 10), v + (i % 2) * 8, s, s, s * 0.85);
    const q = P(u + i * (s + 10) + s * 0.3, v + (i % 2) * 8 + s, s * 0.45);
    d.line(q, [q[0] + 14, q[1] + 7], { stroke: d.c("accent"), "stroke-width": 4, opacity: 0.8 });
  }
}

// ---------- MINE ----------
// open pit: nested terraces stepping down. hw = outer half width, steps = terraces
function pit(d, hw, steps, drop = 44, cy = CY + 60, id = "p") {
  const c = d.c.bind(d);
  const ring = (k) => hw * (1 - k * (0.78 / steps));
  // rim scrape + bench ring (the terrain still reads through)
  d.poly(diamond(CX, cy, hw * 1.22), { fill: c("st-deep"), opacity: 0.2 });
  d.poly(diamond(CX, cy, hw * 1.06), { fill: c("st-dark"), opacity: 0.32 });
  // everything below the rim is seen through the opening: clip to it
  const [T0, R0, B0, L0] = diamond(CX, cy, hw);
  d.emit(
    `<clipPath id="pit-${id}"><path d="M${r(T0[0])} ${r(T0[1])} L${r(R0[0])} ${r(R0[1])} L${r(B0[0])} ${r(B0[1])} L${r(L0[0])} ${r(L0[1])} Z"></path></clipPath><g clip-path="url(#pit-${id})">`,
  );
  for (let i = 0; i < steps; i++) {
    const hwO = ring(i),
      hwI = ring(i + 1),
      yO = cy + i * drop,
      yI = cy + (i + 1) * drop;
    // terrace floor at this depth
    d.poly(diamond(CX, yO, hwO), { fill: c("st-deep"), opacity: 0.55 + i * 0.12 });
    const [Ti, Ri, Bi, Li] = diamond(CX, yO, hwI);
    const [Td, Rd, Bd, Ld] = diamond(CX, yI, hwI);
    // far walls down to the next terrace: upper-left rim faces lower-right (shade), upper-right rim faces lower-left (lit)
    d.poly([Li, Ti, Td, Ld], { fill: c("st-dark") });
    d.poly([Ti, Ri, Rd, Td], { fill: c("st-mid") });
    d.poly([Ti, Ri, Rd, Td], { fill: c("st-lit"), opacity: 0.35 });
    // bench lip
    d.line(Li, Ti, { stroke: c("st-lit"), "stroke-width": 5, opacity: 0.55 });
    d.line(Ti, Ri, { stroke: c("st-hi"), "stroke-width": 5, opacity: 0.7 });
    d.line(Li, Bi, { stroke: c("st-mid"), "stroke-width": 4, opacity: 0.5 });
    d.line(Bi, Ri, { stroke: c("st-dark"), "stroke-width": 4, opacity: 0.6 });
    // haul ramp cut into the lower-left wall
    if (i === 0) {
      const a = [(Li[0] + Bi[0]) / 2 - 16, (Li[1] + Bi[1]) / 2 - 8],
        b = [(Ld[0] + Bd[0]) / 2 + 40, (Ld[1] + Bd[1]) / 2 + 20];
      d.line(a, b, { stroke: c("st-lit"), "stroke-width": 18, opacity: 0.7 });
      d.line(a, b, { stroke: c("st-deep"), "stroke-width": 3, opacity: 0.4, "stroke-dasharray": "10 12" });
    }
  }
  // pit floor, with the deposit showing
  const hwF = ring(steps),
    yF = cy + steps * drop;
  d.poly(diamond(CX, yF, hwF), { fill: c("st-deep"), opacity: 0.92 });
  d.ellipse(CX, yF, hwF * 0.66, hwF * 0.66 * SL, { fill: c("metals"), opacity: 0.4 });
  d.ellipse(CX - hwF * 0.15, yF + 4, hwF * 0.36, hwF * 0.36 * SL, { fill: c("metals"), opacity: 0.4 });
  light(d, CX - hwF * 0.5, yF - 6, 6, 0.8);
  light(d, CX + hwF * 0.45, yF + 8, 6, 0.7);
  d.emit(`</g>`);
  return { hwF, yF };
}
function headframe(d, x, yb, H, nation = false) {
  const c = d.c.bind(d);
  const yt = yb - H;
  lattice(d, x, yb, yt + 40, H * 0.2, H * 0.07, Math.max(4, Math.round(H / 70)));
  // head house
  const hw = H * 0.11;
  d.poly(
    [
      [x - hw, yt + 40],
      [x + hw, yt + 40],
      [x + hw * 0.8, yt + 4],
      [x - hw * 0.8, yt + 4],
    ],
    { fill: c("st-lit") },
  );
  d.poly(
    [
      [x - hw * 0.8, yt + 4],
      [x + hw * 0.8, yt + 4],
      [x + hw * 0.8 - 6, yt - 4],
      [x - hw * 0.8 + 6, yt - 4],
    ],
    { fill: c("st-hi") },
  );
  if (nation)
    d.poly(
      [
        [x - hw * 0.7, yt + 30],
        [x + hw * 0.7, yt + 30],
        [x + hw * 0.7, yt + 18],
        [x - hw * 0.7, yt + 18],
      ],
      { fill: c("nation") },
      true,
    );
  // sheave wheel
  d.circle(x + hw * 1.1, yt + 14, hw * 0.62, { fill: c("st-lit") });
  d.circle(x + hw * 1.1, yt + 14, hw * 0.36, { fill: c("st-mid") });
  d.circle(x + hw * 1.1, yt + 14, hw * 0.12, { fill: c("st-deep") });
  // cable down the shaft
  d.line([x + hw * 1.1, yt + 14 + hw * 0.62], [x + hw * 1.1, yb + 6], {
    stroke: c("st-deep"),
    "stroke-width": 3,
    opacity: 0.6,
  });
  beacon(d, x, yt - 14, 10);
}
function conveyor(d, a, b, w = 14) {
  const c = d.c.bind(d);
  d.line(a, b, { stroke: c("st-dark"), "stroke-width": w + 8, "stroke-linecap": "round", opacity: 0.9 });
  d.line(a, b, { stroke: c("st-lit"), "stroke-width": w, "stroke-linecap": "round" });
  d.line(a, b, { stroke: c("st-deep"), "stroke-width": 3, "stroke-dasharray": "14 18", opacity: 0.5 });
  // legs
  const n = 4;
  for (let i = 1; i < n; i++) {
    const t = i / n,
      x = a[0] + (b[0] - a[0]) * t,
      y = a[1] + (b[1] - a[1]) * t;
    d.line([x, y + w / 2], [x, y + 36 + (1 - t) * 10], { stroke: c("st-mid"), "stroke-width": 6 });
  }
}

function mine(d, level) {
  const c = d.c.bind(d);
  if (level === 1) {
    // scrape, drill rig, one cabin, spoil, crates
    d.ellipse(CX, 884, 250, 92, { fill: c("st-deep"), opacity: 0.26 });
    d.ellipse(CX + 40, 880, 160, 58, { fill: c("st-deep"), opacity: 0.16 });
    // survey track off the tile
    d.path(`M${CX + 150} 930 C 700 960 820 975 960 980`, {
      stroke: c("st-deep"),
      "stroke-width": 9,
      fill: "none",
      opacity: 0.28,
      "stroke-dasharray": "26 18",
    });
    // spoil heap
    d.ellipse(400, 902, 96, 30, { fill: "#000", opacity: 0.2 });
    d.path(`M304 900 Q 380 820 470 836 Q 520 860 496 904 Z`, { fill: c("st-dark") });
    d.path(`M330 896 Q 380 840 456 846 Q 490 870 478 900 Z`, { fill: c("metals"), opacity: 0.45 });
    // bore collar
    shadow(d, 640, 884, 150, 48, 0.26);
    d.ellipse(640, 876, 62, 24, { fill: c("st-dark"), opacity: 0.55 });
    d.ellipse(640, 876, 40, 15, { fill: c("st-deep") });
    d.ellipse(640, 876, 22, 8, { fill: c("accent"), opacity: 0.55 });
    // drill rig: short lattice tripod
    lattice(d, 640, 870, 560, 72, 22, 5, 11);
    d.poly(
      [
        [618, 560],
        [662, 560],
        [670, 528],
        [610, 528],
      ],
      { fill: c("st-lit") },
    );
    d.line([610, 528], [670, 528], { stroke: c("st-hi"), "stroke-width": 4 });
    d.line([640, 870], [640, 560], { stroke: c("st-deep"), "stroke-width": 4, opacity: 0.5 });
    pennant(d, 640, 470, 70);
    d.line([640, 528], [640, 462], { stroke: c("st-mid"), "stroke-width": 7, "stroke-linecap": "round" });
    beacon(d, 640, 452, 11);
    // cabin (the outpost's small pod, reused)
    box(d, -60, 110, 96, 72, 56, 0, { seams: 2 });
    stripL(d, -60, 110, 96, 72, 22, 38, 0.14, 0.5);
    stripR(d, -60, 110, 96, 72, 22, 36, 0.2, 0.5, { opacity: 0.4 });
    crates(d, 90, 150, 2, 26);
    light(d, 560, 905, 6, 0.8);
    light(d, 740, 900, 6, 0.7);
  }
  if (level === 2) {
    const { yF } = pit(d, 205, 2, 46, CY + 96, "m2");
    // conveyor climbing out of the pit to the crusher at the back-left
    conveyor(d, [CX - 40, yF - 6], [372, 666], 14);
    // crusher / hopper block with a nation band
    box(d, -250, -40, 120, 96, 110, 0, { seams: 3 });
    stripL(d, -250, -40, 120, 96, 60, 74, 0.14, 0.7, { color: "nation", nation: true, opacity: 1 });
    stripL(d, -250, -40, 120, 96, 30, 46, 0.14, 0.7);
    stripR(d, -250, -40, 120, 96, 30, 46, 0.16, 0.6, { opacity: 0.4 });
    // hopper on top
    box(d, -236, -26, 60, 60, 44, 110, { top: "st-lit" });
    d.ellipse(P(-206, 4, 154)[0], P(-206, 4, 154)[1], 22, 11, { fill: c("st-deep") });
    // stack
    cylinder(d, P(-150, -30, 110)[0], P(-150, -30, 110)[1], 11, 90, { top: "st-mid" });
    // silo
    cylinder(d, 300, 820, 46, 150, { bands: [0.3, 0.62] });
    // headframe over the pit's deep end
    headframe(d, 760, 720, 300, false);
    // cabin from level 1 survives, on the near-left rim
    box(d, 120, 270, 96, 72, 56, 0, { seams: 2 });
    stripL(d, 120, 270, 96, 72, 22, 38, 0.14, 0.5);
    // haul truck boxes at the ramp
    box(d, -150, 210, 34, 22, 20);
    box(d, -108, 214, 24, 22, 16, 0, { top: "metals" });
    // ground lights on the pad rims
    rimLights(d, CY + 96, 205 * 1.22, true);
  }
  if (level === 3) {
    const { yF } = pit(d, 250, 3, 44, CY + 90, "m3");
    // gantry conveyor crossing to the processing plant at the back-left
    conveyor(d, [CX - 30, yF - 8], [330, 612], 16);
    // processing plant: main block + annex + two stacks
    box(d, -300, -90, 150, 120, 150, 0, { seams: 4 });
    stripL(d, -300, -90, 150, 120, 96, 112, 0.12, 0.76, { color: "nation", nation: true, opacity: 1 });
    stripL(d, -300, -90, 150, 120, 62, 78, 0.12, 0.76);
    stripL(d, -300, -90, 150, 120, 30, 46, 0.12, 0.76, { opacity: 0.55 });
    stripR(d, -300, -90, 150, 120, 62, 78, 0.14, 0.62, { opacity: 0.36 });
    box(d, -300, 30, 150, 70, 70, 0, { seams: 2 });
    stripL(d, -300, 30, 150, 70, 26, 44, 0.12, 0.76, { opacity: 0.85 });
    const s1 = P(-240, -70, 150),
      s2 = P(-190, -70, 150);
    cylinder(d, s1[0], s1[1], 13, 110, { top: "st-mid" });
    cylinder(d, s2[0], s2[1], 13, 96, { top: "st-mid" });
    // hopper
    box(d, -200, -30, 70, 70, 50, 150, { top: "st-lit" });
    d.ellipse(P(-165, 5, 200)[0], P(-165, 5, 200)[1], 26, 13, { fill: c("st-deep") });
    // twin silos, right of plant
    cylinder(d, 262, 884, 50, 170, { bands: [0.28, 0.58] });
    cylinder(d, 346, 904, 40, 136, { bands: [0.4] });
    d.path(`M300 740 L420 740 L420 790`, { stroke: c("st-mid"), "stroke-width": 8, fill: "none" });
    // tall headframe over the shaft at the pit's right
    headframe(d, 790, 700, 440, true);
    // ore stockpile with glow
    d.ellipse(860, 930, 70, 26, { fill: "#000", opacity: 0.2 });
    d.path(`M792 926 Q 860 856 930 926 Z`, { fill: c("st-dark") });
    d.path(`M812 924 Q 860 876 910 924 Z`, { fill: c("metals"), opacity: 0.5 });
    // cabin row (l1 pod kept) and trucks
    box(d, 170, 300, 96, 72, 56, 0, { seams: 2 });
    stripL(d, 170, 300, 96, 72, 22, 38, 0.14, 0.5);
    box(d, 280, 300, 60, 50, 40);
    stripL(d, 280, 300, 60, 50, 14, 26, 0.14, 0.6);
    box(d, -190, 260, 34, 22, 20);
    box(d, -148, 264, 24, 22, 16, 0, { top: "metals" });
    box(d, -236, 254, 34, 22, 20);
    // lights
    rimLights(d, CY + 90, 250 * 1.22, true);
  }
}

// ---------- SOLAR ----------
function solar(d, level) {
  const c = d.c.bind(d);
  if (level === 1) {
    d.ellipse(CX - 40, 760, 330, 120, { fill: c("st-deep"), opacity: 0.24 });
    d.ellipse(CX + 110, 850, 150, 56, { fill: c("st-deep"), opacity: 0.16 });
    d.path(`M${CX + 130} 926 C 740 950 860 966 990 970`, {
      stroke: c("st-deep"),
      "stroke-width": 9,
      fill: "none",
      opacity: 0.28,
      "stroke-dasharray": "26 18",
    });
    // three panel rows along v, stepping in u
    for (let i = 0; i < 2; i++) for (let j = 0; j < 2; j++) panel(d, -130 + i * 130, -60 + j * 170, 92, 140, 4, 16);
    // cable trench to the inverter cabinet
    d.line(P(150, 100, 0), P(236, 100, 0), {
      stroke: c("st-deep"),
      "stroke-width": 5,
      opacity: 0.4,
      "stroke-dasharray": "10 10",
    });
    // inverter cabinet + mast with pennant
    box(d, 236, 60, 70, 80, 70, 0, { seams: 2 });
    stripL(d, 236, 60, 70, 80, 28, 44, 0.14, 0.6);
    stripR(d, 236, 60, 70, 80, 28, 44, 0.2, 0.5, { opacity: 0.4 });
    const m = P(236, 150, 0);
    mast(d, m[0] - 10, m[1], 470, [[66, 40]]);
    d.line([m[0] - 10, 560], [m[0] - 60, 800], { stroke: c("st-dark"), "stroke-width": 3, opacity: 0.45 });
    d.line([m[0] - 10, 560], [m[0] + 46, 800], { stroke: c("st-dark"), "stroke-width": 3, opacity: 0.45 });
    pennant(d, m[0] - 10, 476, 72);
    beacon(d, m[0] - 10, 462, 12);
    light(d, 430, 940);
    light(d, 700, 950);
  }
  if (level === 2) {
    pad(d, 390, CY + 40, 26, 0.32);
    // 3 x 2 field of large panels, rows running lower-left, a service lane between the two rows
    for (let i = 0; i < 3; i++) for (let j = 0; j < 2; j++) panel(d, -150 + i * 140, -120 + j * 190, 96, 150, 10, 16);
    d.line(P(-170, 50, 0), P(260, 50, 0), { stroke: c("st-mid"), "stroke-width": 6, opacity: 0.4 });
    d.line(P(-170, 42, 0), P(260, 42, 0), {
      stroke: c("st-lit"),
      "stroke-width": 3,
      opacity: 0.35,
      "stroke-dasharray": "18 22",
    });
    // battery bank at the back
    for (let k = 0; k < 4; k++) {
      const p = P(-160 + k * 42, -210, 0);
      cylinder(d, p[0], p[1], 18, 44, { bands: [0.5] });
    }
    // transformer / inverter block at the front, nation band
    box(d, 250, 110, 120, 110, 92, 0, { seams: 3 });
    stripL(d, 250, 110, 120, 110, 52, 66, 0.12, 0.74, { color: "nation", nation: true, opacity: 1 });
    stripL(d, 250, 110, 120, 110, 24, 40, 0.12, 0.74);
    stripR(d, 250, 110, 120, 110, 24, 40, 0.16, 0.6, { opacity: 0.4 });
    // pylon leaving the tile (town grammar)
    const py = P(250, -40, 0);
    mast(
      d,
      py[0],
      py[1],
      py[1] - 250,
      [
        [36, 36],
        [72, 30],
      ],
      { w: 8 },
    );
    d.path(
      `M${py[0]} ${py[1] - 232} C ${py[0] + 60} ${py[1] - 262} ${py[0] + 150} ${py[1] - 268} ${py[0] + 230} ${py[1] - 240}`,
      { stroke: c("st-deep"), "stroke-width": 4, fill: "none", opacity: 0.48 },
    );
    // control cabin from level 1
    box(d, 100, 270, 70, 80, 70, 0, { seams: 2 });
    stripL(d, 100, 270, 70, 80, 28, 44, 0.14, 0.6);
    rimLights(d, CY + 40, 390, true);
  }
  if (level === 3) {
    // the pad is the square u,v in [-148, 224]; the field fills it, the substation stands in front of it
    pad(d, 372, CY + 40, 32, 0.4);
    const rowU = [-130, -58, 14, 86, 158];
    rowU.forEach((u, i) => {
      panel(d, u, -128, 56, 142, 10, 16);
      panel(d, u, 62, 56, 143, 10, 16);
      if (i % 2 === 1) {
        box(d, u + 8, 18, 40, 40, 30);
        stripL(d, u + 8, 18, 40, 40, 10, 20, 0.16, 0.6);
      }
    });
    d.line(P(-140, 38, 0), P(214, 38, 0), { stroke: c("st-mid"), "stroke-width": 6, opacity: 0.35 });
    // substation: transformer (nation band), battery bank behind it, control cabin, transmission pylon
    for (let k = 0; k < 4; k++) {
      const p = P(236 + k * 36, -20, 0);
      cylinder(d, p[0], p[1], 16, 40, { bands: [0.5] });
    }
    box(d, 230, 50, 120, 110, 92, 0, { seams: 3 });
    stripL(d, 230, 50, 120, 110, 52, 66, 0.12, 0.74, { color: "nation", nation: true, opacity: 1 });
    stripL(d, 230, 50, 120, 110, 24, 40, 0.12, 0.74);
    stripR(d, 230, 50, 120, 110, 24, 40, 0.16, 0.6, { opacity: 0.4 });
    box(d, 230, 190, 70, 80, 70, 0, { seams: 2 });
    stripL(d, 230, 190, 70, 80, 28, 44, 0.14, 0.6);
    const py = P(318, -36, 0);
    lattice(d, py[0], py[1], py[1] - 360, 46, 14, 7, 10);
    d.line([py[0] - 70, py[1] - 300], [py[0] + 70, py[1] - 300], {
      stroke: c("st-mid"),
      "stroke-width": 8,
      "stroke-linecap": "round",
    });
    d.line([py[0] - 54, py[1] - 250], [py[0] + 54, py[1] - 250], {
      stroke: c("st-mid"),
      "stroke-width": 7,
      "stroke-linecap": "round",
    });
    d.path(
      `M${py[0] + 60} ${py[1] - 296} C ${py[0] + 140} ${py[1] - 320} ${py[0] + 220} ${py[1] - 322} ${py[0] + 300} ${py[1] - 300}`,
      { stroke: c("st-deep"), "stroke-width": 4, fill: "none", opacity: 0.48 },
    );
    d.path(
      `M${py[0] + 50} ${py[1] - 246} C ${py[0] + 130} ${py[1] - 270} ${py[0] + 220} ${py[1] - 272} ${py[0] + 300} ${py[1] - 250}`,
      { stroke: c("st-deep"), "stroke-width": 4, fill: "none", opacity: 0.48 },
    );
    beacon(d, py[0], py[1] - 372, 10);
    rimLights(d, CY + 40, 372, true);
  }
}

// ---------- output ----------
const OUT = process.argv[2] || "out";
mkdirSync(OUT, { recursive: true });
const builders = { mine, solar_plant: solar };
for (const [name, fn] of Object.entries(builders)) {
  for (const level of [1, 2, 3]) {
    for (const mode of ["full", "base", "mark"]) {
      const d = new Draw(LIT, mode);
      fn(d, level);
      writeFileSync(
        `${OUT}/${name}_${level}_${mode}.svg`,
        `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1110 1280" width="1110" height="1280">\n${d.out.join("\n")}\n</svg>\n`,
      );
    }
    // canvas flavour: css vars, as a <g id> symbol fragment
    const d = new Draw({ vars: true }, "full");
    fn(d, level);
    writeFileSync(`${OUT}/${name}_${level}.frag.svg`, `<g id="b-${name}-${level}">\n${d.out.join("\n")}</g>\n`);
  }
}
console.log("ok");
