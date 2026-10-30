# Building sprites

Procedural sources of the sprites in `raw-assets/entities/buildings{m}{tps}/`, drawn in the same grammar as the
settlement sprites documented on the "Settlements" and "Buildings" artboards of the UI-kit design canvas.

- `gen.mjs` - the generator; run it from `client/` as `node art/buildings/gen.mjs art/buildings/svg`.
- `svg/` - its output: `*_full.svg` (preview with nation colour), `*_base.svg` and `*_mark.svg` (the two frames
  the client loads), `*.frag.svg` (the drawing as an SVG fragment on the kit's CSS variables, for the canvas).

The PNGs are rasterised with `rsvg-convert` (`brew install librsvg`) at the frame's native 1110x1280; the base frame
holds everything but the nation colour, the mark frame only the nation-coloured surface, in white, so the renderer
can tint it.
