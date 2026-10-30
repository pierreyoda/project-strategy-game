# Unit counters

Procedural sources of the sprites in `raw-assets/entities/units{m}{tps}/`: NATO-style (APP-6 / MIL-STD-2525)
unit counters on the kit's bevelled card, documented on the "Units" artboard of the UI-kit design canvas.

- `counters.mjs` - the generator; run it from `client/` as `node art/units/counters.mjs art/units/svg`.
- `svg/` - its output: `*_full.svg` (the counter as it looks tinted, for previews), `*_base.svg` and `*_mark.svg`
  (the two frames the client loads).

The PNGs are rasterised with `rsvg-convert` (`brew install librsvg`) at the frame's native 1110x1280. The base frame
is the white card and the echelon tab; the mark frame is the card in white with the frame, the icon and the echelon
cut out, so that tinting it with the nation's colour leaves them white. The strength row is the renderer's, drawn in
the strip left free at the card's foot.
