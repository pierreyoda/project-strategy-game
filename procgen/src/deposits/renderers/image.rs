//! The deposits as an image, as a view of the map's `ImageRenderer`, in the colors of the colored
//! `TextRenderer`.

use image::RgbaImage;
use project_x_core::game::state::TileIndex;

use crate::{
    deposits::GeneratedDeposits,
    map::{PlanetSurface, renderers::image::ImageRenderer},
    nations::GeneratedNations,
};

use super::Cell;

impl ImageRenderer {
    /// Render the deposits over the surface they were generated on, with the nations' capitals if
    /// any.
    pub fn render_deposits(
        &self,
        deposits: &GeneratedDeposits,
        surface: &PlanetSurface,
        nations: Option<&GeneratedNations>,
    ) -> RgbaImage {
        let capitals: Vec<TileIndex> = nations
            .map(|nations| {
                nations
                    .nations()
                    .iter()
                    .filter_map(|nation| surface.index(nation.capital()))
                    .collect()
            })
            .unwrap_or_default();
        self.render_grid(&surface.shape(), |coords| {
            let index = surface.index(coords.into())?;
            Some(Cell::of(deposits, surface, &capitals, index).color())
        })
    }
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use crate::{
        deposits::{
            DepositKind,
            renderers::kind_color,
            tests::{planet, settle},
        },
        map::renderers::image::ImageRenderer,
    };

    #[test]
    fn test_image_renderer_paints_the_deposits_in_their_colors() {
        let surface = planet(3, 40, 26);
        let (nations, deposits) = settle(&surface, 3, 4);
        let image = ImageRenderer::new(8).render_deposits(&deposits, &surface, Some(&nations));
        let colors: std::collections::HashSet<Rgba<u8>> = image.pixels().copied().collect();
        for kind in [DepositKind::Water, DepositKind::Metals] {
            let [r, g, b] = kind_color(kind);
            assert!(colors.contains(&Rgba([r, g, b, 255])), "{kind:?}");
        }
    }
}
