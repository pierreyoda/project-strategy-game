use noise::NoiseFn;
use project_x_core::hex_map::{
    HexMap, HexMapStorage, shape::RectangularHexMapShape, tile::HexMapTile,
};

use super::{projection::CylinderProjection, settings::MapGeneratorSettings};

/// Noise values sampled at the center of each hex of a map.
pub struct ElevationField {
    shape: RectangularHexMapShape,
    /// In the same order as `RectangularHexMapShape::coords`.
    values: Vec<f64>,
}

impl ElevationField {
    /// Sample the given noise function, usually `PlanetGraph::unscaled_final_planet`,
    /// at the center of each hex.
    pub fn sample(shape: RectangularHexMapShape, source: &impl NoiseFn<f64, 3>) -> Self {
        let projection = CylinderProjection::new(&shape);
        let values = shape
            .coords()
            .map(|coords| source.get(projection.project(coords.into())))
            .collect();
        Self { shape, values }
    }

    pub fn shape(&self) -> RectangularHexMapShape {
        self.shape
    }

    /// In the same order as `RectangularHexMapShape::coords`: row by row from north to south,
    /// each row from west to east.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Build the `HexMap`, converting the elevations from planetary elevation units to meters
    /// like the final planet subgroup of libnoise's complex planet example.
    pub fn into_hex_map(self, settings: &MapGeneratorSettings) -> HexMap {
        let min_elevation = settings.get_min_elevation();
        let max_elevation = settings.get_max_elevation();
        let scale = (max_elevation - min_elevation) / 2.0;
        let bias = min_elevation + ((max_elevation - min_elevation) / 2.0);

        let mut tiles = HexMapStorage::with_capacity(self.values.len());
        for (coords, value) in self.shape.coords().zip(self.values) {
            // same computation as `noise::ScaleBias`; float to integer casts saturate
            let elevation = value.mul_add(scale, bias).round() as i16;
            tiles.insert(coords, HexMapTile::from_properties(elevation));
        }
        HexMap::from_tiles(tiles)
    }
}

#[cfg(test)]
mod tests {
    use noise::{Constant, NoiseFn};
    use project_x_core::hex_map::{coordinates::OffsetCoords, shape::RectangularHexMapShape};

    use crate::map::{
        graph::PlanetGraph, projection::CylinderProjection, settings::MapGeneratorSettings,
    };

    use super::ElevationField;

    /// Outputs the height of the sampled point on the cylinder.
    struct Height;

    impl NoiseFn<f64, 3> for Height {
        fn get(&self, point: [f64; 3]) -> f64 {
            point[1]
        }
    }

    #[test]
    fn test_elevation_field_is_sampled_row_by_row_from_north_to_south() {
        let shape = RectangularHexMapShape::new(5, 4);
        let field = ElevationField::sample(shape, &Height);
        let height_bounds = CylinderProjection::new(&shape).height_bounds();

        assert_eq!(field.values().len(), shape.tiles_count());
        assert_eq!(field.values()[0], height_bounds.0);
        for (row, values) in field.values().chunks(5).enumerate() {
            assert!(values.iter().all(|&value| value == values[0]), "row {row}");
            if row > 0 {
                assert!(values[0] > field.values()[(row - 1) * 5], "row {row}");
            }
        }
    }

    #[test]
    fn test_elevation_field_into_hex_map_converts_to_meters() {
        let settings = MapGeneratorSettings::default();
        let shape = RectangularHexMapShape::new(3, 2);
        let cases = [
            (-1.0, -8192),
            (0.0, 0),
            (0.5, 4096),
            (1.0, 8192),
            (0.00006, 0),
            (0.00007, 1),
            (5.0, i16::MAX),
            (-5.0, i16::MIN),
        ];
        for (value, expected) in cases {
            let map = ElevationField::sample(shape, &Constant::new(value)).into_hex_map(&settings);
            for coords in shape.coords() {
                let tile = map.tile(&coords).expect("missing tile");
                assert_eq!(tile.elevation(), expected, "value {value} at {coords:?}");
            }
        }
    }

    #[test]
    fn test_elevation_field_from_planet_graph() {
        let settings = MapGeneratorSettings::new(0, 16, 10);
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let graph = PlanetGraph::build(&settings);
        let map =
            ElevationField::sample(shape, graph.unscaled_final_planet()).into_hex_map(&settings);

        let elevations: Vec<i16> = shape
            .coords()
            .map(|coords| map.tile(&coords).expect("missing tile").elevation())
            .collect();
        assert!(
            elevations.iter().any(|&elevation| elevation < 0),
            "no ocean"
        );
        assert!(elevations.iter().any(|&elevation| elevation > 0), "no land");

        let corner = OffsetCoords::new(15, 9).into();
        assert!(map.tile(&corner).is_some());
    }
}
