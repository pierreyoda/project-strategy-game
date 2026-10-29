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

    /// Planetary elevation that becomes sea level, so that the seas cover the given share
    /// of the map: the matching quantile of the sampled values.
    ///
    /// This is the one part of the elevations that does not come from the noise graph,
    /// which leaves the share of the map underwater to the whims of the seed.
    pub fn water_line(&self, ocean_coverage: f64) -> f64 {
        let Some(last) = self.values.len().checked_sub(1) else {
            return 0.0;
        };
        let index = (ocean_coverage * self.values.len() as f64).round();
        // casts from floats saturate, `NaN` giving 0
        let index = (index as usize).min(last);
        *self
            .values
            .clone()
            .select_nth_unstable_by(index, f64::total_cmp)
            .1
    }

    /// Build the `HexMap`, converting the elevations from planetary elevation units to meters
    /// like the final planet subgroup of libnoise's complex planet example, with the water
    /// line of the wanted ocean coverage as sea level.
    pub fn into_hex_map(self, settings: &MapGeneratorSettings) -> HexMap {
        let (scale, bias) = settings.get_elevation_scale_bias();
        let water_line = self.water_line(settings.get_ocean_coverage());

        let mut tiles = HexMapStorage::with_capacity(self.values.len());
        for (coords, value) in self.shape.coords().zip(self.values) {
            let elevation = to_meters(value, water_line, scale, bias);
            tiles.insert(coords, HexMapTile::from_properties(elevation));
        }
        HexMap::from_tiles(tiles)
    }
}

/// Elevation in meters of a sampled value, the water line becoming sea level.
fn to_meters(value: f64, water_line: f64, scale: f64, bias: f64) -> i16 {
    // same computation as `noise::ScaleBias`; float to integer casts saturate
    (value - water_line).mul_add(scale, bias).round() as i16
}

#[cfg(test)]
mod tests {
    use noise::{Constant, NoiseFn};
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use crate::map::{projection::CylinderProjection, settings::MapGeneratorSettings};

    use super::{ElevationField, to_meters};

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
    fn test_elevations_are_converted_to_meters_from_the_water_line() {
        let (scale, bias) = MapGeneratorSettings::default().get_elevation_scale_bias();
        let cases = [
            (-1.0, 0.0, -8192),
            (0.0, 0.0, 0),
            (0.5, 0.0, 4096),
            (1.0, 0.0, 8192),
            (0.00006, 0.0, 0),
            (0.00007, 0.0, 1),
            (5.0, 0.0, i16::MAX),
            (-5.0, 0.0, i16::MIN),
            // the water line is sea level, whatever its own elevation
            (0.125, 0.125, 0),
            (0.25, 0.125, 1024),
            (0.0, 0.125, -1024),
            (-1.0, 0.125, -9216),
        ];
        for (value, water_line, expected) in cases {
            assert_eq!(
                to_meters(value, water_line, scale, bias),
                expected,
                "value {value}, water line {water_line}"
            );
        }
    }

    #[test]
    fn test_elevation_field_water_line_leaves_the_wanted_share_of_the_map_underwater() {
        // one value per row, from the north pole to the south pole
        let shape = RectangularHexMapShape::new(1, 10);
        let field = ElevationField::sample(shape, &Height, |_| {});
        let values = field.values().to_vec();

        for (ocean_coverage, expected) in [(0.0, 0), (0.3, 3), (0.5, 5), (0.7, 7), (1.0, 9)] {
            let water_line = field.water_line(ocean_coverage);
            assert_eq!(
                water_line.to_bits(),
                values[expected].to_bits(),
                "ocean coverage {ocean_coverage}"
            );
            let underwater = values.iter().filter(|&&value| value < water_line).count();
            assert_eq!(underwater, expected, "ocean coverage {ocean_coverage}");
        }
    }

    #[test]
    fn test_elevation_field_into_hex_map_floods_the_map_up_to_the_water_line() {
        let settings = MapGeneratorSettings::default();
        let shape = RectangularHexMapShape::new(2, 10);
        let field = ElevationField::sample(shape, &Height, |_| {});
        let water_line = field.water_line(settings.get_ocean_coverage());
        let map = field.into_hex_map(&settings);

        let underwater = shape
            .coords()
            .filter(|coords| map.tile(coords).expect("missing tile").elevation() < 0)
            .count();
        // 70% of the rows are below the water line, each of them holding 2 of the 20 hexes
        assert_eq!(underwater, 14);
        assert!(water_line > 0.0, "water line {water_line}");
    }

    #[test]
    fn test_elevation_field_into_hex_map_converts_a_flat_field_to_sea_level() {
        let settings = MapGeneratorSettings::default();
        let shape = RectangularHexMapShape::new(3, 2);
        let map =
            ElevationField::sample(shape, &Constant::new(0.5), |_| {}).into_hex_map(&settings);
        for coords in shape.coords() {
            assert_eq!(map.tile(&coords).expect("missing tile").elevation(), 0);
        }
    }
}
