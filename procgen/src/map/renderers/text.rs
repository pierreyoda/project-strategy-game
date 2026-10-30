//! Text renderer, to display maps in a terminal, for instance from unit tests
//! (with `cargo test -- --nocapture`).
//!
//! Pointy-top hexes are drawn 2 characters wide, odd rows being shoved one character
//! to the east. Row 0 is the north pole.

use project_x_core::hex_map::{HexMap, coordinates::OffsetCoords, shape::RectangularHexMapShape};

use super::{TerrainLegend, ValueRange};
use crate::map::{ElevationField, MapGeneratorSettings};

/// Environment variable enabling colors in `TextRenderer::from_env`, when set to any value.
pub const TEXT_RENDERER_COLORS_ENV_VAR: &str = "PROJECT_X_TEXT_COLORS";

/// Glyphs of `TextRenderer::render_field`, from the minimum value to the maximum one.
const RAMP_GLYPHS: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// Glyphs of `TextRenderer::render_hex_map`, for each band of the `TerrainLegend`.
const TERRAIN_GLYPHS: [char; TerrainLegend::BANDS] = ['~', '-', '.', ':', 'n', '^', 'A'];

/// Renders maps as text, optionally with colors (24-bit ANSI escape codes).
#[derive(Copy, Clone, Debug, Default)]
pub struct TextRenderer {
    pub(crate) colors: bool,
}

impl TextRenderer {
    pub fn new(colors: bool) -> Self {
        Self { colors }
    }

    /// With colors when the `TEXT_RENDERER_COLORS_ENV_VAR` environment variable is set.
    pub fn from_env() -> Self {
        Self::new(std::env::var_os(TEXT_RENDERER_COLORS_ENV_VAR).is_some())
    }

    /// Render the elevations of a map generated with the given settings, with glyphs
    /// (and colors) following noise's terrain gradient.
    pub fn render_hex_map(&self, map: &HexMap, settings: &MapGeneratorSettings) -> String {
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let legend = TerrainLegend::new(settings);
        let mut text = render_grid(&shape, |coords| match map.tile(&coords.into()) {
            Some(tile) => TextCell {
                glyph: TERRAIN_GLYPHS[legend.band(tile.elevation())],
                color: self.colors.then(|| legend.color(tile.elevation())),
            },
            None => TextCell {
                glyph: '?',
                color: None,
            },
        });
        text.push_str(&describe_terrain_legend(&legend));
        text.push('\n');
        text
    }

    /// Render raw noise values, spread over a ramp of glyphs (and grays) between
    /// their minimum and their maximum.
    pub fn render_field(&self, field: &ElevationField) -> String {
        let shape = field.shape();
        let columns = shape.columns() as usize;
        let values = field.values();
        let range = ValueRange::of(values);

        let ramp: String = RAMP_GLYPHS.iter().collect();
        let mut text = format!("min {:+.4} {ramp:?} max {:+.4}\n", range.min, range.max);
        text.push_str(&render_grid(&shape, |coords| {
            let value = values[coords.row() as usize * columns + coords.col() as usize];
            let index = ((range.ratio(value) * RAMP_GLYPHS.len() as f64) as usize)
                .min(RAMP_GLYPHS.len() - 1);
            TextCell {
                glyph: RAMP_GLYPHS[index],
                color: self.colors.then(|| range.gray(value)),
            }
        }));
        text
    }
}

/// One line describing the terrain glyphs, like `~ < -128 m  - < 0 m  [...]  A >= 3072 m`.
fn describe_terrain_legend(legend: &TerrainLegend) -> String {
    let bounds = legend.bounds();
    let mut parts: Vec<String> = TERRAIN_GLYPHS
        .iter()
        .zip(bounds)
        .map(|(glyph, bound)| format!("{glyph} < {bound:.0} m"))
        .collect();
    parts.push(format!(
        "{} >= {:.0} m",
        TERRAIN_GLYPHS[bounds.len()],
        bounds[bounds.len() - 1]
    ));
    parts.join("  ")
}

/// Draw a grid of cells, with a border and the poles marked, like `render_hex_map`.
pub(crate) fn render_grid(
    shape: &RectangularHexMapShape,
    mut cell: impl FnMut(OffsetCoords) -> TextCell,
) -> String {
    let border = format!("+{}+", "-".repeat(2 * shape.columns() as usize + 2));
    let mut text = format!("{border}  N\n");
    for row in 0..shape.rows() {
        let odd = row & 1 == 1;
        text.push_str(if odd { "|  " } else { "| " });
        for col in 0..shape.columns() {
            cell(OffsetCoords::new(col, row)).push_to(&mut text);
        }
        text.push_str(if odd { "|\n" } else { " |\n" });
    }
    text.push_str(&format!("{border}  S\n"));
    text
}

/// One hex of a rendered map: a glyph followed by a space, on an optional background color.
pub(crate) struct TextCell {
    pub(crate) glyph: char,
    pub(crate) color: Option<[u8; 3]>,
}

impl TextCell {
    fn push_to(&self, text: &mut String) {
        match self.color {
            None => {
                text.push(self.glyph);
                text.push(' ');
            }
            Some([r, g, b]) => {
                // black or white glyph, whichever contrasts the most with the background
                let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
                let fg = if luminance > 128.0 { 0 } else { 255 };
                text.push_str(&format!(
                    "\x1b[48;2;{r};{g};{b}m\x1b[38;2;{fg};{fg};{fg}m{} \x1b[0m",
                    self.glyph
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use noise::NoiseFn;
    use project_x_core::hex_map::{HexMap, shape::RectangularHexMapShape, tile::HexMapTile};

    use crate::map::{ElevationField, MapGeneratorSettings, PlanetGraph, SubGroupId};

    use super::TextRenderer;

    /// Outputs the height of the sampled point on the cylinder.
    struct Height;

    impl NoiseFn<f64, 3> for Height {
        fn get(&self, point: [f64; 3]) -> f64 {
            point[1]
        }
    }

    /// Map whose tiles have the given elevations, in the order of `RectangularHexMapShape::coords`.
    fn build_hex_map(shape: &RectangularHexMapShape, elevations: &[i16]) -> HexMap {
        HexMap::from_tiles(
            shape
                .coords()
                .zip(elevations)
                .map(|(coords, &elevation)| (coords, HexMapTile::from_properties(elevation)))
                .collect(),
        )
    }

    fn strip_ansi_escape_codes(text: &str) -> String {
        let mut stripped = String::with_capacity(text.len());
        let mut in_escape_code = false;
        for character in text.chars() {
            match character {
                '\x1b' => in_escape_code = true,
                'm' if in_escape_code => in_escape_code = false,
                _ if in_escape_code => {}
                _ => stripped.push(character),
            }
        }
        stripped
    }

    #[test]
    fn test_text_renderer_render_hex_map() {
        let settings = MapGeneratorSettings::new(0, 3, 2);
        let shape = RectangularHexMapShape::new(3, 2);
        let map = build_hex_map(&shape, &[-200, 100, 4000, -50, 600, 2500]);

        assert_eq!(
            TextRenderer::new(false).render_hex_map(&map, &settings),
            "+--------+  N\n\
             | ~ . A  |\n\
             |  - : ^ |\n\
             +--------+  S\n\
             ~ < -128 m  - < 0 m  . < 512 m  : < 1024 m  n < 2048 m  ^ < 3072 m  A >= 3072 m\n"
        );
    }

    #[test]
    fn test_text_renderer_render_hex_map_with_missing_tiles() {
        let settings = MapGeneratorSettings::new(0, 3, 2);
        let shape = RectangularHexMapShape::new(3, 2);
        let map = build_hex_map(&shape, &[-200, 100, 4000, -50]);

        let text = TextRenderer::new(false).render_hex_map(&map, &settings);
        assert_eq!(text.lines().nth(2), Some("|  - ? ? |"));
    }

    #[test]
    fn test_text_renderer_terrain_glyphs() {
        // one elevation per band of the terrain legend
        let settings = MapGeneratorSettings::new(0, 7, 1);
        let shape = RectangularHexMapShape::new(7, 1);
        let map = build_hex_map(&shape, &[-200, -50, 100, 600, 1500, 2500, 4000]);

        let text = TextRenderer::new(false).render_hex_map(&map, &settings);
        assert_eq!(text.lines().nth(1), Some("| ~ - . : n ^ A  |"));
    }

    #[test]
    fn test_text_renderer_render_field() {
        // the height increases linearly from one row to the next
        let field = ElevationField::sample(RectangularHexMapShape::new(2, 4), &Height, |_| {});
        let text = TextRenderer::new(false).render_field(&field);
        let lines: Vec<&str> = text.lines().collect();

        assert!(lines[0].starts_with("min -"), "{}", lines[0]);
        assert!(lines[0].contains(r#"" .:-=+*#%@""#), "{}", lines[0]);
        assert_eq!(
            lines[1..],
            [
                "+------+  N",
                "|      |",
                "|  - - |",
                "| * *  |",
                "|  @ @ |",
                "+------+  S",
            ]
        );
    }

    #[test]
    fn test_text_renderer_colors_only_add_escape_codes() {
        let settings = MapGeneratorSettings::new(0, 3, 2);
        let shape = RectangularHexMapShape::new(3, 2);
        let map = build_hex_map(&shape, &[-200, 100, 4000, -50, 600, 2500]);
        let field = ElevationField::sample(shape, &Height, |_| {});

        let colored = TextRenderer::new(true).render_hex_map(&map, &settings);
        assert!(colored.contains("\x1b[48;2;"));
        assert_eq!(
            strip_ansi_escape_codes(&colored),
            TextRenderer::new(false).render_hex_map(&map, &settings)
        );

        let colored = TextRenderer::new(true).render_field(&field);
        assert!(colored.contains("\x1b[48;2;"));
        assert_eq!(
            strip_ansi_escape_codes(&colored),
            TextRenderer::new(false).render_field(&field)
        );
    }

    /// Displays a generated planet and all its subgroups. Run with:
    /// `cargo test -p project_x_procgen view_planet -- --ignored --nocapture`
    /// (with `PROJECT_X_TEXT_COLORS=1` for colors).
    #[test]
    #[ignore]
    fn view_planet() {
        let settings = MapGeneratorSettings::new(0, 48, 30);
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let graph = PlanetGraph::build(&settings);
        let renderer = TextRenderer::from_env();

        for id in SubGroupId::ALL {
            let field = ElevationField::sample(shape, graph.node(id), |_| {});
            println!("{}\n{}", id.label(), renderer.render_field(&field));
        }

        let map = ElevationField::sample(shape, graph.unscaled_final_planet(), |_| {})
            .into_hex_map(&settings);
        println!("planet\n{}", renderer.render_hex_map(&map, &settings));
    }
}
