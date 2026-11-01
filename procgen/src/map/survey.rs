//! A survey of a generated planet against the Earth-like criteria: one number per criterion, a
//! target range for each, and whether the planet meets it.
//!
//! The survey reads the classified surface, nothing else, so it says the same thing of a planet
//! whether it comes out of the generator, the WASM bridge or a test's drawing. It is a tuning
//! instrument first: `view_survey` prints it over many seeds, and a few guards pin the loose ends
//! of the ranges so that a tuning pass cannot regress quietly.

use std::{cmp::Reverse, collections::VecDeque, fmt};

use project_x_core::game::state::TileIndex;

use super::{TerrainKind, landmasses::Landmasses, surface::PlanetSurface};

/// Share of the land a landmass needs to count as a continent.
const CONTINENT_SHARE: f64 = 0.05;
/// Below this share of the land, a landmass is a speck.
const ISLET_SHARE: f64 = 0.01;
/// A mountain component counts as a chain from this many tiles.
const CHAIN_MIN: usize = 5;
/// An enclosed water body counts as a sea from this share of the map, puddles excepted.
const SEA_SHARE: f64 = 0.01;
/// Up to this many tiles, an enclosed water body is a puddle, whatever the map.
const PUDDLE_MAX: usize = 3;
/// Hills this close to a mountain are its foothills.
const FOOTHILLS_REACH: u32 = 2;

/// What a criterion asks of its value.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Target {
    Within(f64, f64),
    AtLeast(f64),
    AtMost(f64),
    /// Reported, never judged: a fact about the seed rather than a quality of the planet.
    Info,
}

impl Target {
    pub fn met_by(self, value: f64) -> bool {
        match self {
            Self::Within(min, max) => (min..=max).contains(&value),
            Self::AtLeast(min) => value >= min,
            Self::AtMost(max) => value <= max,
            Self::Info => true,
        }
    }
}

/// How a criterion's value reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Unit {
    Share,
    Count,
    Tiles,
    Meters,
    Ratio,
    Flag,
}

impl Unit {
    pub fn format(self, value: f64) -> String {
        match self {
            Self::Share => format!("{:.0}%", value * 100.0),
            Self::Count => format!("{value:.0}"),
            Self::Tiles => format!("{value:.1} tiles"),
            Self::Meters => format!("{value:.0} m"),
            Self::Ratio => format!("{value:.2}"),
            Self::Flag => (if value > 0.5 { "yes" } else { "no" }).to_string(),
        }
    }

    pub fn format_target(self, target: Target) -> String {
        let one = |value| match self {
            Self::Share => format!("{:.0}%", value * 100.0),
            Self::Count | Self::Flag => format!("{value:.0}"),
            Self::Tiles => format!("{value:.0}"),
            Self::Meters => format!("{value:.0} m"),
            Self::Ratio => format!("{value:.2}"),
        };
        match target {
            Target::Within(min, max) => format!("{}-{}", one(min), one(max)),
            Target::AtLeast(min) => format!(">= {}", one(min)),
            Target::AtMost(max) => format!("<= {}", one(max)),
            Target::Info => String::new(),
        }
    }
}

/// One line of the survey.
#[derive(Clone, Debug, PartialEq)]
pub struct Criterion {
    /// Its number in the criteria list, like `A2`.
    pub id: &'static str,
    pub name: &'static str,
    /// `NaN` when the planet has nothing to measure it on, such as a continent-only metric on
    /// an all-sea planet; such a criterion is neither met nor failed.
    pub value: f64,
    pub unit: Unit,
    pub target: Target,
    /// Whether missing it fails the planet outright, whatever else it meets. Marked `!` after
    /// its number.
    pub mandatory: bool,
}

impl Criterion {
    /// The same criterion, made mandatory.
    pub fn mandatory(self) -> Self {
        Self {
            mandatory: true,
            ..self
        }
    }

    /// `None` when there was nothing to measure.
    pub fn met(&self) -> Option<bool> {
        (!self.value.is_nan()).then(|| self.target.met_by(self.value))
    }
}

impl fmt::Display for Criterion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mark = match self.met() {
            _ if self.target == Target::Info => ' ',
            Some(true) => '+',
            Some(false) => 'x',
            None => '-',
        };
        let value = if self.value.is_nan() {
            "n/a".to_string()
        } else {
            self.unit.format(self.value)
        };
        let mandatory = if self.mandatory { "!" } else { "" };
        write!(f, "[{mark}] {}{mandatory} {}: {value}", self.id, self.name)?;
        let target = self.unit.format_target(self.target);
        if !target.is_empty() {
            write!(f, " ({target})")?;
        }
        Ok(())
    }
}

/// Everything the survey measured.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanetSurvey {
    // A: continents and oceans
    pub land_share: f64,
    pub continents: usize,
    pub biggest_continent_share: f64,
    pub top_three_share: f64,
    /// The least compact continent: its share of tiles with no water around them.
    pub interior_share_min: f64,
    /// The loneliest continent: water tiles to cross to the nearest other one.
    pub neighbour_distance_max: f64,
    pub islet_share: f64,
    pub islands: usize,
    // B: relief
    pub plains_share: f64,
    pub hills_share: f64,
    pub mountains_share: f64,
    pub chained_mountains_share: f64,
    pub mean_chain_size: f64,
    pub mean_chain_elongation: f64,
    pub coastal_chains_share: f64,
    /// The most walled-in continent: its largest plain over all its plains.
    pub plains_connectivity_min: f64,
    pub foothills_share: f64,
    /// The most broken-up continent: its largest plain over its area.
    pub largest_plain_share_min: f64,
    /// The least plain continent: its plains over its plains, hills and mountains.
    pub continent_plains_share_min: f64,
    /// The most mountainous continent: its mountains over its plains, hills and mountains.
    pub continent_mountains_share_max: f64,
    pub median_land_elevation: f64,
    pub highest_peak: f64,
    // C: coasts and inland water
    pub shallow_fringe_share: f64,
    pub enclosed_seas: usize,
    pub puddles: usize,
    pub world_ocean_share: f64,
    pub circumnavigable: bool,
    // D: poles
    pub polar_sea_ice_share: f64,
    pub polar_continent: bool,
}

impl PlanetSurvey {
    pub fn of(surface: &PlanetSurface) -> Self {
        let tiles = surface.tiles_count() as f64;
        let landmasses = Landmasses::find(surface);
        let land = landmasses.land_tiles();
        let land_f = land as f64;

        // A: continents and oceans
        let mut sizes: Vec<usize> = landmasses.ids().map(|id| landmasses.area(id)).collect();
        sizes.sort_unstable_by_key(|&size| Reverse(size));
        let share = |size: usize| size as f64 / land_f.max(1.0);
        let continent_ids: Vec<_> = landmasses
            .ids()
            .filter(|&id| share(landmasses.area(id)) >= CONTINENT_SHARE)
            .collect();
        let continents = continent_ids.len();
        let biggest_continent_share = sizes.first().map_or(0.0, |&size| share(size));
        let top_three_share = sizes.iter().take(3).map(|&size| share(size)).sum();
        let interior_share_min = min_over(continent_ids.iter().map(|&id| {
            let tiles = landmasses.tiles(id);
            let interior = tiles
                .iter()
                .filter(|&&index| surface.neighbors(index).all(|n| surface.is_land(n)))
                .count();
            interior as f64 / tiles.len() as f64
        }));
        let neighbour_distance_max = if continents < 2 {
            f64::NAN
        } else {
            continent_ids
                .iter()
                .map(|&id| water_distance_to_another_continent(surface, &landmasses, id))
                .fold(0.0, f64::max)
        };
        let islet_share = sizes
            .iter()
            .filter(|&&size| share(size) < ISLET_SHARE)
            .map(|&size| share(size))
            .sum();
        let islands = sizes
            .iter()
            .filter(|&&size| (ISLET_SHARE..CONTINENT_SHARE).contains(&share(size)))
            .count();

        // B: relief
        let count = |kind: TerrainKind| {
            surface
                .terrain()
                .iter()
                .filter(|&&terrain| terrain == kind)
                .count()
        };
        let (plains, hills, mountains) = (
            count(TerrainKind::Flat),
            count(TerrainKind::Hills),
            count(TerrainKind::Mountains),
        );
        let classified = (plains + hills + mountains) as f64;
        let chains = components(surface, |index| {
            surface.terrain_at(index) == TerrainKind::Mountains
        });
        let real_chains: Vec<&Vec<TileIndex>> = chains
            .iter()
            .filter(|chain| chain.len() >= CHAIN_MIN)
            .collect();
        let chained: usize = real_chains.iter().map(|chain| chain.len()).sum();
        let chained_mountains_share = if mountains == 0 {
            f64::NAN
        } else {
            chained as f64 / mountains as f64
        };
        let mean_chain_size = mean(real_chains.iter().map(|chain| chain.len() as f64));
        let mean_chain_elongation = mean(
            real_chains
                .iter()
                .map(|chain| diameter(surface, chain) as f64 / (chain.len() as f64).sqrt()),
        );
        let coastal_chains_share = mean(real_chains.iter().map(|chain| {
            let coastal = chain.iter().any(|&index| surface.is_coastal(index));
            if coastal { 1.0 } else { 0.0 }
        }));

        let plains_regions = components(surface, |index| {
            surface.terrain_at(index) == TerrainKind::Flat
        });
        let mut largest_plain = vec![0usize; landmasses.len()];
        let mut plains_of = vec![0usize; landmasses.len()];
        for region in &plains_regions {
            let Some(landmass) = landmasses.of(region[0]) else {
                continue;
            };
            let slot = &mut largest_plain[landmass.index()];
            *slot = (*slot).max(region.len());
            plains_of[landmass.index()] += region.len();
        }
        let plains_connectivity_min = min_over(continent_ids.iter().filter_map(|&id| {
            let plains = plains_of[id.index()];
            (plains > 0).then(|| largest_plain[id.index()] as f64 / plains as f64)
        }));
        let largest_plain_share_min = min_over(
            continent_ids
                .iter()
                .map(|&id| largest_plain[id.index()] as f64 / landmasses.area(id) as f64),
        );
        // B9: each continent on its own, so that one mountainous continent among plain ones
        // does not hide in the planet's average
        let (mut continent_plains_share_min, mut continent_mountains_share_max) =
            (f64::NAN, f64::NAN);
        for &id in &continent_ids {
            let (mut plains, mut hills, mut mountains) = (0usize, 0usize, 0usize);
            for &index in landmasses.tiles(id) {
                match surface.terrain_at(index) {
                    TerrainKind::Flat => plains += 1,
                    TerrainKind::Hills => hills += 1,
                    TerrainKind::Mountains => mountains += 1,
                    _ => {}
                }
            }
            let classified = (plains + hills + mountains) as f64;
            if classified == 0.0 {
                continue;
            }
            continent_plains_share_min = continent_plains_share_min.min(plains as f64 / classified);
            continent_mountains_share_max =
                continent_mountains_share_max.max(mountains as f64 / classified);
        }
        let foothills = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.terrain_at(index) == TerrainKind::Hills)
            .filter(|&index| {
                surface
                    .within(index, FOOTHILLS_REACH)
                    .iter()
                    .any(|&(tile, _)| surface.terrain_at(tile) == TerrainKind::Mountains)
            })
            .count();
        let foothills_share = if hills == 0 {
            f64::NAN
        } else {
            foothills as f64 / hills as f64
        };
        let mut land_elevations: Vec<i16> = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.is_land(index))
            .map(|index| surface.elevation_at(index))
            .collect();
        let median_land_elevation = if land_elevations.is_empty() {
            f64::NAN
        } else {
            let middle = land_elevations.len() / 2;
            f64::from(*land_elevations.select_nth_unstable(middle).1)
        };
        let highest_peak = land_elevations
            .iter()
            .max()
            .map_or(f64::NAN, |&peak| f64::from(peak));

        // C: coasts and inland water
        let fringe: Vec<TileIndex> = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| !surface.is_land(index))
            .filter(|&index| surface.neighbors(index).any(|n| surface.is_land(n)))
            .collect();
        let shallow_fringe_share = if fringe.is_empty() {
            f64::NAN
        } else {
            let shallow = fringe
                .iter()
                .filter(|&&index| surface.terrain_at(index) == TerrainKind::ShallowWater)
                .count();
            shallow as f64 / fringe.len() as f64
        };
        let mut waters = components(surface, |index| !surface.is_land(index));
        waters.sort_unstable_by_key(|body| Reverse(body.len()));
        let water_tiles: usize = waters.iter().map(Vec::len).sum();
        let world_ocean_share = if water_tiles == 0 {
            f64::NAN
        } else {
            waters[0].len() as f64 / water_tiles as f64
        };
        let enclosed_seas = waters
            .iter()
            .skip(1)
            .filter(|body| body.len() > PUDDLE_MAX && body.len() as f64 / tiles >= SEA_SHARE)
            .count();
        let puddles = waters
            .iter()
            .skip(1)
            .filter(|body| body.len() <= PUDDLE_MAX)
            .count();
        let circumnavigable = waters
            .first()
            .is_some_and(|ocean| wraps_around(surface, ocean));

        // D: poles
        let iced_rows: Vec<bool> = (0..surface.shape().rows() as u32)
            .map(|row| {
                let columns = surface.shape().columns() as u32;
                (row * columns..(row + 1) * columns).any(|index| {
                    matches!(
                        surface.terrain_at(index),
                        TerrainKind::Ice | TerrainKind::RoughIce
                    )
                })
            })
            .collect();
        let (mut polar_sea, mut polar_sea_ice) = (0, 0);
        for index in 0..surface.tiles_count() as TileIndex {
            let row = index / surface.shape().columns() as u32;
            if iced_rows[row as usize] && !surface.is_land(index) {
                polar_sea += 1;
                if matches!(
                    surface.terrain_at(index),
                    TerrainKind::Ice | TerrainKind::RoughIce
                ) {
                    polar_sea_ice += 1;
                }
            }
        }
        let polar_sea_ice_share = if polar_sea == 0 {
            0.0
        } else {
            polar_sea_ice as f64 / polar_sea as f64
        };
        let last_row = surface.shape().rows() as u32 - 1;
        let polar_continent = continent_ids.iter().any(|&id| {
            landmasses.tiles(id).iter().any(|&index| {
                let row = index / surface.shape().columns() as u32;
                row == 0 || row == last_row
            })
        });

        Self {
            land_share: land_f / tiles,
            continents,
            biggest_continent_share,
            top_three_share,
            interior_share_min,
            neighbour_distance_max,
            islet_share,
            islands,
            plains_share: plains as f64 / classified,
            hills_share: hills as f64 / classified,
            mountains_share: mountains as f64 / classified,
            chained_mountains_share,
            mean_chain_size,
            mean_chain_elongation,
            coastal_chains_share,
            plains_connectivity_min,
            foothills_share,
            largest_plain_share_min,
            continent_plains_share_min,
            continent_mountains_share_max,
            median_land_elevation,
            highest_peak,
            shallow_fringe_share,
            enclosed_seas,
            puddles,
            world_ocean_share,
            circumnavigable,
            polar_sea_ice_share,
            polar_continent,
        }
    }

    /// The criteria, in the order of the list they come from.
    pub fn criteria(&self) -> Vec<Criterion> {
        use Target::{AtLeast, AtMost, Info, Within};
        use Unit::{Count, Flag, Meters, Ratio, Share, Tiles};
        let flag = |value: bool| if value { 1.0 } else { 0.0 };
        let criterion = |id, name, value, unit, target| Criterion {
            id,
            name,
            value,
            unit,
            target,
            mandatory: false,
        };
        vec![
            criterion("A1", "land", self.land_share, Share, Within(0.40, 0.50)),
            criterion(
                "A2",
                "continents",
                self.continents as f64,
                Count,
                Within(3.0, 6.0),
            ),
            criterion(
                "A2",
                "biggest continent, of the land",
                self.biggest_continent_share,
                Share,
                Within(0.35, 0.60),
            ),
            criterion(
                "A2",
                "top three continents, of the land",
                self.top_three_share,
                Share,
                AtLeast(0.85),
            ),
            criterion(
                "A3",
                "least compact continent, interior",
                self.interior_share_min,
                Share,
                AtLeast(0.55),
            ),
            criterion(
                "A4",
                "loneliest continent, water to a neighbour",
                self.neighbour_distance_max,
                Tiles,
                AtMost(6.0),
            ),
            criterion(
                "A5",
                "land in islets",
                self.islet_share,
                Share,
                AtMost(0.05),
            ),
            criterion(
                "A5",
                "islands",
                self.islands as f64,
                Count,
                Within(1.0, 4.0),
            ),
            criterion("B1", "plains", self.plains_share, Share, Within(0.65, 0.75)),
            criterion("B1", "hills", self.hills_share, Share, Within(0.20, 0.28)),
            criterion(
                "B1",
                "mountains",
                self.mountains_share,
                Share,
                Within(0.05, 0.10),
            ),
            criterion(
                "B2",
                "mountains in chains",
                self.chained_mountains_share,
                Share,
                AtLeast(0.6),
            ),
            criterion(
                "B2",
                "mean chain",
                self.mean_chain_size,
                Tiles,
                Within(6.0, 20.0),
            ),
            criterion(
                "B3",
                "mean chain elongation",
                self.mean_chain_elongation,
                Ratio,
                AtLeast(2.0),
            ),
            criterion(
                "B4",
                "chains reaching the coast",
                self.coastal_chains_share,
                Share,
                Within(0.3, 0.7),
            ),
            criterion(
                "B5",
                "most walled-in continent, plains connected",
                self.plains_connectivity_min,
                Share,
                AtLeast(0.7),
            ),
            criterion(
                "B6",
                "hills that are foothills",
                self.foothills_share,
                Share,
                AtLeast(0.5),
            ),
            criterion(
                "B7",
                "most broken-up continent, largest plain",
                self.largest_plain_share_min,
                Share,
                AtLeast(0.35),
            ),
            criterion(
                "B9",
                "least plain continent, plains",
                self.continent_plains_share_min,
                Share,
                AtLeast(0.55),
            ),
            criterion(
                "B9",
                "most mountainous continent, mountains",
                self.continent_mountains_share_max,
                Share,
                AtMost(0.15),
            ),
            criterion(
                "B8",
                "median land elevation",
                self.median_land_elevation,
                Meters,
                Within(200.0, 600.0),
            ),
            criterion(
                "B8",
                "highest peak",
                self.highest_peak,
                Meters,
                Within(4000.0, 7000.0),
            ),
            criterion(
                "C1",
                "coastal water that is shallow",
                self.shallow_fringe_share,
                Share,
                AtLeast(0.7),
            ),
            criterion(
                "C3",
                "enclosed seas",
                self.enclosed_seas as f64,
                Count,
                AtMost(2.0),
            ),
            criterion("C3", "puddles", self.puddles as f64, Count, AtMost(3.0)),
            criterion(
                "C4",
                "world ocean, of the water",
                self.world_ocean_share,
                Share,
                AtLeast(0.95),
            ),
            criterion(
                "C5",
                "circumnavigable",
                flag(self.circumnavigable),
                Flag,
                AtLeast(1.0),
            ),
            criterion(
                "D1",
                "polar sea under ice",
                self.polar_sea_ice_share,
                Share,
                AtMost(0.0),
            )
            .mandatory(),
            criterion(
                "D2",
                "a polar continent",
                flag(self.polar_continent),
                Flag,
                Info,
            ),
        ]
    }

    /// Criteria met over criteria judged.
    pub fn score(&self) -> (usize, usize) {
        score(&self.criteria())
    }
}

/// Criteria met over criteria judged: the informative ones and those with nothing to measure are
/// left out.
pub fn score(criteria: &[Criterion]) -> (usize, usize) {
    let judged: Vec<bool> = criteria
        .iter()
        .filter(|criterion| criterion.target != Target::Info)
        .filter_map(Criterion::met)
        .collect();
    (judged.iter().filter(|&&met| met).count(), judged.len())
}

impl fmt::Display for PlanetSurvey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for criterion in self.criteria() {
            writeln!(f, "{criterion}")?;
        }
        let (met, judged) = self.score();
        write!(f, "{met} of {judged} criteria met")
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (mut sum, mut count) = (0.0, 0usize);
    for value in values {
        sum += value;
        count += 1;
    }
    if count == 0 {
        f64::NAN
    } else {
        sum / count as f64
    }
}

fn min_over(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(f64::NAN, f64::min)
}

/// The connected groups of the tiles `include` picks, each in row-major order, wrapping around
/// east-west like the map.
pub(crate) fn components(
    surface: &PlanetSurface,
    include: impl Fn(TileIndex) -> bool,
) -> Vec<Vec<TileIndex>> {
    let mut seen = vec![false; surface.tiles_count()];
    let mut groups = Vec::new();
    for start in 0..surface.tiles_count() as TileIndex {
        if seen[start as usize] || !include(start) {
            continue;
        }
        seen[start as usize] = true;
        let mut group = Vec::new();
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            group.push(index);
            for neighbor in surface.neighbors(index) {
                if !seen[neighbor as usize] && include(neighbor) {
                    seen[neighbor as usize] = true;
                    stack.push(neighbor);
                }
            }
        }
        group.sort_unstable();
        groups.push(group);
    }
    groups
}

/// The longest shortest path within a group of tiles, in steps: two breadth-first searches, the
/// second from the farthest tile the first one reached.
fn diameter(surface: &PlanetSurface, group: &[TileIndex]) -> u32 {
    let farthest = |from: TileIndex| {
        let mut distance = vec![u32::MAX; surface.tiles_count()];
        distance[from as usize] = 0;
        let mut queue = VecDeque::from([from]);
        let mut farthest = (from, 0);
        while let Some(index) = queue.pop_front() {
            let next = distance[index as usize] + 1;
            for neighbor in surface.neighbors(index) {
                if distance[neighbor as usize] == u32::MAX && group.binary_search(&neighbor).is_ok()
                {
                    distance[neighbor as usize] = next;
                    if next > farthest.1 {
                        farthest = (neighbor, next);
                    }
                    queue.push_back(neighbor);
                }
            }
        }
        farthest
    };
    let (end, _) = farthest(group[0]);
    farthest(end).1
}

/// Water tiles to cross from a continent to the nearest tile of another one, by breadth-first
/// search over the water from its shores.
fn water_distance_to_another_continent(
    surface: &PlanetSurface,
    landmasses: &Landmasses,
    from: super::landmasses::LandmassId,
) -> f64 {
    let land_f = landmasses.land_tiles() as f64;
    let is_other_continent = |index: TileIndex| {
        landmasses
            .of(index)
            .is_some_and(|id| id != from && landmasses.area(id) as f64 / land_f >= CONTINENT_SHARE)
    };
    let mut seen = vec![false; surface.tiles_count()];
    let mut queue = VecDeque::new();
    for &index in landmasses.tiles(from) {
        seen[index as usize] = true;
        queue.push_back((index, 0u32));
    }
    while let Some((index, crossed)) = queue.pop_front() {
        for neighbor in surface.neighbors(index) {
            if seen[neighbor as usize] {
                continue;
            }
            seen[neighbor as usize] = true;
            if is_other_continent(neighbor) {
                return f64::from(crossed);
            }
            if !surface.is_land(neighbor) {
                queue.push_back((neighbor, crossed + 1));
            }
        }
    }
    f64::NAN
}

/// Whether a group of tiles goes all the way around the world east-west: a breadth-first search
/// that counts the seam crossings finds a tile reached with two different counts.
fn wraps_around(surface: &PlanetSurface, group: &[TileIndex]) -> bool {
    let columns = surface.shape().columns() as TileIndex;
    if columns < 3 {
        return false;
    }
    let mut winding = vec![None; surface.tiles_count()];
    let start = group[0];
    winding[start as usize] = Some(0i32);
    let mut queue = VecDeque::from([start]);
    while let Some(index) = queue.pop_front() {
        let col = index % columns;
        let wind = winding[index as usize].expect("queued tiles have a winding");
        for neighbor in surface.neighbors(index) {
            if group.binary_search(&neighbor).is_err() {
                continue;
            }
            let neighbor_col = neighbor % columns;
            let crossing = if col == columns - 1 && neighbor_col == 0 {
                1
            } else if col == 0 && neighbor_col == columns - 1 {
                -1
            } else {
                0
            };
            let reached_with = wind + crossing;
            match winding[neighbor as usize] {
                None => {
                    winding[neighbor as usize] = Some(reached_with);
                    queue.push_back(neighbor);
                }
                Some(known) if known != reached_with => return true,
                Some(_) => {}
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{PlanetSurvey, Target};
    use crate::map::surface::tests::surface_from_art;

    #[test]
    fn test_survey_counts_continents_islands_and_islets() {
        // 20 × 10: a continent of 120, an island of 6, two specks of 1
        let land = ".".repeat(20);
        let sea = "~".repeat(20);
        let surface = surface_from_art(
            &[
                land.as_str(),
                land.as_str(),
                land.as_str(),
                land.as_str(),
                land.as_str(),
                land.as_str(),
                sea.as_str(),
                "~~~...~~~~~~~~~~~~~~",
                "~~~...~~~~~.~~~~~~~~",
                "~~~~~~~~~~~~~~~~.~~~",
            ]
            .join("|"),
        );
        let survey = PlanetSurvey::of(&surface);
        assert_eq!(survey.continents, 1, "6 of 128 land tiles is under 5%");
        assert_eq!(survey.islands, 1, "but over 1%");
        assert!((survey.islet_share - 2.0 / 128.0).abs() < 1e-9);
        assert!((survey.biggest_continent_share - 120.0 / 128.0).abs() < 1e-9);
        assert!((survey.land_share - 0.64).abs() < 1e-9);
    }

    #[test]
    fn test_survey_tells_the_ocean_from_enclosed_seas_and_puddles() {
        // a world ocean on the first rows, a lake of 1 and a sea of 4 inside the land
        let surface = surface_from_art(
            &[
                "~~~~~~~~~~",
                "~~~~~~~~~~",
                "..........",
                "..~.......",
                "..........",
                "......~~..",
                "......~~..",
                "..........",
            ]
            .join("|"),
        );
        let survey = PlanetSurvey::of(&surface);
        assert_eq!(survey.puddles, 1, "the lake");
        assert_eq!(
            survey.enclosed_seas, 1,
            "4 of 80 tiles: over 1% of the map, over a puddle"
        );
        assert!((survey.world_ocean_share - 20.0 / 25.0).abs() < 1e-9);
        assert!(survey.circumnavigable, "the open rows wrap around");
    }

    #[test]
    fn test_survey_knows_when_land_blocks_the_way_around() {
        let open = surface_from_art("~~~~~|.....|~~~~~");
        assert!(PlanetSurvey::of(&open).circumnavigable);
        // a column of land from pole to pole: the sea is one body, but no way around
        let barred = surface_from_art("~~.~~|~~.~~|~~.~~");
        let survey = PlanetSurvey::of(&barred);
        assert_eq!(
            survey.enclosed_seas + survey.puddles,
            0,
            "one body of water"
        );
        assert!(!survey.circumnavigable);
    }

    #[test]
    fn test_survey_measures_mountain_chains() {
        // a chain of 7 along a row, a lone peak, on a plain
        let surface = surface_from_art(
            &[
                "..........",
                "..^^^^^^^.",
                "..........",
                "......^...",
                "..........",
            ]
            .join("|"),
        );
        let survey = PlanetSurvey::of(&surface);
        assert!((survey.chained_mountains_share - 7.0 / 8.0).abs() < 1e-9);
        assert_eq!(survey.mean_chain_size, 7.0);
        assert!((survey.mean_chain_elongation - 6.0 / 7f64.sqrt()).abs() < 1e-9);
        assert_eq!(survey.coastal_chains_share, 0.0, "no coast at all");
        assert!(
            survey.plains_connectivity_min > 0.99,
            "the plain goes around the chain"
        );
    }

    #[test]
    fn test_survey_judges_each_continent_on_its_own_relief() {
        // a plain continent to the north, a mountainous one to the south, and together they
        // average out to something that looks fine
        let surface = surface_from_art(
            &[
                "..........",
                "..........",
                "..........",
                "~~~~~~~~~~",
                "^^^^^..^^^",
                "^^^.^^^^^^",
                "^^^^^^^^..",
            ]
            .join("|"),
        );
        let survey = PlanetSurvey::of(&surface);
        assert_eq!(survey.continents, 2);
        assert!(
            (survey.plains_share - 35.0 / 60.0).abs() < 1e-9,
            "the average hides it"
        );
        assert!((survey.continent_plains_share_min - 5.0 / 30.0).abs() < 1e-9);
        assert!((survey.continent_mountains_share_max - 25.0 / 30.0).abs() < 1e-9);
        let criteria = survey.criteria();
        let b9: Vec<_> = criteria.iter().filter(|c| c.id == "B9").collect();
        assert_eq!(b9.len(), 2);
        assert!(b9.iter().all(|c| c.met() == Some(false)));
    }

    #[test]
    fn test_survey_sees_a_wall_and_the_foothills_along_it() {
        // two mountain walls from pole to pole - one across the seam - split the plain into a
        // strip of 1 and a strip of 2, hills along the first wall
        let surface = surface_from_art("^.:^..^|^.:^..^|^.:^..^|^.:^..^");
        let survey = PlanetSurvey::of(&surface);
        assert!((survey.plains_connectivity_min - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(survey.foothills_share, 1.0);
        assert!(survey.interior_share_min > 0.99, "no water anywhere");
    }

    #[test]
    fn test_survey_reads_the_coast_and_the_poles() {
        // ice over the polar sea on the first row, ice over land on the last, a shallow fringe
        let surface = surface_from_art(&["____~", "~~-~~", "~-.-~", "~~-~~", "**.**"].join("|"));
        let survey = PlanetSurvey::of(&surface);
        assert_eq!(survey.polar_sea_ice_share, 4.0 / 5.0);
        assert!(survey.polar_continent, "the icy land on the last row");
        // 9 water tiles touch land, the 4 shallow ones among them
        assert!((survey.shallow_fringe_share - 4.0 / 9.0).abs() < 1e-9);
    }

    #[test]
    fn test_survey_has_nothing_to_say_about_a_planet_without_land() {
        let survey = PlanetSurvey::of(&surface_from_art("~~~|~~~"));
        assert_eq!(survey.continents, 0);
        assert!(survey.interior_share_min.is_nan());
        assert!(survey.neighbour_distance_max.is_nan());
        assert!(survey.median_land_elevation.is_nan());
        let criteria = survey.criteria();
        let continents = criteria.iter().find(|c| c.name == "continents").unwrap();
        assert_eq!(continents.met(), Some(false));
        let interior = criteria
            .iter()
            .find(|c| c.id == "A3")
            .expect("the interior criterion");
        assert_eq!(interior.met(), None, "nothing to measure");
        assert_eq!(
            interior.to_string(),
            "[-] A3 least compact continent, interior: n/a (>= 55%)"
        );
    }

    #[test]
    fn test_criteria_read_as_one_line_each() {
        let criterion = super::Criterion {
            id: "C5",
            name: "circumnavigable",
            value: 1.0,
            unit: super::Unit::Flag,
            target: Target::AtLeast(1.0),
            mandatory: false,
        };
        assert_eq!(criterion.to_string(), "[+] C5 circumnavigable: yes (>= 1)");
        assert_eq!(
            criterion.clone().mandatory().to_string(),
            "[+] C5! circumnavigable: yes (>= 1)"
        );
        let info = super::Criterion {
            id: "D2",
            name: "a polar continent",
            value: 0.0,
            unit: super::Unit::Flag,
            target: Target::Info,
            mandatory: false,
        };
        assert_eq!(info.to_string(), "[ ] D2 a polar continent: no");
    }
}
