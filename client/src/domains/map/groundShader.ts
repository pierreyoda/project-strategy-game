import { type BufferImageSource, Shader, type Texture } from "pixi.js";

import type { GroundTextures } from "./groundTextures";
import type { HexLayout } from "./hexLayout";
import type { HexMapData } from "./hexMap";
import type { MapLook } from "./mapLook";
import { TerrainKind, UNKNOWN_TERRAIN } from "./terrain";

/**
 * The ground under the hex map, blended between neighboring terrains by one fragment shader.
 *
 * A pointy-top hex grid is the Voronoi diagram of its centers: a pixel's hex is whichever center is
 * nearest. Softening that test - letting the second and third nearest centers weigh in within a
 * band of the border - blends the terrains without a single transition tile, and nudging the
 * distances with noise turns the straight hex sides into shorelines. The shader reads the terrain
 * of each tile from a data texture, which is the map's own `terrain` buffer uploaded as is.
 *
 * GLSL only, which is why `GameView` asks for WebGL: a WGSL twin is still to be written.
 */

/** The terrains the shader knows a ground texture for, in the order of its samplers. */
const GROUND_TERRAINS = [
  TerrainKind.DeepWater,
  TerrainKind.ShallowWater,
  TerrainKind.Flat,
  TerrainKind.Hills,
  TerrainKind.Mountains,
  TerrainKind.Ice,
  TerrainKind.RoughIce,
] as const;

/** The mesh is placed at the map's origin, so its positions are world positions as `hexLayout` counts them. */
const VERTEX = /* glsl */ `#version 300 es
  in vec2 aPosition;
  out vec2 vWorld;

  uniform mat3 uProjectionMatrix;
  uniform mat3 uWorldTransformMatrix;
  uniform mat3 uTransformMatrix;

  void main() {
    vWorld = aPosition;
    mat3 modelViewProjection = uProjectionMatrix * uWorldTransformMatrix * uTransformMatrix;
    gl_Position = vec4((modelViewProjection * vec3(aPosition, 1.0)).xy, 0.0, 1.0);
  }
`;

// the version line has to come first: PixiJS compiles a program as GLSL ES 1.00 without it
const FRAGMENT = /* glsl */ `#version 300 es
  precision highp float;
  precision highp int;

  in vec2 vWorld;
  out vec4 fragColor;

  /** The mesh's color and alpha, which is how the fade-in of the textured tiles reaches the ground. */
  uniform vec4 uColor;

  uniform sampler2D uTerrain;
  uniform sampler2D uGround0;
  uniform sampler2D uGround1;
  uniform sampler2D uGround2;
  uniform sampler2D uGround3;
  uniform sampler2D uGround4;
  uniform sampler2D uGround5;
  uniform sampler2D uGround6;

  uniform float uColumns;
  uniform float uRows;
  uniform float uSize;
  uniform float uWidth;
  uniform float uRowHeight;
  uniform float uPeriod;
  uniform float uBand;
  uniform float uWarp;
  /** Cells of warp noise along one period of the map: an integer, so the noise wraps with it. */
  uniform float uWarpCells;
  uniform float uGroundScale;

  const int GROUNDS = ${GROUND_TERRAINS.length};
  const int UNKNOWN = ${UNKNOWN_TERRAIN};
  const float SQRT3 = 1.7320508075688772;

  // "lowbias32", like the tile picks of the map: the same cell always wanders the same way
  float cellRandom(uvec2 cell, uint salt) {
    uint hash = cell.x * 0x27d4eb2du ^ cell.y * 0x165667b1u ^ salt;
    hash = (hash ^ (hash >> 16)) * 0x7feb352du;
    hash = (hash ^ (hash >> 15)) * 0x846ca68bu;
    return float(hash ^ (hash >> 16)) / 4294967296.0;
  }

  // value noise in [0, 1), periodic over uWarpCells along x
  float warpNoise(vec2 q, uint salt) {
    vec2 cell = floor(q);
    vec2 t = q - cell;
    t = t * t * (3.0 - 2.0 * t);
    float x0 = mod(cell.x, uWarpCells);
    float x1 = mod(cell.x + 1.0, uWarpCells);
    // rows of cells never wrap: the map does not, north to south
    uvec2 c00 = uvec2(uint(x0), uint(cell.y + 1024.0));
    uvec2 c10 = uvec2(uint(x1), uint(cell.y + 1024.0));
    uvec2 c01 = uvec2(uint(x0), uint(cell.y + 1025.0));
    uvec2 c11 = uvec2(uint(x1), uint(cell.y + 1025.0));
    float top = mix(cellRandom(c00, salt), cellRandom(c10, salt), t.x);
    float bottom = mix(cellRandom(c01, salt), cellRandom(c11, salt), t.x);
    return mix(top, bottom, t.y);
  }

  // like hexCenter in hexLayout.ts
  vec2 hexCenter(ivec2 hex) {
    float odd = float(hex.y & 1);
    return vec2(uWidth * (float(hex.x) + 0.5 * odd) + uWidth * 0.5, uRowHeight * float(hex.y) + uSize);
  }

  // like hexAt in hexLayout.ts
  ivec2 hexAt(vec2 p) {
    float px = p.x - uWidth * 0.5;
    float py = p.y - uSize;
    float q = ((SQRT3 / 3.0) * px - py / 3.0) / uSize;
    float r = ((2.0 / 3.0) * py) / uSize;
    float s = -q - r;
    float rq = floor(q + 0.5);
    float rr = floor(r + 0.5);
    float rs = floor(s + 0.5);
    float dq = abs(rq - q);
    float dr = abs(rr - r);
    float ds = abs(rs - s);
    if (dq > dr && dq > ds) {
      rq = -rr - rs;
    } else if (dr > ds) {
      rr = -rq - rs;
    }
    int row = int(rr);
    // >> floors, including for negative rows
    return ivec2(int(rq) + (row >> 1), row);
  }

  // the terrain of a tile, UNKNOWN beyond the poles; columns wrap around
  int terrainAt(ivec2 hex) {
    if (hex.y < 0 || hex.y >= int(uRows)) {
      return UNKNOWN;
    }
    int col = int(mod(float(hex.x), uColumns));
    return int(texelFetch(uTerrain, ivec2(col, hex.y), 0).r * 255.0 + 0.5);
  }

  vec4 groundAt(int ground, vec2 uv) {
    if (ground == 0) return texture(uGround0, uv);
    if (ground == 1) return texture(uGround1, uv);
    if (ground == 2) return texture(uGround2, uv);
    if (ground == 3) return texture(uGround3, uv);
    if (ground == 4) return texture(uGround4, uv);
    if (ground == 5) return texture(uGround5, uv);
    return texture(uGround6, uv);
  }

  void main() {
    // the edges wander, the ground under them does not
    vec2 q = vWorld * (uWarpCells / uPeriod);
    vec2 wander = vec2(warpNoise(q, 1u), warpNoise(q, 2u)) * 2.0 - 1.0;
    vec2 p = vWorld + uWarp * wander;

    ivec2 hex = hexAt(p);
    int odd = hex.y & 1;
    ivec2 around[7];
    around[0] = hex;
    around[1] = hex + ivec2(1, 0);
    around[2] = hex + ivec2(-1, 0);
    around[3] = hex + ivec2(odd, -1);
    around[4] = hex + ivec2(odd - 1, -1);
    around[5] = hex + ivec2(odd, 1);
    around[6] = hex + ivec2(odd - 1, 1);

    float distances[7];
    float nearest = 1e9;
    for (int i = 0; i < 7; i++) {
      distances[i] = distance(p, hexCenter(around[i]));
      nearest = min(nearest, distances[i]);
    }

    float weights[GROUNDS];
    for (int g = 0; g < GROUNDS; g++) {
      weights[g] = 0.0;
    }
    float total = 0.0;
    for (int i = 0; i < 7; i++) {
      int terrain = terrainAt(around[i]);
      if (terrain == UNKNOWN || terrain >= GROUNDS) {
        continue;
      }
      // the nearest center always weighs 1; the others fade out across the band past it
      float weight = uBand > 0.0 ? clamp(1.0 - (distances[i] - nearest) / uBand, 0.0, 1.0) : step(distances[i], nearest);
      weights[terrain] += weight;
      total += weight;
    }
    if (total <= 0.0) {
      fragColor = vec4(0.0);
      return;
    }

    vec2 uv = vWorld / uGroundScale;
    vec4 color = vec4(0.0);
    for (int g = 0; g < GROUNDS; g++) {
      if (weights[g] > 0.0) {
        color += groundAt(g, uv) * (weights[g] / total);
      }
    }
    fragColor = vec4(color.rgb, 1.0) * uColor;
  }
`;

/** What the shader needs to know of the map, the layout and the look. */
export type GroundUniforms = {
  uColumns: number;
  uRows: number;
  uSize: number;
  uWidth: number;
  uRowHeight: number;
  uPeriod: number;
  uBand: number;
  uWarp: number;
  uWarpCells: number;
  uGroundScale: number;
};

/**
 * The uniforms for this map and look.
 *
 * The warp noise is laid out in whole cells along one period of the map, so that the second copy of
 * the map wanders exactly like the first, the way the ground noise's period divides the map's.
 */
export const groundUniforms = (map: HexMapData, layout: HexLayout, period: number, look: MapLook): GroundUniforms => ({
  uColumns: map.columns,
  uRows: map.rows,
  uSize: layout.size,
  uWidth: layout.width,
  uRowHeight: layout.rowHeight,
  uPeriod: period,
  uBand: look.edgeBand,
  uWarp: look.edgeWarp,
  uWarpCells: Math.max(1, Math.round(period / look.edgeWarpScale)),
  uGroundScale: look.groundScale,
});

const uniformResources = (uniforms: GroundUniforms) =>
  Object.fromEntries(Object.entries(uniforms).map(([name, value]) => [name, { value, type: "f32" }]));

/** The shader, bound to the map's terrain and the terrains' ground textures. */
export const createGroundShader = (
  terrain: BufferImageSource,
  grounds: GroundTextures,
  uniforms: GroundUniforms,
): Shader => {
  const samplers = Object.fromEntries(
    GROUND_TERRAINS.map((kind, index): [string, Texture["source"]] => [`uGround${index}`, grounds[kind].source]),
  );
  return Shader.from({
    gl: { vertex: VERTEX, fragment: FRAGMENT, name: "hex-ground" },
    resources: {
      groundUniforms: uniformResources(uniforms),
      uTerrain: terrain,
      ...samplers,
    },
  });
};

/** Point the shader at a new look without rebuilding it. */
export const updateGroundUniforms = (shader: Shader, uniforms: GroundUniforms) => {
  const group = shader.resources.groundUniforms as { uniforms: Record<string, number> };
  Object.assign(group.uniforms, uniforms);
};
