//! Image renderer, to draw maps as images (for instance saved as PNG files).
//!
//! Draws the actual pointy-top hexagons, odd rows being shoved half a hex to the east,
//! with the same colors as the colored `TextRenderer`. Row 0 is the north pole, at the top.
//! Pixels outside of the map are transparent.

use image::{Rgba, RgbaImage};
use project_x_core::hex_map::{
    HexMap,
    coordinates::{CubeCoords, OffsetCoords},
    shape::RectangularHexMapShape,
};

use super::{TerrainLegend, ValueRange};
use crate::map::{ElevationField, MapGeneratorSettings};

const TRANSPARENT: Rgba<u8> = Rgba([0, 0, 0, 0]);

/// Renders maps as images.
#[derive(Copy, Clone, Debug)]
pub struct ImageRenderer {
    /// Distance from the center of a hex to its corners, in pixels.
    hex_size: f64,
}

impl ImageRenderer {
    /// `hex_size` is the distance from the center of a hex to its corners, in pixels.
    pub fn new(hex_size: u32) -> Self {
        assert!(hex_size > 0, "hex_size > 0");
        Self {
            hex_size: hex_size as f64,
        }
    }

    /// Render the elevations of a map generated with the given settings, with the colors
    /// of noise's terrain gradient.
    pub fn render_hex_map(&self, map: &HexMap, settings: &MapGeneratorSettings) -> RgbaImage {
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let legend = TerrainLegend::new(settings);
        self.render_grid(&shape, |coords| {
            map.tile(&coords.into())
                .map(|tile| legend.color(tile.elevation()))
        })
    }

    /// Render raw noise values, in grays from black (their minimum) to white (their maximum).
    pub fn render_field(&self, field: &ElevationField) -> RgbaImage {
        let shape = field.shape();
        let columns = shape.columns() as usize;
        let values = field.values();
        let range = ValueRange::of(values);
        self.render_grid(&shape, |coords| {
            Some(range.gray(values[coords.row() as usize * columns + coords.col() as usize]))
        })
    }

    fn render_grid(
        &self,
        shape: &RectangularHexMapShape,
        mut color: impl FnMut(OffsetCoords) -> Option<[u8; 3]>,
    ) -> RgbaImage {
        let (columns, rows) = (shape.columns(), shape.rows());
        // missing tiles are transparent, like the outside of the map
        let colors: Vec<Rgba<u8>> = shape
            .coords()
            .map(|coords| match color(coords.into()) {
                Some([r, g, b]) => Rgba([r, g, b, 255]),
                None => TRANSPARENT,
            })
            .collect();

        // pointy-top hexes: neighbors in a row are √3 sizes apart, and rows 1.5 sizes apart
        let size = self.hex_size;
        let hex_width = 3f64.sqrt() * size;
        let width = (hex_width * (columns as f64 + 0.5)).ceil() as u32;
        let height = (1.5 * size * (rows - 1) as f64 + 2.0 * size).ceil() as u32;
        RgbaImage::from_fn(width, height, |x, y| {
            // pixel center, relative to the center of the hex (0, 0)
            let px = x as f64 + 0.5 - hex_width / 2.0;
            let py = y as f64 + 0.5 - size;
            let q = (3f64.sqrt() / 3.0 * px - py / 3.0) / size;
            let r = (2.0 / 3.0 * py) / size;
            let offset = OffsetCoords::from(round_axial_coords(q, r));
            if (0..columns).contains(&offset.col()) && (0..rows).contains(&offset.row()) {
                colors[offset.row() as usize * columns as usize + offset.col() as usize]
            } else {
                TRANSPARENT
            }
        })
    }
}

/// Hex containing the given fractional axial coordinates.
///
/// See: https://www.redblobgames.com/grids/hexagons/#rounding
fn round_axial_coords(q: f64, r: f64) -> CubeCoords {
    let s = -q - r;
    let (mut rounded_q, mut rounded_r, rounded_s) = (q.round(), r.round(), s.round());
    let q_diff = (rounded_q - q).abs();
    let r_diff = (rounded_r - r).abs();
    let s_diff = (rounded_s - s).abs();
    if q_diff > r_diff && q_diff > s_diff {
        rounded_q = -rounded_r - rounded_s;
    } else if r_diff > s_diff {
        rounded_r = -rounded_q - rounded_s;
    }
    CubeCoords::from_axial_coords(rounded_q as i16, rounded_r as i16)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::Rgba;
    use noise::NoiseFn;
    use project_x_core::hex_map::{HexMap, shape::RectangularHexMapShape, tile::HexMapTile};

    use crate::map::{
        ElevationField, MapGeneratorSettings, PlanetGraph, SubGroupId, renderers::TerrainLegend,
    };

    use super::{ImageRenderer, TRANSPARENT};

    const HEX_SIZE: f64 = 10.0;

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

    /// Pixel at the center of the given hex.
    fn hex_center(col: i16, row: i16) -> (u32, u32) {
        let hex_width = 3f64.sqrt() * HEX_SIZE;
        let x = (col as f64 + 0.5 * (row & 1) as f64) * hex_width + hex_width / 2.0;
        let y = 1.5 * HEX_SIZE * row as f64 + HEX_SIZE;
        (x as u32, y as u32)
    }

    fn opaque([r, g, b]: [u8; 3]) -> Rgba<u8> {
        Rgba([r, g, b, 255])
    }

    #[test]
    fn test_image_renderer_render_hex_map() {
        let settings = MapGeneratorSettings::new(0, 3, 2);
        let shape = RectangularHexMapShape::new(3, 2);
        let elevations = [-200, 100, 4000, -50, 600, 2500];
        let map = build_hex_map(&shape, &elevations);
        let legend = TerrainLegend::new(&settings);

        let image = ImageRenderer::new(HEX_SIZE as u32).render_hex_map(&map, &settings);
        // 3.5 hexes wide, and 2 rows of hexes overlapping by a quarter of their height
        assert_eq!(image.dimensions(), (61, 35));

        for (index, elevation) in elevations.into_iter().enumerate() {
            let (col, row) = ((index % 3) as i16, (index / 3) as i16);
            let (x, y) = hex_center(col, row);
            assert_eq!(
                *image.get_pixel(x, y),
                opaque(legend.color(elevation)),
                "({col}, {row})"
            );
        }

        // around the top corner of the first hex
        assert_eq!(*image.get_pixel(0, 0), TRANSPARENT);
        // odd rows are shoved half a hex to the east, and even rows end half a hex earlier
        assert_eq!(*image.get_pixel(0, hex_center(0, 1).1), TRANSPARENT);
        assert_eq!(*image.get_pixel(60, hex_center(0, 0).1), TRANSPARENT);
        assert_ne!(*image.get_pixel(60, hex_center(0, 1).1), TRANSPARENT);
    }

    #[test]
    fn test_image_renderer_render_hex_map_with_missing_tiles() {
        let settings = MapGeneratorSettings::new(0, 3, 2);
        let shape = RectangularHexMapShape::new(3, 2);
        let map = build_hex_map(&shape, &[-200, 100, 4000, -50]);

        let image = ImageRenderer::new(HEX_SIZE as u32).render_hex_map(&map, &settings);
        let (x, y) = hex_center(0, 1);
        assert_ne!(*image.get_pixel(x, y), TRANSPARENT);
        let (x, y) = hex_center(2, 1);
        assert_eq!(*image.get_pixel(x, y), TRANSPARENT);
    }

    #[test]
    fn test_image_renderer_render_field() {
        // the height increases linearly from one row to the next
        let field = ElevationField::sample(RectangularHexMapShape::new(2, 4), &Height, |_| {});
        let image = ImageRenderer::new(HEX_SIZE as u32).render_field(&field);

        for col in 0..2 {
            let (x, y) = hex_center(col, 0);
            assert_eq!(*image.get_pixel(x, y), opaque([0; 3]));
            let (x, y) = hex_center(col, 3);
            assert_eq!(*image.get_pixel(x, y), opaque([255; 3]));
        }
    }

    /// Draws a generated planet and all its subgroups as PNG files in `target/procgen-images`.
    /// Run with:
    /// `cargo test -p project_x_procgen --features debug-images render_planet_images -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn render_planet_images() {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/procgen-images");
        std::fs::create_dir_all(&directory).expect("cannot create the images directory");

        let settings = MapGeneratorSettings::default();
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let graph = PlanetGraph::build(&settings);
        let renderer = ImageRenderer::new(8);

        for (index, id) in SubGroupId::ALL.into_iter().enumerate() {
            let field = ElevationField::sample(shape, graph.node(id), |_| {});
            let filename = format!("{index:02}_{}.png", id.label().replace(' ', "_"));
            renderer
                .render_field(&field)
                .save(directory.join(&filename))
                .expect("cannot save the image");
        }

        let map = ElevationField::sample(shape, graph.unscaled_final_planet(), |_| {})
            .into_hex_map(&settings);
        renderer
            .render_hex_map(&map, &settings)
            .save(directory.join("planet.png"))
            .expect("cannot save the image");

        let directory = directory.canonicalize().expect("missing images directory");
        println!("images written to {}", directory.display());
    }
}
