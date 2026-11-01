//! The deposits in a terminal, as a view of the map's `TextRenderer`.
//!
//! One letter per kind - `w` water, `m` metals, `o` oil, `u` uranium - lowercase where the geology
//! put the deposit, uppercase where a guarantee or a top-up did; `@` on the capitals. The terrain
//! shows under them: `~` water, `.` plains, `:` hills, `^` mountains, `*` ice.

use project_x_core::{game::state::TileIndex, simulation::ids::EntityIndex};

use crate::{
    deposits::{DepositKind, GeneratedDeposits},
    map::{
        PlanetSurface,
        renderers::text::{TextCell, TextRenderer, render_grid},
    },
    nations::GeneratedNations,
};

use super::{Cell, kind_letter};

impl TextRenderer {
    /// Render the deposits over the surface they were generated on, with the nations' capitals if
    /// any, and a legend.
    pub fn render_deposits(
        &self,
        deposits: &GeneratedDeposits,
        surface: &PlanetSurface,
        nations: Option<&GeneratedNations>,
    ) -> String {
        let capitals: Vec<TileIndex> = nations
            .map(|nations| {
                nations
                    .nations()
                    .iter()
                    .filter_map(|nation| surface.index(nation.capital()))
                    .collect()
            })
            .unwrap_or_default();
        let mut text = render_grid(&surface.shape(), |coords| {
            let index = surface
                .index(coords.into())
                .expect("the grid stays within the map");
            let cell = Cell::of(deposits, surface, &capitals, index);
            TextCell {
                glyph: cell.glyph(),
                color: self.colors.then(|| cell.color()),
            }
        });
        let report = deposits.report();
        let counts: Vec<String> = DepositKind::ALL
            .iter()
            .map(|&kind| {
                format!(
                    "{} {:?} {}",
                    kind_letter(kind),
                    kind,
                    deposits.of_kind(kind).count()
                )
            })
            .collect();
        text.push_str(&format!(
            "{}; {} guaranteed for {} nations, {} top-ups\n",
            counts.join(", "),
            report.guarantees.laid,
            report.guarantees.nations_helped,
            report.top_ups,
        ));
        if let Some(nations) = nations {
            for nation in nations.nations() {
                let held = |kind: DepositKind| {
                    deposits
                        .of_kind(kind)
                        .filter(|d| nations.spheres().owner(d.tile()) == Some(nation.id()))
                        .count()
                };
                text.push_str(&format!(
                    "nation {}: sphere holds {}\n",
                    nation.id().index(),
                    DepositKind::ALL
                        .iter()
                        .map(|&kind| format!("{} {}", held(kind), kind_letter(kind)))
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        deposits::tests::{planet, settle},
        map::renderers::text::TextRenderer,
    };

    /// Displays the deposits of a generated planet. Run with:
    /// `cargo test -p project_x_procgen view_deposits --release -- --ignored --nocapture`
    /// (with `PROJECT_X_TEXT_COLORS=1` for colors).
    #[test]
    #[ignore]
    fn view_deposits() {
        let surface = planet(0, 80, 52);
        let (nations, deposits) = settle(&surface, 0, 8);
        println!(
            "{}",
            TextRenderer::from_env().render_deposits(&deposits, &surface, Some(&nations))
        );
    }

    #[test]
    fn test_text_renderer_draws_the_deposits_and_their_legend() {
        let surface = planet(3, 40, 26);
        let (nations, deposits) = settle(&surface, 3, 4);
        let text = TextRenderer::new(false).render_deposits(&deposits, &surface, Some(&nations));
        println!("{text}");
        assert_eq!(text.matches('@').count(), 4, "the capitals");
        assert!(text.contains('m') || text.contains('M'), "metals");
        assert!(text.contains("guaranteed for"));
        assert_eq!(text.matches("nation ").count(), 4);
    }
}
