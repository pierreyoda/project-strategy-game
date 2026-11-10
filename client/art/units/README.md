# Unit counters

Procedural sources of the sprites in `raw-assets/entities/units{m}{tps}/`: WW2 staff-map signs (infantry cross,
motorized cross on wheels, light tank track oval) on the kit's bevelled card, documented on the "Units" artboard of
the UI-kit design canvas.

- `counters.mjs` - the generator; run it from `client/` as `node art/units/counters.mjs art/units/svg`.
- `svg/` - its output: `*_full.svg` (the counter as it looks tinted, for previews), `*_base.svg` and `*_mark.svg`
  (the two frames the client loads).

The PNGs are rasterised with `rsvg-convert` (`brew install librsvg`) at the frame's native 1110x1280. The base frame
is the white card; the mark frame is the card in white with the border and the sign cut out, so that tinting it with
the nation's colour leaves them white. The companies are the renderer's, drawn as circles across the card's bottom
edge (`MapEntityRenderer.svelte`).
