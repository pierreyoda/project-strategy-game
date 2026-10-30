//! The nations as an image, as a view of the map's `ImageRenderer`, in the colors of the colored
//! `TextRenderer`: a nation's color over the land it owns, the same shaded over its sphere.

use image::RgbaImage;

use crate::{
    map::renderers::image::ImageRenderer,
    nations::{GeneratedNations, PlanetSurface},
};

use super::Cell;

impl ImageRenderer {
    /// Render the nations over the surface they were generated on.
    pub fn render_nations(&self, nations: &GeneratedNations, surface: &PlanetSurface) -> RgbaImage {
        self.render_grid(&surface.shape(), |coords| {
            let index = surface.index(coords.into())?;
            Some(Cell::of(nations, surface, index).color())
        })
    }
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use crate::{
        map::renderers::image::ImageRenderer,
        nations::{
            NationsGenerator, NationsGeneratorSettings,
            renderers::{WATER, nation_color},
            surface::tests::surface_from_art,
        },
    };

    #[test]
    fn test_image_renderer_paints_each_nation_in_its_color() {
        let surface = surface_from_art("~.....~|~.....~|~.....~");
        let settings = NationsGeneratorSettings::new(0, 2)
            .with_capital_spacing(Some(3))
            .with_site_jitter(1);
        let nations = NationsGenerator::from_settings(settings).generate(&surface);
        let image = ImageRenderer::new(8).render_nations(&nations, &surface);

        let colors: std::collections::HashSet<Rgba<u8>> = image.pixels().copied().collect();
        let [r, g, b] = WATER;
        assert!(colors.contains(&Rgba([r, g, b, 255])), "water");
        for nation in 0..2 {
            let [r, g, b] = nation_color(nation);
            assert!(colors.contains(&Rgba([r, g, b, 255])), "nation {nation}");
        }
        assert!(
            colors.contains(&Rgba([0, 0, 0, 0])),
            "the outside of the map"
        );
    }
}
