use project_x_core::hex_map::coordinates::CubeCoordsScalar;

use super::coastline::OpenPoles;

pub type MapGeneratorSeedScalar = u32;

/// Highest supported seed.
///
/// The noise functions derive their own seeds by adding offsets to it (up to +145),
/// without overflow checks.
pub const MAX_SEED: MapGeneratorSeedScalar = MapGeneratorSeedScalar::MAX - 256;

/// Procedural map generation settings.
#[derive(Clone, Debug)]
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
    /// How much the lowlands flatten, in planetary elevation units: 0.0 keeps the relief of the
    /// original example. Applied to the sampled elevations above the water line like the water
    /// line itself, never below it: the seas keep their depths and their share of the map.
    ///
    /// A height `h` above the water line becomes `h² / (h + flattening)`: the plateaus sink
    /// towards the plains while a peak keeps its height but for the flattening itself, so the
    /// mountain ranges survive what the hills do not.
    lowland_flattening: f64,
    /// Width in tiles of the shallow shelf along the coasts: 0 keeps the sea floor as sampled.
    ///
    /// A tile is hundreds of kilometers across, so a real continental shelf is narrower than one
    /// and the sampled depth one tile off the coast is the abyss beyond it - 2.6 km at the median.
    /// Within this many tiles of land, depths are compressed into the shallow band instead,
    /// keeping their order. Applied to the sampled elevations like the water line.
    coastal_shelf_width: u32,
    /// How much the coastline follows the smoothed field rather than the sampled one, from 0.0
    /// (the noise's own coast, every ripple of it) to 1.0 (the smoothed field's coast). Applied to
    /// the sampled values like the water line: see `Coastline`.
    ///
    /// The noise's coast sprawls - lobes joined by necks, inlets, fringes of islets; the smoothed
    /// field's follows the lie of the land, which makes the continents whole.
    coast_smoothing: f64,
    /// How far the smoothing reaches: passes of averaging each value with its neighbors', each
    /// pass spreading it one tile further.
    coast_smoothing_passes: u32,
    /// Half the width in tiles of the narrowest land the coastline keeps: necks and fringes up to
    /// twice as wide are cut, which parts the lobes a neck was joining. 0 keeps them all.
    isthmus_cut: u32,
    /// The biggest share of the land one continent may hold, from 0.0 to 1.0: a bigger one is cut
    /// at its waist, if it has one. 1.0 leaves the continents whole.
    continent_max_share: f64,
    /// The least share of the land a continent holds: a landmass too big for an island (5% of the
    /// land) but under this share is worn down to an island. 0.05 or less leaves them all.
    continent_min_share: f64,
    /// How much of its convex hull each continent's land covers at least, from 0.0 to 1.0: a
    /// continent short of it grows into the sea of its hull, the bays between its arms first
    /// filling. 0.0 leaves the continents as they are.
    continent_fill: f64,
    /// How much each step into a continent's convex hull lifts a tile in the order the sea of the
    /// hull fills, in planetary elevation units: the depths of the bays first, the edge of the
    /// hull last, so that the coast keeps the lie of the land rather than the hull's straight
    /// edges. 0.0 fills the highest ground first, wherever it lies.
    bay_depth_lift: f64,
    /// The share of the coast traded with the sea, from 0.0 to 1.0: for each coastal tile in
    /// that share, a tile of sea where the ground stands above its surroundings joins the land -
    /// a cape - and a tile of land where it lies below them sinks - a cove. Any roughness at all
    /// also breaks the runs of coast left straight, a cape and a cove at a time. 0.0 leaves the
    /// coasts as smooth, and as straight, as the continents come out.
    coast_roughness: f64,
    /// Chance that land may reach a pole, drawn for each pole from the seed: see `OpenPoles`.
    /// Otherwise the pole is open sea, and the continents are kept off its polar rows.
    polar_land_chance: f64,
    /// Absolute latitude, from 0.0 (the equator) to 1.0 (the poles), from which a closed pole
    /// starts pulling the land under the sea: the continents round off before it, rather than
    /// end along its polar rows.
    polar_ocean_latitude: f64,
    /// How far a closed pole pulls the land under at the pole itself, in planetary elevation
    /// units.
    polar_ocean_pull: f64,
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
    /// Tuned against the `WorldSurvey` - the planet's Earth-like criteria and its settlement's,
    /// half and half - jointly with the nations' and the deposits' defaults: a coordinate descent
    /// over every setting the survey can feel, every mandatory criterion held on every seed, then
    /// checked on 80 seeds it never saw. There the terrain meets about 72% of its criteria,
    /// against 70% for the planet-only tuning before. The coastline's settings came later, against
    /// the same survey with its continent criteria reworked: the world's score up from about 83%
    /// to 87%. A coordinate descent over every map setting then, on 64 seeds, kept only what held
    /// on 144 others and 64 more it never saw - the relief's - every mandatory criterion met on
    /// all of them. The continents' fill came next, with the continent frequency and the coast
    /// smoothing tuned alongside, then the coasts' own detail - the depth of the bays, the
    /// roughness - against the survey's straight coast criteria: each checked on 144 unseen
    /// seeds, at 80 x 52 and at 60 x 40. Where a value strays from the original example, the
    /// comment says why;
    /// `view_survey` shows the criteria one by one.
    fn default() -> Self {
        Self {
            seed: 0,
            columns: 80,
            rows: 52,
            // a little under the original example's 1.0 - a few big continents with open ocean
            // in between - for the coastline to fill in
            continent_frequency: 0.95,
            // below the example's "close to 2.0": the continent noise's octaves sit closer
            // together, which rounds the continents off - more interior, fewer fringes - and
            // scored best on all three seed sets pooled (1.7 against 1.8 and 1.6)
            continent_lacunarity: 1.7,
            mountain_lacunarity: 1.8,
            hills_lacunarity: 2.0,
            // with the hills' twist and the relief below, retuned with the coastline: the land it
            // moves off the poles and into the bays comes out low, which had cost the hills
            plains_lacunarity: 2.8,
            badlands_lacunarity: 2.212890625,
            // twistier ranges read as chains rather than knots
            mountains_twist: 1.625,
            hills_twist: 1.0,
            badlands_twist: 1.0,
            sea_level: 0.0,
            // 57.5% sea: Earth has 71%, but a game wants more land to play on. At 50% the
            // continents fuse; the noise graph on its own leaves 33% to 60% of the map
            // underwater, depending on the seed
            ocean_coverage: 0.575,
            // with the mountains below, the land outside the polar rows comes out mostly plain,
            // a quarter hills and a few percent mountains - the continents' plateaus were
            // reading as hills before it
            lowland_flattening: 0.09375,
            // one tile of shallow water along every coast, like Earth's shelves at this scale
            coastal_shelf_width: 1,
            // the coastline: 2 or 3 big, compact continents with the sea all around them (user
            // decision, 2026-11-10), checked on 128 seeds - see `Coastline`
            coast_smoothing: 1.0,
            coast_smoothing_passes: 4,
            isthmus_cut: 2,
            continent_max_share: 0.6,
            // over the 10% first tried: the fourth and fifth continents went, for the same score
            continent_min_share: 0.15,
            // whole continents rather than crescents and stars: well over the 75% the survey asks
            // of the least filled one, for the roughness below to take its share
            continent_fill: 0.85,
            // coasts the sea drew, not a ruler. More than any land stands above the sea: the bays
            // fill from their depths, and the coast stops short of the hull's straight edges
            bay_depth_lift: 0.5,
            // capes and coves along one coastal tile in seven, on top of the straight runs the
            // coastline breaks anyway: from 0.05 to 0.2 the survey cannot tell, more frays the
            // continents
            coast_roughness: 0.15,
            // land on a pole now and then, as Earth has Antarctica: one planet in four for each
            // pole (user decision, 2026-11-10)
            polar_land_chance: 0.25,
            polar_ocean_latitude: 0.7,
            polar_ocean_pull: 0.4,
            shelf_level: -0.5,
            // 0.675 rather than the original example's 0.5: raising the water line sinks the
            // land by about 800 m and the flattening above takes the plateaus down with the
            // ranges, so the ranges need a little more to start from
            mountains_amount: 0.675,
            // no badlands: their broken relief walled the plains in
            badlands_amount: 0.0,
            // rough terrain only where the land is high: the lowlands stay plains, the ranges
            // gather on the high ground
            terrain_offset: 1.25,
            mountain_glaciation: 1.5,
            river_depth: 0.0234375,
            // left at the example's range: it scales every elevation and every band of the
            // classification alike, so it relabels the meters without changing the planet, and
            // the bands in meters are mirrored outside this crate
            min_elevation: -8192.0,
            max_elevation: 8192.0,
        }
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

    pub fn with_seed(self, seed: MapGeneratorSeedScalar) -> Self {
        assert!(seed <= MAX_SEED, "seed <= MAX_SEED");
        Self { seed, ..self }
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

    /// How much the lowlands flatten, in planetary elevation units: 0.0 keeps the example's
    /// relief.
    pub fn with_lowland_flattening(self, lowland_flattening: f64) -> Self {
        assert!(lowland_flattening >= 0.0, "lowland_flattening >= 0.0");
        Self {
            lowland_flattening,
            ..self
        }
    }

    /// Width in tiles of the shallow shelf along the coasts: 0 keeps the sea floor as sampled.
    pub fn with_coastal_shelf_width(self, coastal_shelf_width: u32) -> Self {
        Self {
            coastal_shelf_width,
            ..self
        }
    }

    /// How much the coastline follows the smoothed field rather than the sampled one, from 0.0 to
    /// 1.0, and how far the smoothing reaches, in passes of one tile each.
    pub fn with_coast_smoothing(self, coast_smoothing: f64, coast_smoothing_passes: u32) -> Self {
        assert!(
            (0.0..=1.0).contains(&coast_smoothing),
            "0.0 <= coast_smoothing <= 1.0"
        );
        Self {
            coast_smoothing,
            coast_smoothing_passes,
            ..self
        }
    }

    /// Half the width in tiles of the narrowest land the coastline keeps: 0 keeps it all.
    pub fn with_isthmus_cut(self, isthmus_cut: u32) -> Self {
        Self {
            isthmus_cut,
            ..self
        }
    }

    /// Chance that land may reach each pole, from 0.0 (never) to 1.0 (always, as sampled).
    pub fn with_polar_land_chance(self, polar_land_chance: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&polar_land_chance),
            "0.0 <= polar_land_chance <= 1.0"
        );
        Self {
            polar_land_chance,
            ..self
        }
    }

    /// The biggest share of the land one continent may hold before it is cut at its waist: 1.0
    /// leaves the continents whole.
    pub fn with_continent_max_share(self, continent_max_share: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&continent_max_share),
            "0.0 <= continent_max_share <= 1.0"
        );
        Self {
            continent_max_share,
            ..self
        }
    }

    /// The least share of the land a continent holds before it is worn down to an island: 0.05 or
    /// less leaves them all.
    pub fn with_continent_min_share(self, continent_min_share: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&continent_min_share),
            "0.0 <= continent_min_share <= 1.0"
        );
        Self {
            continent_min_share,
            ..self
        }
    }

    /// How much of its convex hull each continent's land covers at least: 0.0 leaves them as
    /// they are.
    pub fn with_continent_fill(self, continent_fill: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&continent_fill),
            "0.0 <= continent_fill <= 1.0"
        );
        Self {
            continent_fill,
            ..self
        }
    }

    /// How much each step into a continent's hull lifts a tile in the order its bays fill.
    pub fn with_bay_depth_lift(self, bay_depth_lift: f64) -> Self {
        assert!(bay_depth_lift >= 0.0, "0.0 <= bay_depth_lift");
        Self {
            bay_depth_lift,
            ..self
        }
    }

    /// The share of the coast traded with the sea for capes and coves: 0.0 leaves it smooth.
    pub fn with_coast_roughness(self, coast_roughness: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&coast_roughness),
            "0.0 <= coast_roughness <= 1.0"
        );
        Self {
            coast_roughness,
            ..self
        }
    }

    /// The coastline as the noise draws it: no smoothing, no isthmus cut, every continent whole,
    /// kept and as filled in as it comes, its coasts left alone, both poles open. The continents
    /// still keep their straits.
    pub fn with_sampled_coastline(self) -> Self {
        self.with_coast_smoothing(0.0, 0)
            .with_isthmus_cut(0)
            .with_continent_max_share(1.0)
            .with_continent_min_share(0.0)
            .with_continent_fill(0.0)
            .with_coast_roughness(0.0)
            .with_polar_land_chance(1.0)
    }

    /// Which poles land may reach on this planet, drawn from its seed.
    pub fn open_poles(&self) -> OpenPoles {
        OpenPoles::draw(u64::from(self.seed), self.polar_land_chance)
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
        get_lowland_flattening: lowland_flattening, f64,
        get_coastal_shelf_width: coastal_shelf_width, u32,
        get_coast_smoothing: coast_smoothing, f64,
        get_coast_smoothing_passes: coast_smoothing_passes, u32,
        get_isthmus_cut: isthmus_cut, u32,
        get_continent_max_share: continent_max_share, f64,
        get_continent_min_share: continent_min_share, f64,
        get_continent_fill: continent_fill, f64,
        get_bay_depth_lift: bay_depth_lift, f64,
        get_coast_roughness: coast_roughness, f64,
        get_polar_land_chance: polar_land_chance, f64,
        get_polar_ocean_latitude: polar_ocean_latitude, f64,
        get_polar_ocean_pull: polar_ocean_pull, f64,
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
    use crate::map::{MapGenerator, PlanetSurface, PlanetSurvey, TerrainClassifier, TerrainKind};

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

    #[test]
    fn test_settings_take_a_lowland_flattening() {
        let settings = MapGeneratorSettings::default().with_lowland_flattening(0.125);
        assert_eq!(settings.get_lowland_flattening(), 0.125);
    }

    #[test]
    #[should_panic(expected = "lowland_flattening >= 0.0")]
    fn test_settings_reject_a_negative_flattening() {
        MapGeneratorSettings::default().with_lowland_flattening(-0.1);
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

    /// Plains, hills and mountains as shares of the land outside the polar rows, on a map
    /// generated with the default settings.
    fn land_mix(seed: MapGeneratorSeedScalar) -> (f64, f64, f64) {
        let (columns, rows) = (40, 26);
        let settings = MapGeneratorSettings::new(seed, columns, rows);
        let map = MapGenerator::from_settings(MapGeneratorSettings::new(seed, columns, rows))
            .generate(|_| {});
        let classifier = TerrainClassifier::new(&settings);
        let (mut plains, mut hills, mut mountains) = (0, 0, 0);
        for coords in RectangularHexMapShape::new(columns, rows).coords() {
            let elevation = map.tile(&coords).expect("missing tile").elevation();
            match classifier.classify(coords.into(), elevation) {
                TerrainKind::Flat => plains += 1,
                TerrainKind::Hills => hills += 1,
                TerrainKind::Mountains => mountains += 1,
                _ => {}
            }
        }
        let land = f64::from(plains + hills + mountains);
        (
            f64::from(plains) / land,
            f64::from(hills) / land,
            f64::from(mountains) / land,
        )
    }

    #[test]
    fn test_default_settings_survey_as_a_few_continents_in_one_ocean() {
        for seed in [0, 1, 2, 3] {
            let (columns, rows) = (40, 26);
            let settings = MapGeneratorSettings::new(seed, columns, rows);
            let map = MapGenerator::from_settings(MapGeneratorSettings::new(seed, columns, rows))
                .generate(|_| {});
            let survey = PlanetSurvey::of(
                &PlanetSurface::classify(&map, &settings),
                settings.open_poles(),
            );
            // the loose ends of the survey's own ranges: what a retuning must not break - a map
            // this small samples the planet coarsely enough to merge two continents into one
            assert!(
                (1..=6).contains(&survey.continents),
                "seed {seed}: {} continents",
                survey.continents
            );
            assert!(
                survey.top_three_share >= 0.7,
                "seed {seed}: the top three continents hold {:.0}% of the land",
                survey.top_three_share * 100.0
            );
            // the polar sea stays open, and the coasts are shelved
            assert_eq!(
                survey.polar_sea_ice_share, 0.0,
                "seed {seed}: ice over the polar sea"
            );
            assert!(
                survey.shallow_fringe_share >= 0.7,
                "seed {seed}: {:.0}% of the coastal water is shallow",
                survey.shallow_fringe_share * 100.0
            );
            assert!(
                survey.world_ocean_share >= 0.75,
                "seed {seed}: the world ocean holds {:.0}% of the water",
                survey.world_ocean_share * 100.0
            );
        }
    }

    #[test]
    fn test_default_settings_leave_most_of_the_land_flat() {
        for seed in [0, 1, 2, 3] {
            let (plains, hills, mountains) = land_mix(seed);
            let shares = format!(
                "seed {seed}: plains {:.0}%, hills {:.0}%, mountains {:.0}%",
                plains * 100.0,
                hills * 100.0,
                mountains * 100.0
            );
            // a continent is mostly plain, with hills in belts and a few real ranges
            assert!(plains >= 0.6, "{shares}");
            assert!(hills <= 0.32, "{shares}");
            assert!(mountains > 0.0 && mountains <= 0.15, "{shares}");
        }
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
