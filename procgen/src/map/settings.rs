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
    sea_level: f64,
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
            columns: 0,
            rows: 0,
            continent_frequency: 1.0,
            continent_lacunarity: 2.208984375,
            mountain_lacunarity: 2.142578125,
            hills_lacunarity: 2.162109375,
            plains_lacunarity: 2.314453125,
            badlands_lacunarity: 2.212890625,
            mountains_twist: 1.0,
            hills_twist: 1.0,
            badlands_twist: 1.0,
            sea_level: 0.0,
            shelf_level: -0.375,
            mountains_amount: 0.5,
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
        Self {
            seed,
            columns,
            rows,
            ..Self::default()
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
}
