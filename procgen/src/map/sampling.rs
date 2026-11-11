use std::collections::VecDeque;

use noise::NoiseFn;
use project_x_core::hex_map::{
    HexMap, HexMapStorage,
    coordinates::{CubeCoordsScalar, OffsetCoords},
    shape::RectangularHexMapShape,
    tile::HexMapTile,
};

use super::{
    coastline::Coastline, projection::CylinderProjection, settings::MapGeneratorSettings,
    terrain::TERRAIN_GRADIENT_BOUNDS,
};

/// Depth of the shallow band below the water line, in planetary elevation units.
const SHALLOW_BAND: f64 = -TERRAIN_GRADIENT_BOUNDS[0];
/// A shelf tile already this shallow keeps its depth...
const SHELF_KEPT: f64 = SHALLOW_BAND / 2.0;
/// ...a deeper one is compressed towards this depth, never reaching it: inside the band.
const SHELF_FLOOR: f64 = SHALLOW_BAND * 0.9;
/// Depth beyond `SHELF_KEPT` at which a tile lands halfway between it and `SHELF_FLOOR`, about
/// 1 000 m: the abyss one tile off a coast comes out near the floor, a shoal near the top.
const SHELF_HALF_DEPTH: f64 = 0.125;

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
    ///
    /// The `Coastline` decides which tiles are land. A tile it leaves on the side of the water line
    /// it was sampled on keeps its sampled elevation. One it sinks takes the depth of the blended
    /// field below the coastline's own water line, on the right side of sea level - shallows
    /// where a neck was cut. One it raises from the sea takes the elevation of the land around
    /// it, spread in from the coast it grew from: a bay filled between hills is hilly too.
    pub fn into_hex_map(self, settings: &MapGeneratorSettings) -> HexMap {
        let (scale, bias) = settings.get_elevation_scale_bias();
        let water_line = self.water_line(settings.get_ocean_coverage());
        let flattening = settings.get_lowland_flattening();
        let coastline = Coastline::draw(self.shape, &self.values, settings);
        let shelf = self.shelf(coastline.land(), settings.get_coastal_shelf_width());

        let mut elevations = Vec::with_capacity(self.values.len());
        let mut raised = Vec::with_capacity(self.values.len());
        for (index, (&value, &on_shelf)) in self.values.iter().zip(&shelf).enumerate() {
            let land = coastline.is_land(index);
            raised.push(land && value < water_line);
            let above = if land == (value >= water_line) {
                value - water_line
            } else {
                coastline.relief(index)
            };
            let above = if on_shelf { shelf_value(above) } else { above };
            let elevation = to_meters(above, flattening, scale, bias);
            elevations.push(if land {
                elevation.max(0)
            } else {
                elevation.min(-1)
            });
        }
        self.spread_inland(&mut elevations, &raised);

        let mut tiles = HexMapStorage::with_capacity(self.values.len());
        for (coords, elevation) in self.shape.coords().zip(elevations) {
            tiles.insert(coords, HexMapTile::from_properties(elevation));
        }
        HexMap::from_tiles(tiles)
    }

    /// Give each raised tile the mean elevation of its neighbors on land already settled - the
    /// land as sampled first, then the raised tiles one step further at a time: a breadth-first
    /// search from the coasts the coastline grew from. A raised tile out of reach of any
    /// sampled land keeps its elevation.
    fn spread_inland(&self, elevations: &mut [i16], raised: &[bool]) {
        let columns = self.shape.columns() as usize;
        let index = |coords: OffsetCoords| coords.row() as usize * columns + coords.col() as usize;
        let neighbors: Vec<Vec<usize>> = self
            .shape
            .coords()
            .map(|coords| {
                self.shape
                    .neighbors(coords)
                    .map(|neighbor| index(neighbor.into()))
                    .collect()
            })
            .collect();
        let land = |tile: usize| elevations[tile] >= 0;
        let mut settled: Vec<bool> = (0..raised.len())
            .map(|tile| land(tile) && !raised[tile])
            .collect();
        let mut ring: Vec<usize> = (0..raised.len())
            .filter(|&tile| raised[tile] && neighbors[tile].iter().any(|&n| settled[n]))
            .collect();
        while !ring.is_empty() {
            // a whole ring from the elevations of the ones before, so that the order within it
            // does not matter
            let spread: Vec<(usize, i16)> = ring
                .iter()
                .map(|&tile| {
                    let around: Vec<i32> = neighbors[tile]
                        .iter()
                        .filter(|&&n| settled[n])
                        .map(|&n| i32::from(elevations[n]))
                        .collect();
                    let mean = around.iter().sum::<i32>() as f64 / around.len() as f64;
                    (tile, mean.round() as i16)
                })
                .collect();
            for &(tile, elevation) in &spread {
                elevations[tile] = elevation;
                settled[tile] = true;
            }
            let mut next: Vec<usize> = spread
                .iter()
                .flat_map(|&(tile, _)| neighbors[tile].iter().copied())
                .filter(|&n| raised[n] && !settled[n])
                .collect();
            next.sort_unstable();
            next.dedup();
            ring = next;
        }
    }

    /// Whether each tile is sea within `width` steps of land, in the order of the values: a
    /// breadth-first search from every shore at once, wrapping around east-west like the map.
    fn shelf(&self, land: &[bool], width: u32) -> Vec<bool> {
        let columns = self.shape.columns() as usize;
        let index = |coords: OffsetCoords| coords.row() as usize * columns + coords.col() as usize;
        let mut distance = vec![u32::MAX; self.values.len()];
        let mut queue = VecDeque::new();
        for coords in self.shape.coords() {
            if land[index(coords.into())] {
                distance[index(coords.into())] = 0;
                queue.push_back(coords);
            }
        }
        while let Some(coords) = queue.pop_front() {
            let next = distance[index(coords.into())] + 1;
            if next > width {
                continue;
            }
            for neighbor in self.shape.neighbors(coords) {
                let neighbor_index = index(neighbor.into());
                if distance[neighbor_index] == u32::MAX {
                    distance[neighbor_index] = next;
                    queue.push_back(neighbor);
                }
            }
        }
        distance
            .into_iter()
            .map(|distance| distance > 0 && distance <= width)
            .collect()
    }
}

/// A sea tile's height once on the shelf, below the water line: depths already within the upper
/// half of the shallow band are kept, deeper ones compressed into its lower half, so that the sea
/// floor keeps the order it was sampled in and every shelf tile reads as shallow water.
fn shelf_value(above: f64) -> f64 {
    let depth = -above;
    if depth <= SHELF_KEPT {
        return above;
    }
    let beyond = depth - SHELF_KEPT;
    let compressed = SHELF_KEPT + (SHELF_FLOOR - SHELF_KEPT) * beyond / (beyond + SHELF_HALF_DEPTH);
    -compressed
}

/// Elevation in meters of a height above the water line, in planetary elevation units, the
/// water line becoming sea level and the lowlands above it flattened: a height `h` becomes
/// `h² / (h + flattening)`, which sinks the plateaus towards the plains and costs a peak little
/// more than the flattening itself. The seas keep their depths.
fn to_meters(above: f64, flattening: f64, scale: f64, bias: f64) -> i16 {
    let relieved = if above > 0.0 {
        above * above / (above + flattening)
    } else {
        above
    };
    // same computation as `noise::ScaleBias`; float to integer casts saturate
    relieved.mul_add(scale, bias).round() as i16
}

#[cfg(test)]
mod tests {
    use noise::{Constant, NoiseFn};
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use crate::map::{projection::CylinderProjection, settings::MapGeneratorSettings};

    use super::{ElevationField, SHALLOW_BAND, shelf_value, to_meters};

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
            (-1.0, 0.0, 0.0, -8192),
            (0.0, 0.0, 0.0, 0),
            (0.5, 0.0, 0.0, 4096),
            (1.0, 0.0, 0.0, 8192),
            (0.00006, 0.0, 0.0, 0),
            (0.00007, 0.0, 0.0, 1),
            (5.0, 0.0, 0.0, i16::MAX),
            (-5.0, 0.0, 0.0, i16::MIN),
            // the water line is sea level, whatever its own elevation
            (0.125, 0.125, 0.0, 0),
            (0.25, 0.125, 0.0, 1024),
            (0.0, 0.125, 0.0, -1024),
            (-1.0, 0.125, 0.0, -9216),
            // the flattening halves a lowland as high as itself, costs a peak about itself,
            // and leaves the seas alone
            (0.25, 0.125, 0.125, 512),
            (0.5, 0.125, 0.125, 2304),
            (1.125, 0.125, 0.125, 7282),
            (0.125, 0.125, 0.125, 0),
            (0.0, 0.125, 0.125, -1024),
            (-1.0, 0.125, 0.125, -9216),
        ];
        for (value, water_line, flattening, expected) in cases {
            assert_eq!(
                to_meters(value - water_line, flattening, scale, bias),
                expected,
                "value {value}, water line {water_line}, flattening {flattening}"
            );
        }
    }

    #[test]
    fn test_shelf_keeps_shoals_and_brings_the_abyss_up_into_the_shallow_band() {
        // a shoal keeps its depth
        assert_eq!(shelf_value(-0.005), -0.005);
        // depths of any size end up shallower than the band, in the order they came in
        let depths = [0.01, 0.05, 0.3, 1.0, 5.0];
        let shelved: Vec<f64> = depths.iter().map(|depth| -shelf_value(-depth)).collect();
        for (depth, shelved) in depths.iter().zip(&shelved) {
            assert!(
                *shelved > 0.0 && *shelved < SHALLOW_BAND,
                "{depth} became {shelved}"
            );
        }
        assert!(
            shelved.windows(2).all(|pair| pair[0] < pair[1]),
            "{shelved:?}"
        );
    }

    #[test]
    fn test_elevation_field_into_hex_map_lays_a_shelf_along_the_coast() {
        // one value per row from the north pole to the south pole: sea to the north, land to the
        // south, the coast between rows 5 and 6
        let shape = RectangularHexMapShape::new(2, 10);
        let shallow = |map: &project_x_core::hex_map::HexMap, row: i16| {
            let coords = project_x_core::hex_map::coordinates::OffsetCoords::new(0, row).into();
            let elevation = map.tile(&coords).expect("missing tile").elevation();
            (-128..0).contains(&elevation)
        };

        let sampled = MapGeneratorSettings::default()
            .with_ocean_coverage(0.6)
            .with_sampled_coastline();
        let map = ElevationField::sample(shape, &Height, |_| {}).into_hex_map(&sampled);
        assert!(shallow(&map, 5), "the row along the coast");
        assert!(!shallow(&map, 4), "the next one out");
        assert!(!shallow(&map, 6), "the land");

        let wide = sampled.clone().with_coastal_shelf_width(2);
        let map = ElevationField::sample(shape, &Height, |_| {}).into_hex_map(&wide);
        assert!(shallow(&map, 5) && shallow(&map, 4) && !shallow(&map, 3));

        let none = sampled.with_coastal_shelf_width(0);
        let map = ElevationField::sample(shape, &Height, |_| {}).into_hex_map(&none);
        assert!(!shallow(&map, 5), "the sea floor as sampled");
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
        let settings = MapGeneratorSettings::default().with_sampled_coastline();
        let shape = RectangularHexMapShape::new(2, 10);
        let field = ElevationField::sample(shape, &Height, |_| {});
        let water_line = field.water_line(settings.get_ocean_coverage());
        let map = field.into_hex_map(&settings);

        let underwater = shape
            .coords()
            .filter(|coords| map.tile(coords).expect("missing tile").elevation() < 0)
            .count();
        // the covered share of the rows is below the water line, each row holding 2 hexes
        let rows_underwater = (settings.get_ocean_coverage() * 10.0).round() as usize;
        assert_eq!(underwater, rows_underwater * 2);
        assert!(water_line > 0.0, "water line {water_line}");
    }

    #[test]
    fn test_elevation_field_into_hex_map_converts_a_flat_field_to_sea_level() {
        let settings = MapGeneratorSettings::default().with_sampled_coastline();
        let shape = RectangularHexMapShape::new(3, 2);
        let map =
            ElevationField::sample(shape, &Constant::new(0.5), |_| {}).into_hex_map(&settings);
        for coords in shape.coords() {
            assert_eq!(map.tile(&coords).expect("missing tile").elevation(), 0);
        }
    }

    /// Rows from the north pole to the south pole that hold land, and the land tiles.
    fn land_rows(
        map: &project_x_core::hex_map::HexMap,
        shape: RectangularHexMapShape,
    ) -> (Vec<bool>, usize) {
        let mut rows = vec![false; shape.rows() as usize];
        let mut land = 0;
        for coords in shape.coords() {
            if map.tile(&coords).expect("missing tile").elevation() >= 0 {
                let row = project_x_core::hex_map::coordinates::OffsetCoords::from(coords).row();
                rows[row as usize] = true;
                land += 1;
            }
        }
        (rows, land)
    }

    #[test]
    fn test_raised_tiles_take_the_elevation_of_the_land_around_them() {
        // one row of 6, wrapping around: sampled land at 300 m and 900 m, then three raised
        // tiles, then the sea
        let shape = RectangularHexMapShape::new(6, 1);
        let field = ElevationField::sample(shape, &Constant::new(0.0), |_| {});
        let mut elevations = vec![300, 900, 0, 0, 0, -50];
        let raised = [false, false, true, true, true, false];
        field.spread_inland(&mut elevations, &raised);
        // each from the one before it, the sea never counting
        assert_eq!(elevations, [300, 900, 900, 900, 900, -50]);

        // between two settled tiles, the mean
        let mut elevations = vec![300, 0, 900, -50, -50, -50];
        field.spread_inland(&mut elevations, &[false, true, false, false, false, false]);
        assert_eq!(elevations[1], 600);
        // out of reach of any sampled land, as it was
        let mut elevations = vec![-50, 12, -50, -50, -50, -50];
        field.spread_inland(&mut elevations, &[false, true, false, false, false, false]);
        assert_eq!(elevations[1], 12);
    }

    #[test]
    fn test_elevation_field_into_hex_map_keeps_the_land_off_a_closed_pole() {
        // the land rises towards the south pole
        let shape = RectangularHexMapShape::new(6, 20);
        let settings = MapGeneratorSettings::default()
            .with_ocean_coverage(0.6)
            .with_polar_land_chance(1.0);
        let map = ElevationField::sample(shape, &Height, |_| {}).into_hex_map(&settings);
        let (rows, land) = land_rows(&map, shape);
        assert!(rows[19], "an open pole keeps its land");
        assert_eq!(land, 48);

        let closed = settings.with_polar_land_chance(0.0);
        let map = ElevationField::sample(shape, &Height, |_| {}).into_hex_map(&closed);
        let (rows, land) = land_rows(&map, shape);
        assert!(!rows[19], "a closed pole is open sea: {rows:?}");
        // the land the pole gave up is found further north
        assert_eq!(land, 48);
        assert!(rows[11] && !rows[10], "{rows:?}");
    }
}
