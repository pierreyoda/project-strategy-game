//! Forests generation: which land is wooded, from the climate a classified surface implies.
//!
//! No noise graph and no seed of its own: a forest grows where a tile is wet and warm enough, and
//! the planet's own elevations break the stands up. A layer over the terrain rather than a kind of
//! terrain - a wooded plain is still a plain to every procgen rule - so nothing downstream changes:
//! - **rain** follows the climate bands - the rainy equator, the dry subtropics, the wet
//!   mid-latitudes - and comes in on the prevailing winds: the trade winds and the polar
//!   easterlies blow west, the westerlies east. The air takes up water over the sea, then rains
//!   it out over land: a little on every plain, much more on the slopes of hills and ranges, so
//!   the windward coasts and slopes are wet and the lands in the lee of a range lie in its rain
//!   shadow. Far inland along the wind, the air runs dry;
//! - **cold** stops the trees past the polar treeline, and below it, above an altitude that
//!   drops towards the poles;
//! - the wettest of the land eligible becomes forest, `FOREST_SHARE` of it, like the water line
//!   takes the deepest `ocean_coverage` of the map;
//! - the ground's own variety breaks the stands up twice over: in patches, pooled over
//!   `PATCH_RADIUS` hexes, then in clearings, tile by tile.
//!
//! Tuned against the survey's forest criteria (G): fewer forests than bare plains, in stands of a
//! few to a few dozen tiles, no likelier on a coast than inland, and sparse in the rain shadows.
//!
//! `ForestParams` gathers what tuning moves; `Forests::grow` uses its defaults.

use project_x_core::{game::state::TileIndex, hex_map::vegetation::Vegetation};

use crate::{
    deposits::Geology,
    map::{PlanetSurface, TerrainKind},
};

/// Share of the land the forests cover, at most: under Earth's 31%, so that the open plains
/// outnumber the forests.
pub const FOREST_SHARE: f64 = 0.24;
/// No tree beyond this absolute latitude, from 0.0 at the equator to 1.0 at the poles: about 70°.
pub const TREELINE_LATITUDE: f64 = 0.78;
/// The altitude of the treeline at the equator, in meters; it drops to the ground at
/// `TREELINE_LATITUDE`.
pub const TREELINE_ELEVATION: f64 = 3000.0;
/// The westerlies blow between these absolute latitudes, about 30° and 60°: the trade winds
/// equatorwards, the polar easterlies polewards.
pub const WESTERLIES: (f64, f64) = (0.33, 0.67);
/// Moisture the air takes up over each hex of water, up to saturation at 1.0: a lake recharges
/// it a little, an ocean fully.
pub const SEA_UPTAKE: f64 = 0.5;
/// Share of its moisture the air keeps over a hex of plain, of hills, of mountains: what it loses
/// is the rain that falls there.
pub const PLAINS_KEEP: f64 = 0.94;
pub const HILLS_KEEP: f64 = 0.75;
pub const MOUNTAINS_KEEP: f64 = 0.4;
/// How much of the rain a tile gets comes with the wind, from 0.0 (the climate band alone) to 1.0
/// (none without it).
pub const WIND_WEIGHT: f64 = 0.75;
/// How much the ground's own variety moves a tile's wetness, from 0.0 (not at all) up: what breaks
/// the climate into stands.
pub const PATCHINESS: f64 = 0.4;
/// Radius of the neighborhood whose variety is pooled into patches.
pub const PATCH_RADIUS: u32 = 0;
/// How much a tile's own variety, unpooled, moves its wetness: what opens clearings in the stands.
/// None by default: the clearings broke the stands into specks.
pub const CLEARINGS: f64 = 0.0;

/// What tuning moves in the forests' growth. The defaults are the constants above.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ForestParams {
    /// See `FOREST_SHARE`.
    pub share: f64,
    /// See `SEA_UPTAKE`.
    pub sea_uptake: f64,
    /// See `PLAINS_KEEP` and its siblings.
    pub plains_keep: f64,
    pub hills_keep: f64,
    pub mountains_keep: f64,
    /// See `WIND_WEIGHT`.
    pub wind_weight: f64,
    /// See `PATCHINESS`.
    pub patchiness: f64,
    /// See `PATCH_RADIUS`.
    pub patch_radius: u32,
    /// See `CLEARINGS`.
    pub clearings: f64,
}

impl Default for ForestParams {
    fn default() -> Self {
        Self {
            share: FOREST_SHARE,
            sea_uptake: SEA_UPTAKE,
            plains_keep: PLAINS_KEEP,
            hills_keep: HILLS_KEEP,
            mountains_keep: MOUNTAINS_KEEP,
            wind_weight: WIND_WEIGHT,
            patchiness: PATCHINESS,
            patch_radius: PATCH_RADIUS,
            clearings: CLEARINGS,
        }
    }
}

/// Rain by absolute latitude, from the large-scale circulation alone, interpolated between these
/// points.
const BAND_RAIN: [(f64, f64); 9] = [
    (0.0, 1.0),
    (0.12, 0.9),
    (0.22, 0.65),
    (0.3, 0.45),
    (0.39, 0.65),
    (0.45, 0.85),
    (0.6, 0.85),
    (0.7, 0.75),
    (TREELINE_LATITUDE, 0.3),
];

/// The wooded tiles of a planet.
#[derive(Clone, Debug, PartialEq)]
pub struct Forests {
    /// Row-major.
    wooded: Vec<bool>,
}

impl Forests {
    pub fn grow(surface: &PlanetSurface) -> Self {
        Self::grow_with(surface, &ForestParams::default())
    }

    pub fn grow_with(surface: &PlanetSurface, params: &ForestParams) -> Self {
        let tiles = surface.tiles_count();
        let rain = wind_rain(surface, params);
        let variety: Vec<f64> = (0..tiles as TileIndex)
            .map(|index| variety(surface, index))
            .collect();
        let scores: Vec<f64> = (0..tiles as TileIndex)
            .map(|index| {
                let wetness = wetness(surface, index, rain[index as usize], params.wind_weight);
                if wetness <= 0.0 {
                    return 0.0;
                }
                let around = surface.within(index, params.patch_radius);
                let pooled = around
                    .iter()
                    .map(|&(tile, _)| variety[tile as usize])
                    .sum::<f64>()
                    / around.len() as f64;
                let own = variety[index as usize];
                wetness
                    * (1.0 + params.patchiness * (pooled - 0.5) * 2.0).max(0.0)
                    * (1.0 + params.clearings * (own - 0.5) * 2.0).max(0.0)
            })
            .collect();

        // the wettest of the eligible land, up to the share
        let land = (0..tiles as TileIndex)
            .filter(|&index| surface.is_land(index))
            .count();
        let mut eligible: Vec<TileIndex> = (0..tiles as TileIndex)
            .filter(|&index| scores[index as usize] > 0.0)
            .collect();
        eligible.sort_by(|&a, &b| {
            scores[b as usize]
                .total_cmp(&scores[a as usize])
                .then(a.cmp(&b))
        });
        let mut wooded = vec![false; tiles];
        let count = ((land as f64 * params.share).round() as usize).min(eligible.len());
        for &index in &eligible[..count] {
            wooded[index as usize] = true;
        }
        Self { wooded }
    }

    pub fn is_wooded(&self, index: TileIndex) -> bool {
        self.wooded[index as usize]
    }

    /// Row-major.
    pub fn wooded(&self) -> &[bool] {
        &self.wooded
    }

    /// Row-major, as the world paints it.
    pub fn vegetation(&self) -> Vec<Vegetation> {
        self.wooded
            .iter()
            .map(|&wooded| {
                if wooded {
                    Vegetation::Forest
                } else {
                    Vegetation::None
                }
            })
            .collect()
    }
}

/// The way the prevailing wind blows at an absolute latitude, along a row: `1` towards the east
/// (the westerlies), `-1` towards the west (the trade winds and the polar easterlies).
pub fn wind_towards(latitude: f64) -> i32 {
    if (WESTERLIES.0..WESTERLIES.1).contains(&latitude) {
        1
    } else {
        -1
    }
}

/// The rain the wind brings each tile, from 0.0 up, row-major: the air's moisture as it arrives,
/// times what the tile wrings out of it - more on a slope than on a plain - averaged with the
/// tile's neighbors on land, so that the rows the wind follows do not show.
pub fn wind_rain(surface: &PlanetSurface, params: &ForestParams) -> Vec<f64> {
    let shape = surface.shape();
    let (columns, rows) = (shape.columns() as usize, shape.rows() as usize);
    let mut rain = vec![0.0; surface.tiles_count()];
    for row in 0..rows {
        let towards = wind_towards(surface.latitude((row * columns) as TileIndex).abs());
        let column = |step: usize| {
            if towards > 0 {
                step % columns
            } else {
                columns - 1 - step % columns
            }
        };
        // around the world twice: the first lap only brings the air to its steady state
        let mut moisture: f64 = 0.5;
        for step in 0..2 * columns {
            let index = row * columns + column(step);
            let tile = index as TileIndex;
            if !surface.is_land(tile) {
                moisture = (moisture + params.sea_uptake).min(1.0);
                continue;
            }
            let keep = match surface.terrain_at(tile) {
                TerrainKind::Mountains | TerrainKind::RoughIce => params.mountains_keep,
                TerrainKind::Hills => params.hills_keep,
                _ => params.plains_keep,
            };
            if step >= columns {
                rain[index] = moisture * (1.0 - keep) / (1.0 - params.plains_keep).max(1e-6);
            }
            moisture *= keep;
        }
    }
    (0..surface.tiles_count() as TileIndex)
        .map(|index| {
            // over land only: the sea gets no rain of its own to lend the coast
            let around: Vec<f64> = surface
                .within(index, 1)
                .into_iter()
                .filter(|&(tile, _)| surface.is_land(tile))
                .map(|(tile, _)| rain[tile as usize])
                .collect();
            if around.is_empty() {
                0.0
            } else {
                around.iter().sum::<f64>() / around.len() as f64
            }
        })
        .collect()
}

/// How wet a tile is for trees, from 0.0 (no tree can grow) up: the climate band's rain, made
/// wetter or drier by what the wind brings - `wind_weight` of it - and none on mountains, ice,
/// water or past the treeline.
pub fn wetness(surface: &PlanetSurface, index: TileIndex, wind_rain: f64, wind_weight: f64) -> f64 {
    if !surface.is_land(index)
        || !matches!(
            surface.terrain_at(index),
            TerrainKind::Flat | TerrainKind::Hills
        )
    {
        return 0.0;
    }
    let latitude = surface.latitude(index).abs();
    if latitude >= TREELINE_LATITUDE {
        return 0.0;
    }
    let treeline = TREELINE_ELEVATION * (1.0 - latitude / TREELINE_LATITUDE);
    if f64::from(surface.elevation_at(index)) > treeline {
        return 0.0;
    }
    let band = BAND_RAIN
        .windows(2)
        .find(|pair| latitude <= pair[1].0)
        .map_or(0.0, |pair| {
            let ((from, low), (to, high)) = (pair[0], pair[1]);
            low + (high - low) * (latitude - from) / (to - from)
        });
    band * (1.0 - wind_weight + wind_weight * wind_rain.min(2.0))
}

/// From 0.0 to 1.0, a stand-in for what the climate does not tell - soil, slope, fire: a hash of
/// the tile and of its elevation, which differs from one planet to the next.
fn variety(surface: &PlanetSurface, index: TileIndex) -> f64 {
    // splitmix64
    let mut z = (u64::from(index) << 16 | u64::from(surface.elevation_at(index) as u16))
        .wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Whether the tile lies in the subtropical dry belt, where the deserts are.
pub fn is_dry(surface: &PlanetSurface, index: TileIndex) -> bool {
    Geology::is_dry(surface, index)
}

#[cfg(test)]
mod tests {
    use super::{
        ForestParams, Forests, TREELINE_LATITUDE, WIND_WEIGHT, wetness, wind_rain, wind_towards,
    };
    use crate::{deposits::tests::planet, map::surface::tests::surface_from_art};

    #[test]
    fn test_no_forest_on_mountains_ice_water_or_past_the_treeline() {
        let surface = surface_from_art(&[".^*~"; 21].join("|"));
        for row in 0..21u32 {
            let index = row * 4;
            assert_eq!(
                wetness(&surface, index + 1, 1.0, WIND_WEIGHT),
                0.0,
                "mountains"
            );
            assert_eq!(wetness(&surface, index + 2, 1.0, WIND_WEIGHT), 0.0, "ice");
            assert_eq!(wetness(&surface, index + 3, 1.0, WIND_WEIGHT), 0.0, "water");
            let past_treeline = surface.latitude(index).abs() >= TREELINE_LATITUDE;
            assert_eq!(
                wetness(&surface, index, 1.0, WIND_WEIGHT) == 0.0,
                past_treeline,
                "row {row}"
            );
        }
    }

    #[test]
    fn test_the_climate_bands_shun_the_subtropics() {
        // 21 rows: latitude steps of 0.1
        let surface = surface_from_art(&["...."; 21].join("|"));
        let at = |row: u32| wetness(&surface, row * 4, 1.0, WIND_WEIGHT);
        assert!(at(10) > at(13), "the equator over the subtropics");
        assert!(at(15) > at(13), "the temperate band over the subtropics");
    }

    #[test]
    fn test_the_winds_blow_west_in_the_tropics_and_east_in_the_mid_latitudes() {
        assert_eq!(wind_towards(0.1), -1);
        assert_eq!(wind_towards(0.5), 1);
        assert_eq!(wind_towards(0.8), -1);
    }

    #[test]
    fn test_the_wind_rains_on_the_coast_it_comes_from_and_on_the_slopes() {
        // an equatorial strip of land: the trade wind blows west, off the ocean to the east
        let surface = surface_from_art(
            &[
                "~~~~~~~~~~~~~~~~~~~~~~",
                "~~~..........:^.....~~",
                "~~~~~~~~~~~~~~~~~~~~~~",
            ]
            .join("|"),
        );
        let rain = wind_rain(&surface, &ForestParams::default());
        let at = |col: u32| rain[(22 + col) as usize];
        assert!(at(18) > at(16), "the windward coast over inland");
        assert!(at(14) > at(16), "the range wrings the air out");
        assert!(at(10) < at(16), "the rain shadow behind the range");
    }

    #[test]
    fn test_forests_cover_their_share_of_a_planet() {
        let surface = planet(3, 80, 52);
        let forests = Forests::grow(&surface);
        let land = (0..surface.tiles_count() as u32)
            .filter(|&index| surface.is_land(index))
            .count();
        let wooded = forests.wooded().iter().filter(|&&wooded| wooded).count();
        let share = wooded as f64 / land as f64;
        assert!(
            (0.15..=super::FOREST_SHARE + 0.001).contains(&share),
            "{share}"
        );
        assert!(
            (0..surface.tiles_count() as u32)
                .filter(|&index| forests.is_wooded(index))
                .all(|index| surface.is_land(index))
        );
        assert_eq!(
            forests,
            Forests::grow(&surface),
            "the same planet, the same forests"
        );
    }
}
