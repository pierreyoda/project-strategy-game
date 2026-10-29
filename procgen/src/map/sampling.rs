use noise::NoiseFn;
use project_x_core::hex_map::{
    HexMap, HexMapStorage, coordinates::CubeCoordsScalar, shape::RectangularHexMapShape,
    tile::HexMapTile,
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
    ///
    /// `on_row_sampled` is called with the index of each row once sampled, from north to south.
    pub fn sample(
        shape: RectangularHexMapShape,
        source: &impl NoiseFn<f64, 3>,
        mut on_row_sampled: impl FnMut(CubeCoordsScalar),
    ) -> Self {
        let projection = CylinderProjection::new(&shape);
        let columns = shape.columns() as usize;
        let mut values = Vec::with_capacity(shape.tiles_count());
        for (index, coords) in shape.coords().enumerate() {
            values.push(source.get(projection.project(coords.into())));
            if (index + 1) % columns == 0 {
                on_row_sampled((index / columns) as CubeCoordsScalar);
            }
        }
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
        let (scale, bias) = settings.get_elevation_scale_bias();

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
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use crate::map::{projection::CylinderProjection, settings::MapGeneratorSettings};

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
        let mut sampled_rows = Vec::new();
        let field = ElevationField::sample(shape, &Height, |row| sampled_rows.push(row));
        let height_bounds = CylinderProjection::new(&shape).height_bounds();

        assert_eq!(sampled_rows, [0, 1, 2, 3]);
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
            let map = ElevationField::sample(shape, &Constant::new(value), |_| {})
                .into_hex_map(&settings);
            for coords in shape.coords() {
                let tile = map.tile(&coords).expect("missing tile");
                assert_eq!(tile.elevation(), expected, "value {value} at {coords:?}");
            }
        }
    }
}
