use project_x_core::hex_map::coordinates::CubeCoordsScalar;

pub type MapGeneratorSeedScalar = u32;

/// Highest supported seed.
///
/// The noise functions derive their own seeds by adding offsets to it (up to +145),
/// without overflow checks.
pub const MAX_SEED: MapGeneratorSeedScalar = MapGeneratorSeedScalar::MAX - 256;

/// Procedural map generation settings.
#[derive(Debug)]
pub struct MapGeneratorSettings {
    seed: MapGeneratorSeedScalar,
    /// Number of hexes per row.
    columns: CubeCoordsScalar,
    /// Number of rows, from the north pole to the south pole.
    rows: CubeCoordsScalar,
    /// Frequency of the planet's continents. Higher frequency produces
    /// smaller, more numerous continents. This value is measured in radians.
    continent_frequency: f64,
    /// Lacunarity of the planet's continents. Changing this value produces
    /// slightly different continents. For the best results, this value should
    /// be random, but close to 2.0.
    continent_lacunarity: f64,
    /// Lacunarity of the planet's mountains. Changing the value produces
    /// slightly different mountains. For the best results, this value should
    /// be random, but close to 2.0.
    mountain_lacunarity: f64,
    /// Lacunarity of the planet's hills. Changing this value produces
    /// slightly different hills. For the best results, this value should be
    /// random, but close to 2.0.
    hills_lacunarity: f64,
    /// Lacunarity of the planet's plains. Changing this value produces
    /// slightly different plains. For the best results, this value should be
    /// random, but close to 2.0.
    plains_lacunarity: f64,
    /// Lacunarity of the planet's badlands. Changing this value produces
    /// slightly different badlands. For the best results, this value should
    /// be random, but close to 2.0.
    badlands_lacunarity: f64,
    /// Specifies the "twistiness" of the mountains.
    mountains_twist: f64,
    /// Specifies the "twistiness" of the hills.
    hills_twist: f64,
    /// Specifies the "twistiness" of the badlands.
    badlands_twist: f64,
    /// Specifies the planet's sea level. This value must be between -1.0
    /// (minimum planet elevation) and +1.0 (maximum planet elevation).
    ///
    /// Must stay at 0.0, like the original example: the graph scales the land elevations
    /// around 0.0 by `get_continent_height_scale`, so any other value drowns the continents
    /// instead of raising the seas. `ocean_coverage` is the knob for how much sea there is.
    sea_level: f64,
    /// Share of the map covered by the seas, from 0.0 (no sea) to 1.0 (no land).
    /// Earth is about 0.71.
    ///
    /// Unlike `sea_level`, this is applied to the sampled elevations rather than to the
    /// noise graph: the water line is their matching quantile, which pins the land budget
    /// of every seed instead of letting it swing with the noise.
    ocean_coverage: f64,
    /// Specifies the level on the planet in which continental shelves appear.
    /// This value must be between -1.0 (minimum planet elevation) and +1.0
    /// (maximum planet elevation), and must be less than `SEA_LEVEL`.
    shelf_level: f64,
    /// Determines the amount of mountainous terrain that appears on the
    /// planet. Values range from 0.0 (no mountains) to 1.0 (all terrain is
    /// covered in mountains). Mountains terrain will overlap hilly terrain.
    /// Because the badlands terrain may overlap parts of the mountainous
    /// terrain, setting `MOUNTAINS_AMOUNT` to 1.0 may not completely cover the
    /// terrain in mountains.
    mountains_amount: f64,
    /// Determines the amount of badlands terrain that covers the planet.
    /// Values range from 0.0 (no badlands) to 1.0 (all terrain is covered in
    /// badlands). Badlands terrain will overlap any other type of terrain.
    badlands_amount: f64,
    /// Offset to apply to the terrain type definition. Low values (< 1.0)
    /// cause the rough areas to appear only at high elevations. High values
    /// (> 2.0) cause the rough areas to appear at any elevation. The
    /// percentage of rough areas on the planet are independent of this value.
    terrain_offset: f64,
    /// Specifies the amount of "glaciation" on the mountains. This value
    /// should be close to 1.0 and greater than 1.0.
    mountain_glaciation: f64,
    /// Maximum depth of the rivers, in planetary elevation units.
    river_depth: f64,
    /// Minimum elevation on the planet, in meters. This value is approximate.
    min_elevation: f64,
    /// Maximum elevation on the planet, in meters. This value is approximate.
    max_elevation: f64,
}

impl Default for MapGeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            columns: 80,
            rows: 52,
            // 0.75 rather than the original example's 1.0: the whole continent definition,
            // sampled over a full planet, grows by a third, so that a handful of huge
            // continents hold almost all the land, with open ocean in between
            continent_frequency: 0.75,
            continent_lacunarity: 2.208984375,
            mountain_lacunarity: 2.142578125,
            hills_lacunarity: 2.162109375,
            plains_lacunarity: 2.314453125,
            badlands_lacunarity: 2.212890625,
            mountains_twist: 1.0,
            hills_twist: 1.0,
            badlands_twist: 1.0,
            sea_level: 0.0,
            // Earth is about 71% sea; the noise graph on its own leaves only 33% to 60%
            // of the map underwater, depending on the seed
            ocean_coverage: 0.7,
            shelf_level: -0.375,
            // 0.6 rather than the original example's 0.5: raising the water line sinks the
            // land by about 800 m, which would otherwise leave the continents nearly flat
            mountains_amount: 0.6,
            badlands_amount: 0.3125,
            terrain_offset: 1.0,
            mountain_glaciation: 1.375,
            river_depth: 0.0234375,
            min_elevation: -8192.0,
            max_elevation: 8192.0,
        }
    }
}

macro_rules! settings_scalar_accessors {
    ($ ( $name: ident : $field: ident, $type: ty, )* ) => {
    $(
        pub fn $name(&self) -> $type {
            self.$field
        }
    )*
    }
}

impl MapGeneratorSettings {
    pub fn new(
        seed: MapGeneratorSeedScalar,
        columns: CubeCoordsScalar,
        rows: CubeCoordsScalar,
    ) -> Self {
        assert!(seed <= MAX_SEED, "seed <= MAX_SEED");
        assert!(columns > 0 && rows > 0, "columns > 0 && rows > 0");
        Self {
            seed,
            columns,
            rows,
            ..Self::default()
        }
    }

    /// Share of the map that the seas should cover, from 0.0 (no sea) to 1.0 (no land).
    pub fn with_ocean_coverage(self, ocean_coverage: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&ocean_coverage),
            "0.0 <= ocean_coverage <= 1.0"
        );
        Self {
            ocean_coverage,
            ..self
        }
    }

    settings_scalar_accessors! {
        get_seed: seed, MapGeneratorSeedScalar,
        get_columns: columns, CubeCoordsScalar,
        get_rows: rows, CubeCoordsScalar,
        get_continent_frequency: continent_frequency, f64,
        get_continent_lacunarity: continent_lacunarity, f64,
        get_mountain_lacunarity: mountain_lacunarity, f64,
        get_hills_lacunarity: hills_lacunarity, f64,
        get_plains_lacunarity: plains_lacunarity, f64,
        get_badlands_lacunarity: badlands_lacunarity, f64,
        get_mountains_twist: mountains_twist, f64,
        get_hills_twist: hills_twist, f64,
        get_badlands_twist: badlands_twist, f64,
        get_sea_level: sea_level, f64,
        get_ocean_coverage: ocean_coverage, f64,
        get_shelf_level: shelf_level, f64,
        get_mountains_amount: mountains_amount, f64,
        get_badlands_amount: badlands_amount, f64,
        get_terrain_offset: terrain_offset, f64,
        get_mountain_glaciation: mountain_glaciation, f64,
        get_river_depth: river_depth, f64,
        get_min_elevation: min_elevation, f64,
        get_max_elevation: max_elevation, f64,
    }

    /// Determines the amount of hilly terrain that appears on the planet.
    /// Values range from 0.0 (no hills) to 1.0 (all terrain is covered in
    /// hills). This value must be less than `MOUNTAINS_AMOUNT`. Because the
    /// mountains terrain will overlap parts of the hilly terrain, and the
    /// badlands terrain may overlap parts of the hilly terrain, setting
    /// `HILLS_AMOUNT` to 1.0 may not completely cover the terrain in hills.
    pub fn get_hills_amount(&self) -> f64 {
        (1.0 + self.mountains_amount) / 2.0
    }

    /// Scaling to apply to the base continent elevations, in planetary
    /// elevation units.
    pub fn get_continent_height_scale(&self) -> f64 {
        (1.0 - self.sea_level) / 4.0
    }

    /// Scale and bias converting planetary elevation units to meters, like the final
    /// planet subgroup of libnoise's complex planet example: `value.mul_add(scale, bias)`.
    pub fn get_elevation_scale_bias(&self) -> (f64, f64) {
        let scale = (self.max_elevation - self.min_elevation) / 2.0;
        let bias = self.min_elevation + ((self.max_elevation - self.min_elevation) / 2.0);
        (scale, bias)
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::hex_map::{
        coordinates::{CubeCoordsScalar, OffsetCoords},
        shape::RectangularHexMapShape,
    };

    use super::{MapGeneratorSeedScalar, MapGeneratorSettings};
    use crate::map::MapGenerator;

    #[test]
    fn test_default_settings_have_a_valid_map_size() {
        let settings = MapGeneratorSettings::default();
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        assert_eq!(shape.tiles_count(), 80 * 52);
    }

    #[test]
    #[should_panic(expected = "columns > 0 && rows > 0")]
    fn test_settings_reject_an_empty_map() {
        MapGeneratorSettings::new(0, 80, 0);
    }

    #[test]
    fn test_settings_take_an_ocean_coverage() {
        let settings = MapGeneratorSettings::new(0, 80, 52).with_ocean_coverage(0.25);
        assert_eq!(settings.get_ocean_coverage(), 0.25);
        assert_eq!(settings.get_seed(), 0);
    }

    #[test]
    #[should_panic(expected = "0.0 <= ocean_coverage <= 1.0")]
    fn test_settings_reject_an_ocean_coverage_out_of_bounds() {
        MapGeneratorSettings::default().with_ocean_coverage(1.5);
    }

    /// Share of the map covered in land, and share covered by its biggest landmass, on a map
    /// generated with the default settings. Landmasses wrap around east-west, like the map.
    fn land_shares(seed: MapGeneratorSeedScalar) -> (f64, f64) {
        let (columns, rows) = (40, 26);
        let shape = RectangularHexMapShape::new(columns, rows);
        let settings = MapGeneratorSettings::new(seed, columns, rows);
        let (scale, bias) = settings.get_elevation_scale_bias();
        let sea_level = settings.get_sea_level().mul_add(scale, bias);
        let map = MapGenerator::from_settings(settings).generate(|_| {});
        let index =
            |coords: OffsetCoords| coords.row() as usize * columns as usize + coords.col() as usize;
        let land: Vec<bool> = shape
            .coords()
            .map(|coords| {
                let elevation = map.tile(&coords).expect("missing tile").elevation();
                f64::from(elevation) >= sea_level
            })
            .collect();

        // flood fill each landmass, odd rows being shoved half a hex to the east
        let mut seen = vec![false; land.len()];
        let mut biggest = 0;
        for start in shape.coords().map(OffsetCoords::from) {
            if seen[index(start)] || !land[index(start)] {
                continue;
            }
            seen[index(start)] = true;
            let mut stack = vec![start];
            let mut size = 0;
            while let Some(coords) = stack.pop() {
                size += 1;
                let odd = (coords.row() & 1) as CubeCoordsScalar;
                let neighbors = [
                    (-1, 0),
                    (1, 0),
                    (odd - 1, -1),
                    (odd, -1),
                    (odd - 1, 1),
                    (odd, 1),
                ];
                for (columns_offset, rows_offset) in neighbors {
                    let row = coords.row() + rows_offset;
                    if row < 0 || row >= rows {
                        continue;
                    }
                    let col = (coords.col() + columns_offset).rem_euclid(columns);
                    let neighbor = OffsetCoords::new(col, row);
                    if !seen[index(neighbor)] && land[index(neighbor)] {
                        seen[index(neighbor)] = true;
                        stack.push(neighbor);
                    }
                }
            }
            biggest = biggest.max(size);
        }

        let tiles = shape.tiles_count() as f64;
        (
            land.iter().filter(|&&land| land).count() as f64 / tiles,
            biggest as f64 / tiles,
        )
    }

    #[test]
    fn test_default_settings_generate_earth_like_maps() {
        let settings = MapGeneratorSettings::default();
        for seed in [0, 1, 2, 3] {
            let (land, biggest) = land_shares(seed);
            // the seas cover the wanted share of every map, whatever the seed
            let expected_land = 1.0 - settings.get_ocean_coverage();
            assert!(
                (land - expected_land).abs() < 0.01,
                "seed {seed}: land covers {:.1}% of the map",
                land * 100.0
            );
            // and one huge continent holds a good third of that land
            assert!(
                biggest >= 0.10,
                "seed {seed}: the biggest landmass covers {:.0}% of the map",
                biggest * 100.0
            );
        }
    }
}
