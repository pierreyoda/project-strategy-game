# Deposit art

Procedural sources of the frames in `raw-assets/deposits{m}{tps}/`: water, oil, metals and uranium as they lie on
the ground of a tile, at the three levels of richness the simulation reads a deposit at. Designed on the "Resources"
artboard of the UI-kit design canvas, which also shows the resource lens, where a deposit is drawn as its chip instead.

- `deposits.mjs` - the generator; run it from `client/` as `node art/deposits/deposits.mjs art/deposits/svg`.
- `svg/` - its output: `<resource>_<level>.svg`, levels 1 (trace) to 3 (rich), and the resource lens's chips,
  `chip_<resource>_<level>.svg` and `chip_<resource>_<level>_depleting.svg`.

The PNGs are rasterised with `rsvg-convert` (`brew install librsvg`): the deposits at the frame's native 1110x1280, the
chips at twice their 256x112 frame, like every raw asset (see the generator's header for both commands). There is one
frame per deposit and no mark frame: nobody owns the ground, so there is nothing to tint.
