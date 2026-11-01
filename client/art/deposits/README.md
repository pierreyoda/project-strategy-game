# Deposit art

Procedural sources of the frames in `raw-assets/deposits{m}{tps}/`: water, oil, metals and uranium as they lie on
the ground of a tile, at the three levels of richness the simulation reads a deposit at. Designed on the "Resources"
artboard of the UI-kit design canvas, which also shows the resource lens, where a deposit is drawn as its chip instead.

- `deposits.mjs` - the generator; run it from `client/` as `node art/deposits/deposits.mjs art/deposits/svg`.
- `svg/` - its output, `<resource>_<level>.svg`, levels 1 (trace) to 3 (rich).

The PNGs are rasterised with `rsvg-convert` (`brew install librsvg`) at the frame's native 1110x1280. There is one frame
per deposit and no mark frame: nobody owns the ground, so there is nothing to tint.
