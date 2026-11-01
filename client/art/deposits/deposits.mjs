/**
 * Generates the deposit art: water, oil, metals and uranium as they lie on the ground of a tile, at
 * the three levels of richness the simulation reads a deposit at (`resources::DEPOSIT_LEVELS`), in
 * the terrain tiles' own manner - organic outlines, light from the upper left, no keyline. Designed
 * on the "Resources" artboard of the UI-kit design canvas.
 *
 * 1110x1280 frames like every tile and entity, the art below the hex centre. One frame per deposit:
 * nobody owns the ground, so there is no mark frame to tint.
 *
 * Usage, from `client/`:
 *   node art/deposits/deposits.mjs art/deposits/svg
 *   for f in art/deposits/svg/*.svg; do rsvg-convert -w 1110 -h 1280 "$f" -o "raw-assets/deposits{m}{tps}/$(basename "${f%.svg}").png"; done
 */
import { mkdirSync, writeFileSync } from "node:fs";

const r = (n) => Math.round(n * 10) / 10;
const CX = 555,
  CY = 690;
/** The art is drawn at a base size, then grown: it is all a deposit shows of itself on the map. */
const GROW = 1.9;

// deterministic noise
const rng = (seed) => () => {
  seed |= 0;
  seed = (seed + 0x6d2b79f5) | 0;
  let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
  t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
};

let out = [];
const attrs = (o) =>
  Object.entries(o)
    .map(([k, v]) => `${k}="${typeof v === "number" ? r(v) : v}"`)
    .join(" ");
const path = (d, o) => out.push(`<path d="${d}" ${attrs(o)}/>`);
const circle = (cx, cy, rad, o) => out.push(`<circle ${attrs({ cx, cy, r: rad, ...o })}/>`);
const ellipse = (cx, cy, rx, ry, o) => out.push(`<ellipse ${attrs({ cx, cy, rx, ry, ...o })}/>`);

/** A closed organic outline: an ellipse whose radius wobbles, smoothed with Catmull-Rom. */
function blob(cx, cy, rx, ry, seed, wobble = 0.18, n = 9) {
  const rand = rng(seed);
  const phase = rand() * Math.PI * 2;
  const pts = Array.from({ length: n }, (_, i) => {
    const a = phase + (i / n) * Math.PI * 2;
    const k = 1 + (rand() * 2 - 1) * wobble;
    return [cx + Math.cos(a) * rx * k, cy + Math.sin(a) * ry * k];
  });
  let d = `M${r(pts[0][0])} ${r(pts[0][1])}`;
  for (let i = 0; i < n; i++) {
    const p0 = pts[(i - 1 + n) % n],
      p1 = pts[i],
      p2 = pts[(i + 1) % n],
      p3 = pts[(i + 2) % n];
    const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
    const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
    d += ` C${r(c1[0])} ${r(c1[1])} ${r(c2[0])} ${r(c2[1])} ${r(p2[0])} ${r(p2[1])}`;
  }
  return d + " Z";
}

/** An angular shard, seen from above: `k` vertices around a centre, jittered. */
function shard(cx, cy, rad, seed, k = 5) {
  const rand = rng(seed);
  const phase = rand() * Math.PI;
  return Array.from({ length: k }, (_, i) => {
    const a = phase + (i / k) * Math.PI * 2 + (rand() - 0.5) * 0.5;
    const q = rad * (0.7 + rand() * 0.45);
    return [cx + Math.cos(a) * q, cy + Math.sin(a) * q * 0.82];
  });
}

const poly = (pts) => pts.map((p, i) => `${i ? "L" : "M"}${r(p[0])} ${r(p[1])}`).join(" ") + " Z";

/** The icon's drop in the round, its foot at (x, y), `size` tall. */
function drop(x, y, size, lean = 0.2) {
  const k = size / 20;
  const body = "M12 22a7 7 0 0 0 7-7c0-2-1-3.9-3-5.5s-3.5-4-4-6.5c-.5 2.5-2 4.9-4 6.5C6 11.1 5 13 5 15a7 7 0 0 0 7 7z";
  const at = (tx, ty, sc = 1) => `translate(${r(x + tx)} ${r(y + ty)}) scale(${r(k * sc)}) translate(-12 -22)`;
  path(body, { fill: "#24609e", transform: at(0, 0) });
  path(body, { fill: "#3f8fe6", transform: at(-k * 0.6, -k * 0.6, 0.93) });
  path(body, { fill: "#6fbaf5", transform: at(-k * 1.8, -k * 1.6, 0.62) });
  ellipse(x - k * 3.4, y - k * 10, k * 1.3, k * 2.4, {
    fill: "#ffffff",
    opacity: 0.85,
    transform: `rotate(${r(25 + lean * 10)} ${r(x - k * 3.4)} ${r(y - k * 10)})`,
  });
  circle(x + k * 3.6, y - k * 4.5, k * 0.9, { fill: "#d6f1ff", opacity: 0.7 });
}

/** `drop` standing on the ground over a small puddle, its shadow falling to the lower right. */
function standingDrop(x, y, size) {
  const k = size / 20;
  ellipse(x + k * 4, y + k * 0.8, k * 9, k * 2.6, { fill: "#0e2a3e", opacity: 0.35 });
  ellipse(x, y, k * 7.5, k * 2.4, { fill: "#2a6aa6" });
  ellipse(x - k * 0.4, y - k * 0.3, k * 6.4, k * 1.8, { fill: "#5cc0e6" });
  drop(x, y - k * 0.4, size * 0.92);
}

function torbernite(level, glow) {
  const feats = LAYOUTS[level];
  feats.forEach(([dx, dy, s], i) => {
    path(blob(CX + dx, CY + dy, 135 * s, 100 * s, 501 + i, 0.22), { fill: "#141812", opacity: 0.26 });
    if (glow) {
      ellipse(CX + dx, CY + dy, 128 * s, 94 * s, { fill: "#cde34a", opacity: 0.1 });
      ellipse(CX + dx, CY + dy, 100 * s, 72 * s, { fill: "#cde34a", opacity: 0.1 });
    }
  });
  feats.forEach(([dx, dy, s], i) => {
    const x = CX + dx,
      y = CY + dy,
      rand = rng(511 + i);
    path(blob(x + 8 * s, y + 10 * s, 92 * s, 66 * s, 521 + i, 0.18), { fill: "#1e211d", opacity: 0.5 });
    path(blob(x, y, 90 * s, 64 * s, 521 + i, 0.18), { fill: "#6b6f69" });
    path(blob(x - 14 * s, y - 14 * s, 62 * s, 40 * s, 531 + i, 0.2), { fill: "#8b8f87" });
    // granite speckle
    for (let k = 0; k < 14; k++)
      circle(x + (rand() - 0.5) * 140 * s, y + (rand() - 0.5) * 90 * s, (2 + rand() * 2.5) * s, {
        fill: rand() < 0.5 ? "#4b4e48" : "#b3b5ad",
        opacity: 0.8,
      });
    if (glow) {
      // one dim seam across the rock's near half
      const d = `M${r(x - 66 * s)} ${r(y + 22 * s)} Q ${r(x - 16 * s)} ${r(y + 4 * s)} ${r(x + 10 * s)} ${r(y + 18 * s)} T ${r(x + 60 * s)} ${r(y + 14 * s)}`;
      path(d, { stroke: "#cde34a", "stroke-width": 12 * s, fill: "none", "stroke-linecap": "round", opacity: 0.28 });
      path(d, { stroke: "#bccf45", "stroke-width": 3.6 * s, fill: "none", "stroke-linecap": "round", opacity: 0.95 });
      path(d, { stroke: "#e9f59a", "stroke-width": 1.4 * s, fill: "none", "stroke-linecap": "round", opacity: 0.8 });
    }
    // the plates, a patch per rock and more on the rich one
    const patches = i === 0 ? level : 1;
    for (let p = 0; p < patches; p++) {
      const px = x + (rand() - 0.6) * 70 * s,
        py = y + (rand() - 0.7) * 40 * s;
      for (let k = 0; k < 7; k++) {
        const qx = px + (rand() - 0.5) * 46 * s,
          qy = py + (rand() - 0.5) * 26 * s,
          q = (7 + rand() * 6) * s,
          a = rand() * 90;
        out.push(
          `<g transform="translate(${r(qx)} ${r(qy)}) rotate(${r(a)}) scale(1 0.8)"><rect x="${r(-q)}" y="${r(-q)}" width="${r(2 * q)}" height="${r(2 * q)}" fill="#5f8a2c" stroke="#2f4214" stroke-width="${r(1.4 * s)}"/><rect x="${r(-q)}" y="${r(-q)}" width="${r(2 * q)}" height="${r(q * 0.7)}" fill="#a6c75a"/></g>`,
        );
      }
    }
  });
}

/** Where the features of a deposit go at each richness: offsets from the decal's centre, and a scale. */
const LAYOUTS = {
  1: [[0, 0, 0.95]],
  2: [
    [-70, -12, 1],
    [95, 42, 0.72],
  ],
  3: [
    [-85, -22, 1.08],
    [108, 28, 0.82],
    [5, 108, 0.62],
  ],
};

/** The same outline, shrunk about a point: for the sheen bands of a slick. */
const around = (x, y, k) => `translate(${r(x)} ${r(y)}) scale(${k}) translate(${r(-x)} ${r(-y)})`;

const RES = {
  water: {
    accent: "#4d9bf5",
    // droplets standing on the ground like the icon in the round, each over its own small puddle
    draw(level) {
      const spots = {
        1: [[0, 20, 1]],
        2: [
          [-60, 10, 1],
          [70, 40, 0.72],
        ],
        3: [
          [-70, 0, 1.05],
          [80, 25, 0.8],
          [0, 95, 0.62],
        ],
      }[level];
      spots.sort((p, q) => p[1] - q[1]).forEach(([dx, dy, s]) => standingDrop(CX + dx, CY + dy, 150 * s));
    },
  },
  oil: {
    accent: "#e08a4a",
    draw(level) {
      const feats = LAYOUTS[level];
      feats.forEach(([dx, dy, s], i) =>
        path(blob(CX + dx, CY + dy, 150 * s, 112 * s, 51 + i, 0.32, 13), { fill: "#1a120c", opacity: 0.32 }),
      );
      feats.forEach(([dx, dy, s], i) => {
        const x = CX + dx,
          y = CY + dy;
        const slick = blob(x, y, 98 * s, 74 * s, 61 + i, 0.3, 12);
        // fingers of the slick spreading over the ground
        const rand = rng(71 + i);
        for (let k = 0; k < 2 + level; k++) {
          const a = rand() * Math.PI * 2;
          path(
            blob(
              x + Math.cos(a) * 92 * s,
              y + Math.sin(a) * 70 * s,
              (24 + rand() * 10) * s,
              (14 + rand() * 6) * s,
              75 + k + i * 7,
              0.3,
              7,
            ),
            { fill: "#120d09" },
          );
        }
        path(slick, { fill: "#120d09", stroke: "#3d2c1f", "stroke-width": 6 * s });
        // the sheen: the slick's own outline, shrunk toward the light, in bands of colour like a film
        const [hx, hy] = [x - 12 * s, y - 10 * s];
        for (const [k, col, op, w, dash] of [
          [0.76, "#e08a4a", 0.75, 6, "70 34 24 40"],
          [0.5, "#e08a4a", 0.4, 4, "40 50 90 30"],
        ]) {
          path(slick, {
            fill: "none",
            stroke: col,
            "stroke-width": (w * s) / k,
            opacity: op,
            "stroke-linecap": "round",
            "stroke-dasharray": dash
              .split(" ")
              .map((n) => r((+n * s) / k))
              .join(" "),
            transform: around(hx, hy, k),
          });
        }
        // a gloss on the upper left, the one hard highlight a liquid this dark gets
        path(
          `M${r(x - 74 * s)} ${r(y - 6 * s)} C ${r(x - 70 * s)} ${r(y - 40 * s)} ${r(x - 40 * s)} ${r(y - 58 * s)} ${r(x - 8 * s)} ${r(y - 60 * s)}`,
          { stroke: "#ffffff", "stroke-width": 5 * s, fill: "none", "stroke-linecap": "round", opacity: 0.32 },
        );
        // crust of dried tar on the near rim
        for (let k = 0; k < 3; k++) {
          path(poly(shard(x + (40 + k * 22) * s, y + (52 - k * 10) * s, (9 + k * 2) * s, 81 + k + i * 5, 5)), {
            fill: "#3d2c1f",
          });
        }
      });
    },
  },
  metals: {
    accent: "#a9b6c4",
    draw(level) {
      const feats = LAYOUTS[level];
      feats.forEach(([dx, dy, s], i) =>
        path(blob(CX + dx, CY + dy, 130 * s, 96 * s, 91 + i, 0.22), { fill: "#1d2329", opacity: 0.22 }),
      );
      // ore gravel shed by the outcrops: small chips of the same rock, scattered on the ground
      const gr = rng(97);
      for (let k = 0; k < 6 + level * 6; k++) {
        const [dx, dy, sc] = feats[k % feats.length];
        const a = gr() * Math.PI * 2,
          d = (55 + gr() * 60) * sc;
        const gx = CX + dx + Math.cos(a) * d,
          gy = CY + dy + Math.sin(a) * d * 0.75,
          q = (6 + gr() * 8) * sc;
        path(poly(shard(gx, gy, q, 200 + k, 4)), { fill: gr() < 0.5 ? "#8d9aa8" : "#5b6874" });
      }
      const shards = { 1: 2, 2: 4, 3: 7 }[level];
      const rand = rng(101);
      const spots = [];
      feats.forEach(([dx, dy, s]) => {
        const per = Math.ceil(shards / feats.length);
        for (let k = 0; k < per && spots.length < shards; k++) {
          const a = rand() * Math.PI * 2,
            d = k === 0 ? 0 : 45 * s + rand() * 25 * s;
          spots.push([
            CX + dx + Math.cos(a) * d,
            CY + dy + Math.sin(a) * d * 0.8,
            (k === 0 ? 76 : 46 + rand() * 14) * s,
          ]);
        }
      });
      // back to front, so nearer rocks overlap farther ones
      spots
        .sort((p, q) => p[1] - q[1])
        .forEach(([x, y, rad], i) => {
          const pts = shard(x, y, rad, 111 + i, 5 + (i % 2));
          // rust bleeding into the ground at the foot
          path(blob(x + rad * 0.35, y + rad * 0.5, rad * 0.75, rad * 0.4, 121 + i, 0.3, 7), {
            fill: "#a85d34",
            opacity: 0.45,
          });
          path(poly(pts.map(([px, py]) => [px + rad * 0.12, py + rad * 0.16])), { fill: "#151a1f", opacity: 0.35 });
          path(poly(pts), { fill: "#4b5763" });
          // the faces turned to the light, a fan from the centre to the upper-left vertices
          const lit = pts.filter(([px, py]) => px - x + (py - y) < 0);
          if (lit.length >= 2) path(poly([[x, y], ...lit]), { fill: "#8d9aa8" });
          path(poly(pts.map(([px, py]) => [x + (px - x) * 0.42 - rad * 0.12, y + (py - y) * 0.42 - rad * 0.14])), {
            fill: "#c3ccd5",
          });
          path(`M${r(x - rad * 0.45)} ${r(y - rad * 0.05)} l ${r(rad * 0.3)} ${r(-rad * 0.22)}`, {
            stroke: "#f2f6f9",
            "stroke-width": Math.max(3, rad * 0.07),
            "stroke-linecap": "round",
            opacity: 0.9,
          });
        });
    },
  },
  uranium: {
    accent: "#cde34a",
    // torbernite with a faint halo and one dim seam per rock
    draw(level) {
      torbernite(level, true);
    },
  },
};

const OUT = process.argv[2] || "svg";
mkdirSync(OUT, { recursive: true });
const frame = (body) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1110 1280" width="1110" height="1280">\n${body}\n</svg>\n`;
for (const res of Object.keys(RES)) {
  for (const level of [1, 2, 3]) {
    out = [];
    RES[res].draw(level);
    out = [`<g transform="translate(${CX} ${CY}) scale(${GROW}) translate(${-CX} ${-CY})">`, ...out, "</g>"];
    writeFileSync(`${OUT}/${res}_${level}.svg`, frame(out.join("\n")));
  }
}
console.log("ok");
