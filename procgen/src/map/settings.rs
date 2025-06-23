pub type MapGeneratorSeedScalar = u32;

/// Procedural map generation settings.
#[derive(Debug)]
pub struct MapGeneratorSettings {
    seed: MapGeneratorSeedScalar,
    width: usize,
    height: usize,
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
    /// Determines the amount of hilly terrain that appears on the planet.
    /// Values range from 0.0 (no hills) to 1.0 (all terrain is covered in
    /// hills). This value must be less than `MOUNTAINS_AMOUNT`. Because the
    /// mountains terrain will overlap parts of the hilly terrain, and the
    /// badlands terrain may overlap parts of the hilly terrain, setting
    /// `HILLS_AMOUNT` to 1.0 may not completely cover the terrain in hills.
    hills_amount: f64,
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
    /// Scaling to apply to the base continent elevations, in planetary
    /// elevation units.
    continent_height_scale: f64,
    /// Maximum depth of the rivers, in planetary elevation units.
    river_depth: f64,
}

const DEFAULT_MOUNTAINS_AMOUNT: f64 = 0.5;
const DEFAULT_SEA_LEVEL: f64 = 0.0;

impl Default for MapGeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            width: 0,
            height: 0,
            continent_frequency: 1.0,
            continent_lacunarity: 2.208984375,
            mountain_lacunarity: 2.142578125,
            hills_lacunarity: 2.162109375,
            plains_lacunarity: 2.314453125,
            badlands_lacunarity: 2.212890625,
            mountains_twist: 1.0,
            hills_twist: 1.0,
            badlands_twist: 1.0,
            sea_level: DEFAULT_SEA_LEVEL,
            shelf_level: -0.375,
            mountains_amount: DEFAULT_MOUNTAINS_AMOUNT,
            hills_amount: (1.0 + DEFAULT_MOUNTAINS_AMOUNT) / 2.0,
            badlands_amount: 0.3125,
            terrain_offset: 1.0,
            mountain_glaciation: 1.375,
            continent_height_scale: (1.0 - DEFAULT_SEA_LEVEL) / 4.0,
            river_depth: 0.0234375,
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
    settings_scalar_accessors! {
        get_seed: seed, MapGeneratorSeedScalar,
        get_width: width, usize,
        get_height: height, usize,
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
        get_hills_amount: hills_amount, f64,
        get_badlands_amount: badlands_amount, f64,
        get_terrain_offset: terrain_offset, f64,
        get_mountain_glaciation: mountain_glaciation, f64,
        get_continent_height_scale: continent_height_scale, f64,
        get_river_depth: river_depth, f64,
    }
}
