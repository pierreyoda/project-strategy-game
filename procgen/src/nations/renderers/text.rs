//! The nations in a terminal, as a view of the map's `TextRenderer`.
//!
//! One letter per nation: lowercase within its sphere of influence, uppercase where it owns the
//! land on turn 1, `@` on its capital. `~` is water, `*` ice, `.` land nobody reaches.

use project_x_core::{hex_map::coordinates::OffsetCoords, simulation::ids::EntityIndex};

use crate::{
    map::renderers::text::{TextCell, TextRenderer, render_grid},
    nations::{GeneratedNations, PlanetSurface},
};

use super::Cell;

impl TextRenderer {
    /// Render the nations over the surface they were generated on, with a legend.
    pub fn render_nations(&self, nations: &GeneratedNations, surface: &PlanetSurface) -> String {
        let shape = surface.shape();
        let mut text = render_grid(&shape, |coords| {
            let index = surface
                .index(coords.into())
                .expect("the grid stays within the map");
            let cell = Cell::of(nations, surface, index);
            TextCell {
                glyph: cell.glyph(),
                color: self.colors.then(|| cell.color()),
            }
        });
        for nation in nations.nations() {
            let capital = OffsetCoords::from(nation.capital());
            text.push_str(&format!(
                "{}: capital ({}, {}), owns {} hexes, sphere of {}, site {:.2}\n",
                Cell::letter(nation.id().index()).to_ascii_uppercase(),
                capital.col(),
                capital.row(),
                nation.territory().len(),
                nation.sphere().len(),
                nation.site_score(),
            ));
        }
        let report = nations.report();
        text.push_str(&format!(
            "spacing {} -> {} ({} relaxations), {} unplaced, {} land hexes unclaimed\n",
            report.siting.spacing_requested,
            report.siting.spacing_final,
            report.siting.relaxations,
            report.siting.unplaced,
            report.unclaimed_land,
        ));
        text
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        map::{MapGenerator, MapGeneratorSettings, renderers::text::TextRenderer},
        nations::{
            NationsGenerator, NationsGeneratorSettings, PlanetSurface,
            surface::tests::surface_from_art,
        },
    };

    /// Displays the nations of a generated planet. Run with:
    /// `cargo test -p project_x_procgen view_nations --release -- --ignored --nocapture`
    /// (with `PROJECT_X_TEXT_COLORS=1` for colors).
    #[test]
    #[ignore]
    fn view_nations() {
        let settings = MapGeneratorSettings::new(0, 80, 52);
        let map =
            MapGenerator::from_settings(MapGeneratorSettings::new(0, 80, 52)).generate(|_| {});
        let surface = PlanetSurface::classify(&map, &settings);
        let nations =
            NationsGenerator::from_settings(NationsGeneratorSettings::new(0, 8)).generate(&surface);
        println!(
            "{}",
            TextRenderer::from_env().render_nations(&nations, &surface)
        );
    }

    #[test]
    fn test_text_renderer_draws_the_nations_and_their_legend() {
        let surface = surface_from_art("~.....~|~.....~|~.....~|~~~~~~~|~~.~~~~");
        let settings = NationsGeneratorSettings::new(0, 2)
            .with_capital_spacing(Some(3))
            .with_border_radius(1)
            .with_site_jitter(1);
        let nations = NationsGenerator::from_settings(settings).generate(&surface);
        let text = TextRenderer::new(false).render_nations(&nations, &surface);
        println!("{text}");

        assert_eq!(text.matches('@').count(), 2, "one capital each");
        assert!(text.contains('A') && text.contains('B'), "owned land");
        assert!(text.contains("A: capital ("));
        assert!(text.contains("B: capital ("));
        assert!(
            text.contains("1 land hexes unclaimed"),
            "the islet to the south"
        );
        assert!(text.lines().last().unwrap().starts_with("spacing 3 -> 3"));
    }
}
