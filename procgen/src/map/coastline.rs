//! The coastline: which tiles of the sampled planet are land.
//!
//! Cut at its own water line, the noise graph's planet sprawls: lobes joined by necks a tile or
//! two wide, fringes of islets, land running off both poles. This pass draws the coastline on the
//! sampled values instead, after the graph like the water line, so that the continents come out
//! few, big and whole, with the sea all around them:
//! 1. **consolidated**: each value is blended with the smoothed field around it, so that the coast
//!    follows the lie of the land rather than every ripple of the noise - inlets fill, specks sink;
//! 2. **closed poles**: unless the seed opens a pole to land (`OpenPoles`), the land is pulled
//!    under the sea towards it, and none is left on its polar rows;
//! 3. **isthmuses cut**: land narrower than twice `isthmus_cut` is opened up, which parts the lobes
//!    a neck was joining and trims the thin fringes;
//! 4. **giants split**: a continent holding more than `continent_max_share` of the land is cut in
//!    two or more at its waist, where the land between its broadest parts is narrowest;
//! 5. **minor continents worn down**: a landmass too big for an island but holding less than
//!    `continent_min_share` of the land loses its coast, ring after ring, until it is an island;
//! 6. **filled in**: each continent grows into the sea of its own convex hull, the high ground
//!    and the depths of its bays first, until its land covers `continent_fill` of the hull - the
//!    bays between its arms fill, a crescent becomes a body;
//! 7. **straits**: two continents never come within `MIN_STRAIT` tiles of sea of each other, the
//!    weaker coast giving way.
//!
//! The water line of the blended field is then searched for the land to cover the share of the
//! map that `ocean_coverage` leaves it, as before, and the coasts grown by the few tiles it misses.
//!
//! Last, the coasts get back the detail the smoothing took, tile for tile so that every landmass
//! keeps its area: capes where the ground stands above its surroundings, coves where it lies
//! below, along `coast_roughness` of the coast and wherever it still runs straight. The
//! continents, the islands, the straits and the poles stay as they are; the coasts stop looking
//! drawn with a ruler.

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, VecDeque},
};

use project_x_core::{
    game::rng::{GameRng, RngPurpose},
    hex_map::{coordinates::OffsetCoords, shape::RectangularHexMapShape},
};
use rand::RngExt;

use super::{
    hull::hull_tiles,
    landmasses::CONTINENT_SHARE,
    settings::MapGeneratorSettings,
    shores::{Stretch, straight_tiles},
    terrain::polar_rows,
};

/// The least number of sea tiles between two continents: crossing from one to the other always
/// takes a sea leg.
pub const MIN_STRAIT: u32 = 2;

/// Share of the land each part of a split continent holds at least: a giant is cut at its waist,
/// never chipped at a cape.
const MIN_SPLIT_PART: f64 = 0.15;

/// What a minor continent is worn down to, as a share of `CONTINENT_SHARE`: an island, with room
/// to spare for the land the straits and the water line move afterwards.
const WORN_DOWN: f64 = 0.8;

/// The widest land eroded looking for a giant's waist, in tiles from the sea: a continent with no
/// waist narrower than twice this is left whole.
const MAX_WAIST_CUT: u32 = 8;

/// How deep into a continent's hull the depth of a bay still counts, in tiles: beyond it, the
/// highest ground fills first.
const BAY_DEPTH: u32 = 4;

/// How far a cape rises or a cove sinks from the coast as the continents come out, in tiles.
const ROUGHEN_REACH: u32 = 2;

/// The straight runs of coast the roughening breaks: a tile shorter than the survey's
/// `Stretch::RULER`, so that what is left of them stays under it. Shorter still, and half the
/// coast is a run to break: the continents fray.
const STRAIGHT_RUN: Stretch = Stretch {
    length: Stretch::RULER.length - 1.0,
    within: Stretch::RULER.within,
};

/// How many times over the roughening looks for straight runs to break.
const STRAIGHT_RUN_ROUNDS: usize = 8;

/// Which poles land may reach. Each pole is open with the chance `polar_land_chance`, drawn from
/// the seed; the coastline keeps the land off the closed ones.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OpenPoles {
    pub north: bool,
    pub south: bool,
}

impl OpenPoles {
    pub const NONE: Self = Self {
        north: false,
        south: false,
    };
    pub const BOTH: Self = Self {
        north: true,
        south: true,
    };

    /// The poles of the planet seeded with `seed`, each one open with the given chance.
    pub fn draw(seed: u64, chance: f64) -> Self {
        let mut rng = GameRng::new(seed, RngPurpose::PlanetGeneration);
        // both drawn whatever the first one gives, so that the south never depends on the north
        let north = rng.random::<f64>() < chance;
        let south = rng.random::<f64>() < chance;
        Self { north, south }
    }

    pub fn count(self) -> usize {
        usize::from(self.north) + usize::from(self.south)
    }

    /// Whether the pole of the hemisphere at the given latitude, from -1.0 (the north pole) to
    /// +1.0 (the south pole), is open.
    fn at(self, latitude: f64) -> bool {
        if latitude < 0.0 {
            self.north
        } else {
            self.south
        }
    }
}

/// The land and the sea of a sampled planet.
pub(crate) struct Coastline {
    land: Vec<bool>,
    /// How far above the coastline's water line the blended field lies: positive over the land,
    /// but where the coasts grew to make up the land, and negative over the sea, but where a
    /// closed pole, an isthmus cut or a strait sank it.
    relief: Vec<f64>,
}

impl Coastline {
    /// Draw the coastline of the given values, sampled in the order of `shape.coords()`.
    pub(crate) fn draw(
        shape: RectangularHexMapShape,
        values: &[f64],
        settings: &MapGeneratorSettings,
    ) -> Self {
        let grid = Grid::new(shape);
        let poles = settings.open_poles();
        let smoothed = grid.smooth(values, settings.get_coast_smoothing_passes());
        let weight = settings.get_coast_smoothing();
        let rows = shape.rows() as usize;
        let blended: Vec<f64> = values
            .iter()
            .zip(&smoothed)
            .enumerate()
            .map(|(index, (&value, &smoothed))| {
                let latitude = latitude(index / grid.columns, rows);
                value + weight * (smoothed - value) - polar_pull(latitude, poles, settings)
            })
            .collect();

        let reshape = Reshape {
            shape,
            grid: &grid,
            blended: &blended,
            poles,
            isthmus_cut: settings.get_isthmus_cut(),
            continent_max_share: settings.get_continent_max_share(),
            continent_min_share: settings.get_continent_min_share(),
            continent_fill: settings.get_continent_fill(),
            bay_depth_lift: settings.get_bay_depth_lift(),
        };
        // as many tiles under the sea as `ElevationField::water_line` puts there
        let tiles = values.len();
        let sea = (settings.get_ocean_coverage() * tiles as f64).round() as usize;
        let (mut land, water_line) = reshape.search(tiles - sea.min(tiles));
        if settings.get_coast_roughness() > 0.0 && settings.get_coast_smoothing_passes() > 0 {
            // what stands out of the lie of the land, short of the single tile
            let detail: Vec<f64> = grid
                .smooth(values, 1)
                .iter()
                .zip(&smoothed)
                .map(|(near, far)| near - far)
                .collect();
            reshape.roughen(&mut land, &detail, settings.get_coast_roughness());
        }
        let relief = blended.iter().map(|value| value - water_line).collect();
        Self { land, relief }
    }

    pub(crate) fn is_land(&self, index: usize) -> bool {
        self.land[index]
    }

    /// Whether each tile is land, in the order of the values.
    pub(crate) fn land(&self) -> &[bool] {
        &self.land
    }

    /// Height of a tile above the coastline's water line, in planetary elevation units: the
    /// elevation of a tile the coastline moved from the sea to the land, or back.
    pub(crate) fn relief(&self, index: usize) -> f64 {
        self.relief[index]
    }
}

/// From -1.0 (the north pole) to +1.0 (the south pole), like `PlanetSurface::latitude`.
fn latitude(row: usize, rows: usize) -> f64 {
    if rows < 2 {
        0.0
    } else {
        row as f64 / (rows - 1) as f64 * 2.0 - 1.0
    }
}

/// How far a closed pole pulls the land under at the given latitude: nothing up to
/// `polar_ocean_latitude`, then smoothly more, up to `polar_ocean_pull` at the pole itself.
fn polar_pull(latitude: f64, poles: OpenPoles, settings: &MapGeneratorSettings) -> f64 {
    let from = settings.get_polar_ocean_latitude();
    if poles.at(latitude) || from >= 1.0 {
        return 0.0;
    }
    let along = ((latitude.abs() - from) / (1.0 - from)).clamp(0.0, 1.0);
    settings.get_polar_ocean_pull() * along * along * (3.0 - 2.0 * along)
}

/// One coastline, given its water line.
struct Reshape<'a> {
    shape: RectangularHexMapShape,
    grid: &'a Grid,
    blended: &'a [f64],
    poles: OpenPoles,
    isthmus_cut: u32,
    continent_max_share: f64,
    continent_min_share: f64,
    continent_fill: f64,
    bay_depth_lift: f64,
}

impl Reshape<'_> {
    /// The land of `target` tiles, with its water line: the highest water line that leaves as much
    /// once the poles, isthmuses, giants, minor continents and straits have taken theirs, the
    /// coasts grown by the tiles it misses. The land shrinks as the water line rises, but not
    /// strictly and not smoothly - one more tile can keep a whole minor continent from being worn
    /// down - so a bisection over the blended values finds it, and the coasts make up the rest.
    fn search(&self, target: usize) -> (Vec<bool>, f64) {
        let mut levels = self.blended.to_vec();
        levels.sort_unstable_by(f64::total_cmp);
        let tiles = levels.len();
        if target == 0 || tiles == 0 {
            let above_all = levels.last().map_or(0.0, |&highest| highest + 1.0);
            return (vec![false; tiles], above_all);
        }
        let above_all = levels[tiles - 1] + 1.0;
        // `levels[step]` as the water line leaves `tiles - step` tiles above it, before the cuts;
        // past the last level, none
        let level = |step: usize| levels.get(step).copied().unwrap_or(above_all);
        let land_at = |step: usize| self.land(level(step));
        let count = |land: &[bool]| land.iter().filter(|&&land| land).count();

        let mut low_land = land_at(0);
        if count(&low_land) <= target {
            self.grow_coasts(&mut low_land, target);
            return (low_land, level(0));
        }
        // `low` leaves too much land, `high` no more than the target: from the step that would leave
        // just that before the cuts, which take land away but the hulls give some back
        let (mut low, mut high, mut high_land) = (0, tiles, vec![false; tiles]);
        let guess = tiles - target.min(tiles);
        let guess_land = land_at(guess);
        if count(&guess_land) > target {
            (low, low_land) = (guess, guess_land);
        } else {
            (high, high_land) = (guess, guess_land);
        }
        while high - low > 1 {
            let middle = (low + high) / 2;
            let land = land_at(middle);
            if count(&land) > target {
                (low, low_land) = (middle, land);
            } else {
                (high, high_land) = (middle, land);
            }
        }
        self.grow_coasts(&mut high_land, target);
        // the coasts make up what the cuts took, but for tied values no water line parts; then the
        // closer of the two
        if count(&low_land) - target <= target - count(&high_land) {
            (low_land, level(low))
        } else {
            (high_land, level(high))
        }
    }

    /// Grow the coasts of the continents until the land holds `target` tiles: the highest tile in
    /// the blended field first, the tiles in a continent's convex hull lifted by their depth in
    /// it, so that the bays come before the open coasts. A tile joins the continent it touches,
    /// but never two landmasses at once, never within `MIN_STRAIT` of another one, and never on a
    /// closed pole: the coasts move, the continents, the islands and the straits stay.
    fn grow_coasts(&self, land: &mut [bool], target: usize) {
        let grid = self.grid;
        let mut count = land.iter().filter(|&&land| land).count();
        if count >= target {
            return;
        }
        let (mut landmass, areas) = grid.landmasses(land);
        // an island grown into a continent would undo the minor continents' wearing down
        let continent: Vec<bool> = areas
            .iter()
            .map(|&area| area as f64 >= CONTINENT_SHARE * count as f64)
            .collect();
        let on_continent = |landmass: &[Option<usize>], tile: usize| {
            landmass[tile].is_some_and(|id| continent[id])
        };
        let mut in_hull = vec![false; grid.len()];
        for id in (0..areas.len()).filter(|&id| continent[id]) {
            let tiles: Vec<usize> = (0..grid.len())
                .filter(|&index| landmass[index] == Some(id))
                .collect();
            for index in hull_tiles(grid.columns, &tiles).unwrap_or_default() {
                in_hull[index] = true;
            }
        }
        let rank = self.bay_rank(&in_hull);
        let mut shore: BinaryHeap<Highest> = (0..grid.len())
            .filter(|&index| {
                !land[index]
                    && grid.neighbors[index]
                        .iter()
                        .any(|&n| on_continent(&landmass, n))
            })
            .map(|index| Highest(rank[index], index))
            .collect();
        while count < target {
            let Some(Highest(_, index)) = shore.pop() else {
                return;
            };
            if land[index] || self.closed(index / grid.columns) {
                continue;
            }
            let mut touched = grid.neighbors[index]
                .iter()
                .filter_map(|&neighbor| landmass[neighbor]);
            let Some(own) = touched.next() else {
                continue;
            };
            if !continent[own]
                || touched.any(|other| other != own)
                || grid
                    .within(index, MIN_STRAIT)
                    .into_iter()
                    .any(|other| landmass[other].is_some_and(|id| id != own))
            {
                continue;
            }
            land[index] = true;
            landmass[index] = Some(own);
            count += 1;
            for &neighbor in &grid.neighbors[index] {
                if !land[neighbor] {
                    shore.push(Highest(rank[neighbor], neighbor));
                }
            }
        }
    }

    /// The order the sea fills in: each tile's blended value, lifted by `bay_depth_lift` for each
    /// step it lies into the hull - from the hull's edge and from the closed poles - up to
    /// `BAY_DEPTH` steps.
    fn bay_rank(&self, in_hull: &[bool]) -> Vec<f64> {
        let grid = self.grid;
        let outside: Vec<bool> = (0..grid.len())
            .map(|index| !in_hull[index] || self.closed(index / grid.columns))
            .collect();
        grid.steps_from(&outside, BAY_DEPTH)
            .into_iter()
            .zip(self.blended)
            .map(|(depth, &value)| value + self.bay_depth_lift * f64::from(depth.min(BAY_DEPTH)))
            .collect()
    }

    /// Fill every continent in towards its convex hull, the highest tile in the blended field
    /// first, lifted by its depth in the hull, until its land covers `continent_fill` of the hull.
    /// A tile joins the continent it touches, inside the continent's hull, but never within
    /// `MIN_STRAIT` of another landmass and never on a closed pole: a continent may stop short of
    /// its share.
    fn fill_in(&self, land: &mut [bool]) {
        if self.continent_fill <= 0.0 {
            return;
        }
        let grid = self.grid;
        let (mut landmass, areas) = grid.landmasses(land);
        let total: usize = areas.iter().sum();
        for (id, &area) in areas.iter().enumerate() {
            if (area as f64) < CONTINENT_SHARE * total as f64 {
                continue;
            }
            let tiles: Vec<usize> = (0..grid.len())
                .filter(|&index| landmass[index] == Some(id))
                .collect();
            let Some(hull) = hull_tiles(grid.columns, &tiles) else {
                continue;
            };
            let wanted = (self.continent_fill * hull.len() as f64).ceil() as usize;
            let mut area = area;
            let mut in_hull = vec![false; grid.len()];
            for &index in &hull {
                in_hull[index] = true;
            }
            let rank = self.bay_rank(&in_hull);
            let mut shore: BinaryHeap<Highest> = hull
                .iter()
                .filter(|&&index| {
                    !land[index]
                        && grid.neighbors[index]
                            .iter()
                            .any(|&n| landmass[n] == Some(id))
                })
                .map(|&index| Highest(rank[index], index))
                .collect();
            while area < wanted {
                let Some(Highest(_, index)) = shore.pop() else {
                    break;
                };
                let too_close = || {
                    grid.within(index, MIN_STRAIT)
                        .into_iter()
                        .any(|other| landmass[other].is_some_and(|other| other != id))
                };
                if land[index] || self.closed(index / grid.columns) || too_close() {
                    continue;
                }
                land[index] = true;
                landmass[index] = Some(id);
                area += 1;
                for &neighbor in &grid.neighbors[index] {
                    if !land[neighbor] && in_hull[neighbor] {
                        shore.push(Highest(rank[neighbor], neighbor));
                    }
                }
            }
        }
    }

    /// Trade coast tiles with the sea, a cape for a cove, for `share` of the coastal tiles of each
    /// landmass: the sea tile on its shore with the most `detail` joins it, its coastal tile with
    /// the least sinks, for as long as the first stands above the second. Then the same along
    /// the runs of coast still straight, `STRAIGHT_RUN` or longer, a run at a time until none is
    /// left that a trade can break. No tile is traded twice, and none further than
    /// `ROUGHEN_REACH` from the coast as it was.
    ///
    /// A trade moves the coast and nothing else: every landmass keeps its area, none appears,
    /// goes, joins another or comes within `MIN_STRAIT` of one, no lake opens, no neck is left a
    /// tile wide, and no land rises on a closed pole.
    fn roughen(&self, land: &mut [bool], detail: &[f64], share: f64) {
        let grid = self.grid;
        let sea: Vec<bool> = land.iter().map(|&land| !land).collect();
        let to_sea = grid.steps_from(&sea, ROUGHEN_REACH);
        let to_land = grid.steps_from(land, ROUGHEN_REACH);
        let (landmass, areas) = grid.landmasses(land);
        let mut trading = Trading {
            detail,
            // within reach of the coast as it was, on either side
            open: (0..grid.len())
                .map(|index| to_sea[index].min(to_land[index]) <= ROUGHEN_REACH)
                .collect(),
            landmass,
        };
        for id in 0..areas.len() {
            let coastal = (0..grid.len())
                .filter(|&index| trading.landmass[index] == Some(id) && to_sea[index] == 1)
                .count();
            let trades = (share * coastal as f64).round() as usize;
            self.trade(land, &mut trading, id, trades, None, None);
        }
        for _ in 0..STRAIGHT_RUN_ROUNDS {
            let straight = straight_tiles(self.shape, land, STRAIGHT_RUN);
            // the tiles on a straight run, and the sea along it
            let mut along = straight.clone();
            for index in (0..grid.len()).filter(|&index| straight[index]) {
                for &neighbor in &grid.neighbors[index] {
                    along[neighbor] = true;
                }
            }
            let mut traded = 0;
            for id in 0..areas.len() {
                let on_runs = (0..grid.len())
                    .filter(|&index| straight[index] && trading.landmass[index] == Some(id))
                    .count();
                // a trade for each stretch of a run
                let trades = on_runs.div_ceil(STRAIGHT_RUN.length as usize);
                let on_run = Some(along.as_slice());
                let both = self.trade(land, &mut trading, id, trades, on_run, on_run);
                // where no cape may rise along a run - against a closed pole, across a strait -
                // its coves trade with capes anywhere else
                let coves = self.trade(land, &mut trading, id, trades - both, None, on_run);
                traded += both + coves;
            }
            if traded == 0 {
                break;
            }
        }
    }

    /// Trade up to `trades` capes for coves on one landmass, the capes and the coves each among
    /// the given tiles if any: see `roughen`. Round after round, since a trade opens the tiles
    /// behind it. Returns how many it traded.
    fn trade(
        &self,
        land: &mut [bool],
        trading: &mut Trading,
        id: usize,
        trades: usize,
        capes_among: Option<&[bool]>,
        coves_among: Option<&[bool]>,
    ) -> usize {
        let grid = self.grid;
        let detail = trading.detail;
        let mut left = trades;
        while left > 0 {
            let candidate = |index: usize, among: Option<&[bool]>| {
                trading.open[index] && among.is_none_or(|among| among[index])
            };
            let mut capes: Vec<usize> = (0..grid.len())
                .filter(|&index| candidate(index, capes_among))
                .filter(|&index| self.may_rise(land, &trading.landmass, index) == Some(id))
                .collect();
            capes.sort_unstable_by(|&a, &b| Highest(detail[b], b).cmp(&Highest(detail[a], a)));
            let mut coves: Vec<usize> = (0..grid.len())
                .filter(|&index| trading.landmass[index] == Some(id))
                .filter(|&index| candidate(index, coves_among))
                .filter(|&index| grid.may_sink(land, index))
                .collect();
            coves.sort_unstable_by(|&a, &b| detail[a].total_cmp(&detail[b]).then(a.cmp(&b)));
            let mut coves = coves.into_iter();
            let before = left;
            'capes: for cape in capes {
                // the trades before it this round may have closed it
                if left == 0 || self.may_rise(land, &trading.landmass, cape) != Some(id) {
                    continue;
                }
                land[cape] = true;
                // the first cove left that may still sink, the cape risen
                for cove in coves.by_ref() {
                    if detail[cape] <= detail[cove] {
                        break;
                    }
                    if grid.may_sink(land, cove) {
                        land[cove] = false;
                        (trading.landmass[cape], trading.landmass[cove]) = (Some(id), None);
                        (trading.open[cape], trading.open[cove]) = (false, false);
                        left -= 1;
                        continue 'capes;
                    }
                }
                // no cove for it: the cape goes back, and the round is over
                land[cape] = false;
                break;
            }
            if left == before {
                break;
            }
        }
        trades - left
    }

    /// The landmass a sea tile may join, if any: see `roughen`.
    fn may_rise(&self, land: &[bool], landmass: &[Option<usize>], index: usize) -> Option<usize> {
        let grid = self.grid;
        if land[index] || self.closed(index / grid.columns) || !grid.moves_the_coast(land, index) {
            return None;
        }
        let own = grid.neighbors[index]
            .iter()
            .find_map(|&neighbor| landmass[neighbor])?;
        let alone = !grid
            .within(index, MIN_STRAIT)
            .into_iter()
            .any(|other| landmass[other].is_some_and(|id| id != own));
        alone.then_some(own)
    }

    /// Whether a row is on the polar rows of a closed pole.
    fn closed(&self, row: usize) -> bool {
        let rows = self.grid.rows;
        let polar = polar_rows(rows as i16) as usize;
        (row < polar && !self.poles.north)
            || (row >= rows.saturating_sub(polar) && !self.poles.south)
    }

    /// The land above the given water line, once the poles, isthmuses, giants' waists, minor
    /// continents, the continents' hulls and the straits have had their way.
    fn land(&self, water_line: f64) -> Vec<bool> {
        let grid = self.grid;
        let mut land: Vec<bool> = self
            .blended
            .iter()
            .map(|&value| value >= water_line)
            .collect();
        for row in (0..grid.rows).filter(|&row| self.closed(row)) {
            land[row * grid.columns..(row + 1) * grid.columns].fill(false);
        }
        let mut land = grid.open(&land, self.isthmus_cut);
        grid.split_giants(
            &mut land,
            self.blended,
            self.continent_max_share,
            self.isthmus_cut + 1,
        );
        grid.wear_down_minor_continents(&mut land, self.continent_min_share);
        self.fill_in(&mut land);
        grid.part_continents(&mut land, self.blended);
        land
    }
}

/// The coast as `Reshape::roughen` trades it.
struct Trading<'a> {
    /// How far each tile stands above its surroundings: the capes rise where it is highest, the
    /// coves sink where it is lowest.
    detail: &'a [f64],
    /// The tiles still open to a trade.
    open: Vec<bool>,
    /// The landmass of each tile, kept up with the trades.
    landmass: Vec<Option<usize>>,
}

/// A tile by its value, the highest first; of two of a value, the first.
struct Highest(f64, usize);

impl Ord for Highest {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0).then(other.1.cmp(&self.1))
    }
}

impl PartialOrd for Highest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Highest {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Highest {}

/// The tiles of a map shape, by index in the order of `shape.coords()`, with their neighbors.
struct Grid {
    columns: usize,
    rows: usize,
    /// Wrapping around east-west, like the map.
    neighbors: Vec<Vec<usize>>,
}

impl Grid {
    fn new(shape: RectangularHexMapShape) -> Self {
        let columns = shape.columns() as usize;
        let index = |coords: OffsetCoords| coords.row() as usize * columns + coords.col() as usize;
        Self {
            columns,
            rows: shape.rows() as usize,
            neighbors: shape
                .coords()
                .map(|coords| {
                    shape
                        .neighbors(coords)
                        .map(|neighbor| index(neighbor.into()))
                        .collect()
                })
                .collect(),
        }
    }

    fn len(&self) -> usize {
        self.neighbors.len()
    }

    /// The values averaged with their neighbors', `passes` times over: each pass spreads a value
    /// one tile further. Beyond the poles, where a tile lacks neighbors, it stands in for them.
    fn smooth(&self, values: &[f64], passes: u32) -> Vec<f64> {
        let mut field = values.to_vec();
        for _ in 0..passes {
            field = self
                .neighbors
                .iter()
                .enumerate()
                .map(|(index, neighbors)| {
                    let sum: f64 = neighbors.iter().map(|&neighbor| field[neighbor]).sum();
                    let missing = (6 - neighbors.len()) as f64;
                    (field[index] * (1.0 + missing) + sum) / 7.0
                })
                .collect();
        }
        field
    }

    /// The tiles within `reach` steps of any tile of the set.
    fn grow(&self, set: &[bool], reach: u32) -> Vec<bool> {
        self.steps_from(set, reach)
            .into_iter()
            .map(|steps| steps <= reach)
            .collect()
    }

    /// How many steps each tile lies from the nearest tile of the set, `u32::MAX` beyond `reach`:
    /// a breadth-first search from all of them at once.
    fn steps_from(&self, set: &[bool], reach: u32) -> Vec<u32> {
        let mut distance: Vec<u32> = set
            .iter()
            .map(|&inside| if inside { 0 } else { u32::MAX })
            .collect();
        let mut queue: VecDeque<usize> = (0..self.len()).filter(|&index| set[index]).collect();
        while let Some(index) = queue.pop_front() {
            let next = distance[index] + 1;
            if next > reach {
                continue;
            }
            for &neighbor in &self.neighbors[index] {
                if distance[neighbor] == u32::MAX {
                    distance[neighbor] = next;
                    queue.push_back(neighbor);
                }
            }
        }
        distance
    }

    /// The separate arcs of land around a tile, `turned` taken for sea if it is land and for land
    /// if it is sea: `None` for a tile short of neighbors, on the edge of the map.
    fn land_arcs(&self, land: &[bool], index: usize, turned: Option<usize>) -> Option<usize> {
        // the neighbors come in turn around the tile
        let ring = &self.neighbors[index];
        let is_land = |tile: usize| land[tile] != (Some(tile) == turned);
        (ring.len() == 6).then(|| {
            (0..6)
                .filter(|&k| is_land(ring[k]) && !is_land(ring[(k + 5) % 6]))
                .count()
        })
    }

    /// Whether turning a tile from sea to land or back only moves the coast: one arc of land
    /// around it and one of sea, so that nothing joins or parts on either side, and no neighbor
    /// left with the land around it in more arcs than before - a neck one tile wide.
    fn moves_the_coast(&self, land: &[bool], index: usize) -> bool {
        let ring = &self.neighbors[index];
        if self.land_arcs(land, index, None) != Some(1) || ring.iter().all(|&tile| land[tile]) {
            return false;
        }
        ring.iter().filter(|&&tile| land[tile]).all(|&tile| {
            let (before, after) = (
                self.land_arcs(land, tile, None),
                self.land_arcs(land, tile, Some(index)),
            );
            match (before, after) {
                (Some(before), Some(after)) => after <= before.max(1),
                _ => true,
            }
        })
    }

    /// Whether a land tile may sink: it only moves the coast, and leaves no neighbor alone in
    /// the sea.
    fn may_sink(&self, land: &[bool], index: usize) -> bool {
        land[index]
            && self.moves_the_coast(land, index)
            && self.neighbors[index]
                .iter()
                .filter(|&&tile| land[tile])
                .all(|&tile| {
                    self.neighbors[tile]
                        .iter()
                        .any(|&other| land[other] && other != index)
                })
    }

    /// A morphological opening of the land: what is left of it once every tile within `radius`
    /// of the sea is taken away, grown back by `radius`. Land narrower than `2 × radius` tiles
    /// does not grow back: necks part, fringes and specks go, the bodies of land keep their shape.
    fn open(&self, land: &[bool], radius: u32) -> Vec<bool> {
        if radius == 0 {
            return land.to_vec();
        }
        let sea: Vec<bool> = land.iter().map(|&land| !land).collect();
        let core: Vec<bool> = self
            .grow(&sea, radius)
            .into_iter()
            .map(|near_sea| !near_sea)
            .collect();
        let regrown = self.grow(&core, radius);
        land.iter()
            .zip(regrown)
            .map(|(&land, regrown)| land && regrown)
            .collect()
    }

    /// Cut every landmass holding more than `max_share` of the land at its waist, into parts of
    /// `MIN_SPLIT_PART` of the land or more: eroded from the sea one tile deeper at a time, from
    /// `from`, it falls apart into cores; each tile goes to the core it is fewest steps from, and
    /// where two cores' regions meet, the lower tile of each pair sinks. A landmass with no waist
    /// down to `MAX_WAIST_CUT` stays whole.
    fn split_giants(&self, land: &mut [bool], blended: &[f64], max_share: f64, from: u32) {
        if max_share >= 1.0 {
            return;
        }
        let lower = |a: usize, b: usize| blended[a].total_cmp(&blended[b]).then(a.cmp(&b)).is_lt();
        // each cut parts one landmass in two at least, into parts above a share of the land
        for _ in 0..(1.0 / MIN_SPLIT_PART) as usize {
            let (landmass, areas) = self.landmasses(land);
            let total: usize = areas.iter().sum();
            let part = (MIN_SPLIT_PART * total as f64).ceil() as usize;
            let mut giants: Vec<usize> = (0..areas.len())
                .filter(|&id| areas[id] as f64 > max_share * total as f64)
                .collect();
            giants.sort_unstable_by_key(|&id| std::cmp::Reverse(areas[id]));
            let Some(regions) = giants.into_iter().find_map(|giant| {
                let on_giant: Vec<bool> = landmass.iter().map(|&id| id == Some(giant)).collect();
                self.waist(&on_giant, from, part)
            }) else {
                return;
            };
            for index in 0..self.len() {
                let Some(own) = regions[index] else {
                    continue;
                };
                let meets_another = self.neighbors[index].iter().any(|&neighbor| {
                    regions[neighbor].is_some_and(|other| other != own) && lower(index, neighbor)
                });
                if meets_another {
                    land[index] = false;
                }
            }
        }
    }

    /// Wear every landmass holding from `CONTINENT_SHARE` to `min_share` of the land down to an
    /// island: off with its coast, ring after ring, until what is left holds less than
    /// `WORN_DOWN × CONTINENT_SHARE` of the land.
    fn wear_down_minor_continents(&self, land: &mut [bool], min_share: f64) {
        let (landmass, areas) = self.landmasses(land);
        let total = areas.iter().sum::<usize>() as f64;
        let island = WORN_DOWN * CONTINENT_SHARE * total;
        for (id, &area) in areas.iter().enumerate() {
            let share = area as f64 / total;
            if !(CONTINENT_SHARE..min_share).contains(&share) {
                continue;
            }
            let on_it: Vec<bool> = landmass.iter().map(|&of| of == Some(id)).collect();
            let off: Vec<bool> = on_it.iter().map(|&on| !on).collect();
            // steps to the sea, for every tile of it
            let mut inland = vec![0u32; self.len()];
            let mut reach = 1;
            loop {
                let near_shore = self.grow(&off, reach);
                let left = (0..self.len())
                    .filter(|&index| on_it[index] && !near_shore[index])
                    .count();
                for index in 0..self.len() {
                    if on_it[index] && !near_shore[index] {
                        inland[index] = reach;
                    }
                }
                if (left as f64) < island {
                    break;
                }
                reach += 1;
            }
            for index in 0..self.len() {
                if on_it[index] && inland[index] < reach {
                    land[index] = false;
                }
            }
        }
    }

    /// The regions a landmass splits into at its waist: each tile of it labelled with the core it
    /// is fewest steps from, the cores being what is left of it eroded `radius` tiles from the
    /// sea, for the least radius from `from` that leaves two cores whose regions hold `part` tiles
    /// each at least. `None` when no radius up to `MAX_WAIST_CUT` does.
    fn waist(&self, landmass: &[bool], from: u32, part: usize) -> Option<Vec<Option<usize>>> {
        let off: Vec<bool> = landmass.iter().map(|&on| !on).collect();
        for radius in from.max(1)..=MAX_WAIST_CUT {
            let near_shore = self.grow(&off, radius);
            let core: Vec<bool> = (0..self.len())
                .map(|index| landmass[index] && !near_shore[index])
                .collect();
            let (cores, _) = self.landmasses(&core);
            let regions = self.regions(landmass, &cores);
            let mut areas = vec![0usize; cores.iter().flatten().max().map_or(0, |&id| id + 1)];
            if areas.len() < 2 {
                if areas.is_empty() {
                    return None;
                }
                continue;
            }
            for &region in regions.iter().flatten() {
                areas[region] += 1;
            }
            // the small cores give their tiles to the big ones
            let big: Vec<Option<usize>> = cores
                .iter()
                .map(|core| core.filter(|&id| areas[id] >= part))
                .collect();
            if big
                .iter()
                .flatten()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                >= 2
            {
                return Some(self.regions(landmass, &big));
            }
        }
        None
    }

    /// Each tile of a landmass labelled with the seed it is fewest steps from over the landmass: a
    /// breadth-first search from every seed at once.
    fn regions(&self, landmass: &[bool], seeds: &[Option<usize>]) -> Vec<Option<usize>> {
        let mut regions = seeds.to_vec();
        let mut queue: VecDeque<usize> = (0..self.len())
            .filter(|&index| seeds[index].is_some())
            .collect();
        while let Some(index) = queue.pop_front() {
            for &neighbor in &self.neighbors[index] {
                if landmass[neighbor] && regions[neighbor].is_none() {
                    regions[neighbor] = regions[index];
                    queue.push_back(neighbor);
                }
            }
        }
        regions
    }

    /// Sink the coasts of the continents that come within `MIN_STRAIT` tiles of sea of another
    /// one: of two tiles too close, the lower in the blended field goes, until none are left.
    fn part_continents(&self, land: &mut [bool], blended: &[f64]) {
        loop {
            let (landmass, areas) = self.landmasses(land);
            let total: usize = areas.iter().sum();
            let continent = |index: usize| {
                landmass[index].filter(|&id| areas[id] as f64 >= CONTINENT_SHARE * total as f64)
            };
            let lower =
                |a: usize, b: usize| blended[a].total_cmp(&blended[b]).then(a.cmp(&b)).is_lt();
            let doomed: Vec<usize> = (0..self.len())
                .filter(|&index| {
                    let Some(own) = continent(index) else {
                        return false;
                    };
                    self.within(index, MIN_STRAIT).into_iter().any(|other| {
                        continent(other).is_some_and(|id| id != own) && lower(index, other)
                    })
                })
                .collect();
            if doomed.is_empty() {
                return;
            }
            for index in doomed {
                land[index] = false;
            }
        }
    }

    /// The landmass of each tile, numbered from 0, and the area of each landmass.
    fn landmasses(&self, land: &[bool]) -> (Vec<Option<usize>>, Vec<usize>) {
        let mut landmass = vec![None; self.len()];
        let mut areas = Vec::new();
        for start in 0..self.len() {
            if !land[start] || landmass[start].is_some() {
                continue;
            }
            let id = areas.len();
            landmass[start] = Some(id);
            let mut area = 0;
            let mut stack = vec![start];
            while let Some(index) = stack.pop() {
                area += 1;
                for &neighbor in &self.neighbors[index] {
                    if land[neighbor] && landmass[neighbor].is_none() {
                        landmass[neighbor] = Some(id);
                        stack.push(neighbor);
                    }
                }
            }
            areas.push(area);
        }
        (landmass, areas)
    }

    /// The tiles from 1 to `reach` steps away from a tile.
    fn within(&self, center: usize, reach: u32) -> Vec<usize> {
        let mut ring = vec![center];
        let mut seen = vec![center];
        for _ in 0..reach {
            let mut next = Vec::new();
            for &index in &ring {
                for &neighbor in &self.neighbors[index] {
                    if !seen.contains(&neighbor) {
                        seen.push(neighbor);
                        next.push(neighbor);
                    }
                }
            }
            ring = next;
        }
        seen.remove(0);
        seen
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use super::{Grid, MIN_STRAIT, OpenPoles, latitude};
    use crate::map::shores::{Straightness, Stretch};

    /// A land mask drawn one character per tile, rows separated by `|`: `#` land, `.` sea.
    fn mask(art: &str) -> (Grid, Vec<bool>) {
        let rows: Vec<&str> = art.split('|').collect();
        let shape = RectangularHexMapShape::new(rows[0].len() as i16, rows.len() as i16);
        let land = rows
            .iter()
            .flat_map(|row| row.chars().map(|tile| tile == '#'))
            .collect();
        (Grid::new(shape), land)
    }

    fn draw(grid: &Grid, land: &[bool]) -> String {
        land.chunks(grid.columns)
            .map(|row| {
                row.iter()
                    .map(|&land| if land { '#' } else { '.' })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("|")
    }

    #[test]
    fn test_poles_are_drawn_from_the_seed() {
        assert_eq!(OpenPoles::draw(7, 0.0), OpenPoles::NONE);
        assert_eq!(OpenPoles::draw(7, 1.0), OpenPoles::BOTH);
        assert_eq!(OpenPoles::draw(7, 0.5), OpenPoles::draw(7, 0.5));
        // about the chance asked for, each pole on its own
        let open: Vec<OpenPoles> = (0..400).map(|seed| OpenPoles::draw(seed, 0.25)).collect();
        let north = open.iter().filter(|poles| poles.north).count();
        let south = open.iter().filter(|poles| poles.south).count();
        assert!((70..=130).contains(&north), "{north} open north poles");
        assert!((70..=130).contains(&south), "{south} open south poles");
        assert_eq!(OpenPoles::BOTH.count(), 2);
    }

    #[test]
    fn test_latitude_goes_from_the_north_pole_to_the_south_pole() {
        assert_eq!(latitude(0, 5), -1.0);
        assert_eq!(latitude(2, 5), 0.0);
        assert_eq!(latitude(4, 5), 1.0);
        assert_eq!(latitude(0, 1), 0.0);
    }

    #[test]
    fn test_smoothing_spreads_a_value_one_tile_per_pass() {
        let (grid, _) = mask(".......|.......|.......|.......|.......");
        let mut values = vec![0.0; grid.len()];
        values[2 * 7 + 3] = 7.0;
        assert_eq!(grid.smooth(&values, 0), values);
        let once = grid.smooth(&values, 1);
        assert_eq!(once[2 * 7 + 3], 1.0);
        assert_eq!(once[2 * 7 + 4], 1.0);
        assert_eq!(once[2 * 7 + 5], 0.0);
        let twice = grid.smooth(&values, 2);
        assert!(twice[2 * 7 + 5] > 0.0);
        let total: f64 = twice.iter().sum();
        assert!((total - 7.0).abs() < 1.0, "{total}");
    }

    #[test]
    fn test_opening_cuts_a_neck_and_keeps_the_bodies() {
        // two blobs of land, joined by a neck one tile wide
        let (grid, land) = mask(
            "...............|.####.....####.|.#####...#####.|.#############.|.#####...#####.|.####.....####.|...............",
        );
        assert_eq!(grid.landmasses(&land).1.len(), 1);
        let opened = grid.open(&land, 1);
        let (_, areas) = grid.landmasses(&opened);
        assert_eq!(areas.len(), 2, "{}", draw(&grid, &opened));
        // the neck went, and hardly anything of the blobs
        let kept = opened.iter().filter(|&&land| land).count();
        assert!(kept >= 40, "{kept} of 51: {}", draw(&grid, &opened));
        assert_eq!(grid.open(&land, 0), land);
    }

    #[test]
    fn test_opening_sinks_specks() {
        let (grid, land) = mask(".......|.#.....|.......|....##.|.......");
        assert!(grid.open(&land, 1).iter().all(|&land| !land));
    }

    #[test]
    fn test_continents_too_close_are_parted_by_a_strait() {
        // two continents one tile of sea apart; the west one lies lower, and gives way
        let (grid, mut land) = mask("..###.###..|..###.###..|..###.###..|..###.###..");
        let blended: Vec<f64> = (0..grid.len())
            .map(|index| if index % 11 < 5 { 0.1 } else { 0.2 })
            .collect();
        grid.part_continents(&mut land, &blended);
        // the west one gave way, the east one is whole
        assert_eq!(
            draw(&grid, &land),
            "..##..###..|..##..###..|..##..###..|..##..###.."
        );
        let (landmass, areas) = grid.landmasses(&land);
        assert_eq!(areas.len(), 2);
        for index in (0..grid.len()).filter(|&index| land[index]) {
            for other in grid.within(index, MIN_STRAIT) {
                assert!(
                    !land[other] || landmass[other] == landmass[index],
                    "{index} and {other}"
                );
            }
        }
    }

    #[test]
    fn test_a_continent_keeps_its_own_coves() {
        // one landmass around a bay: nothing to part
        let (grid, mut land) = mask("#####|#...#|#.#.#|#####");
        let before = land.clone();
        grid.part_continents(&mut land, &vec![0.0; grid.len()]);
        assert_eq!(land, before);
    }

    #[test]
    fn test_a_giant_is_split_at_its_waist() {
        // two blobs of 7 × 7 joined by a band three tiles wide, which the isthmus cut leaves
        let blob = "#######";
        let rows: Vec<String> = (0..9)
            .map(|row| match row {
                0 | 8 => ".".repeat(21),
                3..=5 => format!(".{blob}#####{blob}."),
                _ => format!(".{blob}.....{blob}."),
            })
            .collect();
        let (grid, mut land) = mask(&rows.join("|"));
        let blended = vec![0.0; grid.len()];
        assert_eq!(grid.landmasses(&land).1.len(), 1);
        grid.split_giants(&mut land, &blended, 0.6, 2);
        let (_, areas) = grid.landmasses(&land);
        assert_eq!(areas.len(), 2, "{}", draw(&grid, &land));
        assert!(
            areas.iter().all(|&area| area >= 45),
            "{areas:?}: {}",
            draw(&grid, &land)
        );
        // a giant with no waist stays whole, and none is split when giants are allowed
        let (grid, mut round) = mask(
            &[
                ".".repeat(9).as_str(),
                &"#".repeat(9),
                &"#".repeat(9),
                &".".repeat(9),
            ]
            .join("|"),
        );
        let before = round.clone();
        grid.split_giants(&mut round, &vec![0.0; grid.len()], 0.6, 2);
        assert_eq!(round, before);
        let (grid, mut land) = mask(&rows.join("|"));
        let before = land.clone();
        grid.split_giants(&mut land, &blended, 1.0, 2);
        assert_eq!(land, before);
    }

    #[test]
    fn test_a_minor_continent_is_worn_down_to_an_island() {
        // a continent of 120 tiles, another of 15 - over 5% of the land, under 15%
        let rows: Vec<String> = (0..12)
            .map(|row| match row {
                1..=6 => format!(".{}...........", "#".repeat(20)),
                8..=10 => format!("..........{}.................", "#####"),
                _ => ".".repeat(32),
            })
            .collect();
        let (grid, mut land) = mask(&rows.join("|"));
        let (_, areas) = grid.landmasses(&land);
        assert_eq!(areas, [120, 15]);
        grid.wear_down_minor_continents(&mut land, 0.15);
        let (_, areas) = grid.landmasses(&land);
        assert_eq!(areas[0], 120, "the continent keeps its coast");
        let total: usize = areas.iter().sum();
        assert!(
            areas[1..]
                .iter()
                .all(|&area| (area as f64) < 0.05 * total as f64),
            "{areas:?}: {}",
            draw(&grid, &land)
        );
        // a minimum under the continents' own share wears nothing down
        let (grid, mut land) = mask(&rows.join("|"));
        let before = land.clone();
        grid.wear_down_minor_continents(&mut land, 0.05);
        assert_eq!(land, before);
    }

    #[test]
    fn test_coasts_grow_without_joining_landmasses_or_closing_straits() {
        // two continents two tiles of sea apart, the higher ground to the east
        let (grid, mut land) = mask("...........|.##...##...|.##...##...|...........");
        let blended: Vec<f64> = (0..grid.len()).map(|index| (index % 11) as f64).collect();
        let reshape = super::Reshape {
            shape: RectangularHexMapShape::new(grid.columns as i16, grid.rows as i16),
            grid: &grid,
            blended: &blended,
            poles: OpenPoles::BOTH,
            isthmus_cut: 0,
            continent_max_share: 1.0,
            continent_min_share: 0.0,
            continent_fill: 0.0,
            bay_depth_lift: 0.0,
        };
        reshape.grow_coasts(&mut land, 14);
        assert_eq!(land.iter().filter(|&&land| land).count(), 14);
        let (landmass, areas) = grid.landmasses(&land);
        assert_eq!(areas.len(), 2, "{}", draw(&grid, &land));
        for index in (0..grid.len()).filter(|&index| land[index]) {
            for other in grid.within(index, MIN_STRAIT) {
                assert!(!land[other] || landmass[other] == landmass[index]);
            }
        }
        // no more room: it stops short
        reshape.grow_coasts(&mut land, 44);
        assert!(land.iter().filter(|&&land| land).count() < 44);
    }

    #[test]
    fn test_a_continent_fills_in_its_bay() {
        // a C of land around a bay, open to the east
        let rows = [
            "............",
            ".#######....",
            ".#######....",
            ".##.........",
            ".##.........",
            ".##.........",
            ".#######....",
            ".#######....",
            "............",
        ];
        let (grid, mut land) = mask(&rows.join("|"));
        let blended = vec![0.0; grid.len()];
        let fill = |land: &[bool]| {
            let tiles: Vec<usize> = (0..grid.len()).filter(|&index| land[index]).collect();
            tiles.len() as f64 / super::hull_tiles(grid.columns, &tiles).unwrap().len() as f64
        };
        assert!(fill(&land) < 0.7, "{}", fill(&land));
        let reshape = super::Reshape {
            shape: RectangularHexMapShape::new(grid.columns as i16, grid.rows as i16),
            grid: &grid,
            blended: &blended,
            poles: OpenPoles::BOTH,
            isthmus_cut: 0,
            continent_max_share: 1.0,
            continent_min_share: 0.0,
            continent_fill: 0.85,
            bay_depth_lift: 0.0,
        };
        reshape.fill_in(&mut land);
        assert!(
            fill(&land) >= 0.85,
            "{}: {}",
            fill(&land),
            draw(&grid, &land)
        );
        assert_eq!(grid.landmasses(&land).1.len(), 1);
        // nothing to do with the fill off
        let (_, mut untouched) = mask(&rows.join("|"));
        let before = untouched.clone();
        super::Reshape {
            continent_fill: 0.0,
            ..reshape
        }
        .fill_in(&mut untouched);
        assert_eq!(untouched, before);
    }

    #[test]
    fn test_the_bays_fill_from_their_depths() {
        // the same C: with the depth of the bay lifted, its mouth on the hull's edge fills last
        let rows = [
            "............",
            ".#######....",
            ".#######....",
            ".##.........",
            ".##.........",
            ".##.........",
            ".#######....",
            ".#######....",
            "............",
        ];
        let filled = |bay_depth_lift: f64| {
            let (grid, mut land) = mask(&rows.join("|"));
            let blended = vec![0.0; grid.len()];
            super::Reshape {
                shape: RectangularHexMapShape::new(grid.columns as i16, grid.rows as i16),
                grid: &grid,
                blended: &blended,
                poles: OpenPoles::BOTH,
                isthmus_cut: 0,
                continent_max_share: 1.0,
                continent_min_share: 0.0,
                continent_fill: 0.8,
                bay_depth_lift,
            }
            .fill_in(&mut land);
            land
        };
        let mouth = [3 * 12 + 7, 4 * 12 + 7, 5 * 12 + 7];
        let by_height = filled(0.0);
        let by_depth = filled(1.0);
        assert_eq!(
            by_height.iter().filter(|&&land| land).count(),
            by_depth.iter().filter(|&&land| land).count(),
            "as much land either way"
        );
        assert!(
            mouth.iter().any(|&index| by_height[index]),
            "on level ground the first tiles fill, the mouth among them"
        );
        assert!(
            mouth.iter().all(|&index| !by_depth[index]),
            "the mouth stays open"
        );
        assert!(by_depth[4 * 12 + 3], "the head of the bay fills");
    }

    /// A value from 0.0 to 1.0 for each tile, with no pattern to it.
    fn scatter(len: usize) -> Vec<f64> {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                (state >> 40) as f64 / (1u64 << 24) as f64
            })
            .collect()
    }

    #[test]
    fn test_roughening_trades_capes_for_coves_along_straight_coasts() {
        // two continents with ruler-straight coasts, two tiles of sea apart, and an island
        let rows: Vec<String> = (0..18)
            .map(|row| match row {
                3..=12 => format!("..{}..{}....", "#".repeat(18), "#".repeat(14)),
                15 => format!("{}##{}", ".".repeat(6), ".".repeat(32)),
                _ => ".".repeat(40),
            })
            .collect();
        let (grid, mut land) = mask(&rows.join("|"));
        let shape = RectangularHexMapShape::new(40, 18);
        let before = land.clone();
        let (_, areas_before) = grid.landmasses(&land);
        assert_eq!(areas_before, [180, 140, 2]);
        let ruler = |land: &[bool]| Straightness::of(shape, land, Stretch::RULER);
        assert!(ruler(&land).share() > 0.5, "{:?}", ruler(&land));

        let blended = vec![0.0; grid.len()];
        let reshape = super::Reshape {
            shape,
            grid: &grid,
            blended: &blended,
            // the north pole closed: its polar row is row 0
            poles: OpenPoles {
                north: false,
                south: true,
            },
            isthmus_cut: 0,
            continent_max_share: 1.0,
            continent_min_share: 0.0,
            continent_fill: 0.0,
            bay_depth_lift: 0.0,
        };
        reshape.roughen(&mut land, &scatter(grid.len()), 0.1);

        assert_ne!(land, before);
        assert_eq!(ruler(&land).straight, 0, "{}", draw(&grid, &land));
        // the coasts moved, and nothing else
        let (landmass, areas) = grid.landmasses(&land);
        assert_eq!(areas, areas_before, "{}", draw(&grid, &land));
        for index in (0..grid.len()).filter(|&index| land[index]) {
            for other in grid.within(index, MIN_STRAIT) {
                assert!(!land[other] || landmass[other] == landmass[index]);
            }
            assert_ne!(
                grid.land_arcs(&land, index, None),
                Some(2),
                "a neck at {index}"
            );
        }
        let sea: Vec<bool> = land.iter().map(|&land| !land).collect();
        assert_eq!(grid.landmasses(&sea).1.len(), 1, "no lake");
        let moved: Vec<usize> = (0..grid.len())
            .filter(|&index| land[index] != before[index])
            .collect();
        let near_coast = grid.grow(
            &(0..grid.len())
                .map(|index| before[index] && grid.neighbors[index].iter().any(|&n| !before[n]))
                .collect::<Vec<bool>>(),
            super::ROUGHEN_REACH,
        );
        assert!(moved.iter().all(|&index| near_coast[index]));
    }

    #[test]
    fn test_roughening_keeps_off_the_closed_poles() {
        // a continent along the polar row of a closed north pole: its coves sink, no cape rises
        // on the pole
        let rows: Vec<String> = (0..12)
            .map(|row| match row {
                1..=8 => format!("....{}....", "#".repeat(22)),
                _ => ".".repeat(30),
            })
            .collect();
        let (grid, mut land) = mask(&rows.join("|"));
        let blended = vec![0.0; grid.len()];
        let reshape = super::Reshape {
            shape: RectangularHexMapShape::new(30, 12),
            grid: &grid,
            blended: &blended,
            poles: OpenPoles::NONE,
            isthmus_cut: 0,
            continent_max_share: 1.0,
            continent_min_share: 0.0,
            continent_fill: 0.0,
            bay_depth_lift: 0.0,
        };
        reshape.roughen(&mut land, &scatter(grid.len()), 0.1);
        assert!(
            land[..30].iter().all(|&land| !land),
            "{}",
            draw(&grid, &land)
        );
        assert_eq!(land.iter().filter(|&&land| land).count(), 22 * 8);
        // the coast along the pole is broken all the same
        let north: Vec<bool> = land[30..60].to_vec();
        assert!(
            north[4..26].iter().any(|&land| !land),
            "{}",
            draw(&grid, &land)
        );
    }

    #[test]
    fn test_a_tile_may_sink_where_it_only_moves_the_coast() {
        // two blobs joined by a neck two tiles wide
        let (grid, land) = mask(
            &[
                "...............",
                ".####.....####.",
                ".####.....####.",
                ".#############.",
                ".#############.",
                ".####.....####.",
                ".####.....####.",
                "...............",
            ]
            .join("|"),
        );
        assert!(grid.may_sink(&land, 15 + 2), "a tile of a straight coast");
        assert!(
            !grid.may_sink(&land, 3 * 15 + 7),
            "the neck would be left a tile wide"
        );
        assert!(!grid.may_sink(&land, 2 * 15 + 2), "inland");
        assert!(!grid.may_sink(&land, 0), "the sea");
        // a lone tile is no coast to move, and a lake no sea to open
        let (grid, land) = mask(".....|..#..|.....");
        assert!(!grid.may_sink(&land, 7));
        let (grid, land) = mask("#####|##.##|#####|#####");
        assert!(!grid.moves_the_coast(&land, 7));
    }

    #[test]
    fn test_within_reaches_the_rings_around_a_tile() {
        let (grid, _) = mask(".........|.........|.........|.........|.........");
        assert_eq!(grid.within(2 * 9 + 4, 1).len(), 6);
        assert_eq!(grid.within(2 * 9 + 4, 2).len(), 18);
    }
}
