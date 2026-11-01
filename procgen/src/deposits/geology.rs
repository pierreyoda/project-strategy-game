//! Where each kind of deposit belongs, read off the surface alone: a suitability from 0.0 to 1.0
//! per tile and kind.
//!
//! An approximation of real geology, kept to what the surface knows - terrain, elevation, latitude
//! and the distance to a few features:
//! - **metals** come with mountain building: the ranges, their hills and foothills;
//! - **oil** pools in sedimentary basins: low plains well away from the ranges;
//! - **water** springs below relief and around lakes, and runs short in the subtropical deserts
//!   and on salty coasts;
//! - **uranium** is rare, and favors old hills and inland plateaus.
//!
//! Ice and water never hold a deposit: no land unit could reach one offshore, and the ice caps are
//! nobody's home.

use std::collections::VecDeque;

use project_x_core::game::state::TileIndex;

use super::settings::{DepositKind, PerKind};
use crate::map::{PlanetSurface, TerrainKind, survey::components};

/// Mountains this close make a plain foothills.
pub const FOOTHILLS_REACH: u32 = 2;
/// Oil keeps at least this far from any mountain.
pub const BASIN_CLEARANCE: u32 = 3;
/// Water springs this close to hills or mountains.
pub const SPRINGS_REACH: u32 = 3;
/// The subtropical dry belt, in absolute latitude (0.0 the equator, 1.0 the poles): roughly 20°
/// to 35° on Earth.
pub const DRY_BELT: (f64, f64) = (0.22, 0.39);
/// Inland from this many hexes off the coast, a plain counts as a plateau for uranium.
pub const INLAND_REACH: u32 = 4;

/// The distances and features the suitabilities come from, kept for the survey to judge the
/// deposits by the same measures.
#[derive(Clone, Debug)]
pub struct Geology {
    /// Hexes to the nearest mountain; `u32::MAX` on a planet without one.
    mountain_distance: Vec<u32>,
    /// Hexes to the nearest hills or mountain.
    relief_distance: Vec<u32>,
    /// Hexes to the nearest water: 1 on the coast.
    coast_distance: Vec<u32>,
    /// Land next to a lake: water enclosed by land, apart from the world ocean.
    lakeshore: Vec<bool>,
    suitability: PerKind<Vec<f64>>,
}

impl Geology {
    pub fn survey(surface: &PlanetSurface) -> Self {
        let mountain_distance = distance_field(surface, |index| {
            surface.terrain_at(index) == TerrainKind::Mountains
        });
        let relief_distance = distance_field(surface, |index| {
            matches!(
                surface.terrain_at(index),
                TerrainKind::Hills | TerrainKind::Mountains
            )
        });
        let coast_distance = distance_field(surface, |index| !surface.is_land(index));

        // every body of water but the biggest one is a lake, however large
        let mut lakeshore = vec![false; surface.tiles_count()];
        let mut waters = components(surface, |index| !surface.is_land(index));
        waters.sort_unstable_by_key(|body| std::cmp::Reverse(body.len()));
        for lake in waters.iter().skip(1) {
            for &index in lake {
                for neighbor in surface.neighbors(index) {
                    if surface.is_land(neighbor) {
                        lakeshore[neighbor as usize] = true;
                    }
                }
            }
        }

        let mut geology = Self {
            mountain_distance,
            relief_distance,
            coast_distance,
            lakeshore,
            suitability: PerKind::default(),
        };
        for kind in DepositKind::ALL {
            let field = (0..surface.tiles_count() as TileIndex)
                .map(|index| geology.compute_suitability(surface, kind, index))
                .collect();
            *geology.suitability.of_mut(kind) = field;
        }
        geology
    }

    pub fn suitability(&self, kind: DepositKind, index: TileIndex) -> f64 {
        self.suitability.of(kind)[index as usize]
    }

    pub fn mountain_distance(&self, index: TileIndex) -> u32 {
        self.mountain_distance[index as usize]
    }

    pub fn relief_distance(&self, index: TileIndex) -> u32 {
        self.relief_distance[index as usize]
    }

    pub fn coast_distance(&self, index: TileIndex) -> u32 {
        self.coast_distance[index as usize]
    }

    pub fn is_lakeshore(&self, index: TileIndex) -> bool {
        self.lakeshore[index as usize]
    }

    /// Whether the tile lies in the subtropical dry belt.
    pub fn is_dry(surface: &PlanetSurface, index: TileIndex) -> bool {
        let latitude = surface.latitude(index).abs();
        (DRY_BELT.0..=DRY_BELT.1).contains(&latitude)
    }

    fn compute_suitability(
        &self,
        surface: &PlanetSurface,
        kind: DepositKind,
        index: TileIndex,
    ) -> f64 {
        if !surface.is_land(index) {
            return 0.0;
        }
        let terrain = surface.terrain_at(index);
        if matches!(terrain, TerrainKind::Ice | TerrainKind::RoughIce) {
            return 0.0;
        }
        let near_mountains = self.mountain_distance(index) <= FOOTHILLS_REACH;
        match kind {
            DepositKind::Metals => match terrain {
                TerrainKind::Mountains => 1.0,
                TerrainKind::Hills if near_mountains => 0.8,
                TerrainKind::Hills => 0.6,
                _ if near_mountains => 0.45,
                // the old shields, worn flat
                _ => 0.2,
            },
            DepositKind::Oil => match terrain {
                TerrainKind::Flat if self.mountain_distance(index) >= BASIN_CLEARANCE => {
                    // the lower the basin, the thicker its sediments
                    let elevation = f64::from(surface.elevation_at(index));
                    1.0 - 0.6 * ((elevation - 300.0) / 1200.0).clamp(0.0, 1.0)
                }
                TerrainKind::Flat => 0.08,
                TerrainKind::Hills => 0.05,
                _ => 0.0,
            },
            DepositKind::Water => {
                if self.is_lakeshore(index) {
                    return 1.0;
                }
                let springs = self.relief_distance(index) <= SPRINGS_REACH;
                let base = match terrain {
                    TerrainKind::Mountains => 0.3,
                    TerrainKind::Hills => 0.75,
                    _ if springs => 0.85,
                    // the aquifers under the plains
                    _ => 0.5,
                };
                let salty = if self.coast_distance(index) <= 1 && !springs {
                    0.8
                } else {
                    1.0
                };
                let dry = if Self::is_dry(surface, index) {
                    0.25
                } else {
                    1.0
                };
                base * salty * dry
            }
            DepositKind::Uranium => match terrain {
                TerrainKind::Hills => 1.0,
                TerrainKind::Mountains => 0.4,
                _ if self.coast_distance(index) >= INLAND_REACH => 0.2,
                _ => 0.04,
            },
        }
    }
}

/// Hexes from every tile to the nearest tile `is_source` picks, over land and water alike;
/// `u32::MAX` everywhere when there is none.
pub fn distance_field(surface: &PlanetSurface, is_source: impl Fn(TileIndex) -> bool) -> Vec<u32> {
    let mut distance = vec![u32::MAX; surface.tiles_count()];
    let mut queue = VecDeque::new();
    for index in 0..surface.tiles_count() as TileIndex {
        if is_source(index) {
            distance[index as usize] = 0;
            queue.push_back(index);
        }
    }
    while let Some(index) = queue.pop_front() {
        let next = distance[index as usize] + 1;
        for neighbor in surface.neighbors(index) {
            if distance[neighbor as usize] == u32::MAX {
                distance[neighbor as usize] = next;
                queue.push_back(neighbor);
            }
        }
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::{Geology, distance_field};
    use crate::{deposits::settings::DepositKind, map::surface::tests::surface_from_art};

    #[test]
    fn test_distance_field_counts_hexes_to_the_nearest_source() {
        let surface = surface_from_art("^......~");
        let distance = distance_field(&surface, |index| index == 0);
        // a single row wraps around: column 7 is next to column 0
        assert_eq!(distance, [0, 1, 2, 3, 4, 3, 2, 1]);
        let none = distance_field(&surface, |_| false);
        assert!(none.iter().all(|&d| d == u32::MAX));
    }

    #[test]
    fn test_metals_belong_to_the_ranges_and_oil_to_the_basins() {
        // a range on the west, a wide plain to the east, the sea beyond
        let surface = surface_from_art("^:.......~~|^:.......~~|^:.......~~");
        let geology = Geology::survey(&surface);
        let metals = |index| geology.suitability(DepositKind::Metals, index);
        let oil = |index| geology.suitability(DepositKind::Oil, index);
        assert_eq!(metals(0), 1.0, "the range");
        assert!(metals(1) > metals(2), "hills over foothills");
        assert!(metals(2) > metals(6), "foothills over the open plain");
        assert_eq!(oil(0), 0.0, "never in the range");
        assert!(oil(6) > 0.9, "the basin");
        assert!(oil(6) > oil(2) * 5.0, "the basin over the foothills");
        assert_eq!(metals(9), 0.0, "never at sea");
    }

    #[test]
    fn test_water_springs_below_relief_and_around_lakes() {
        // a lake in the middle of a plain, a range to the west, a wide ocean to the east
        let surface = surface_from_art(
            "~^.........~~~~|~^.........~~~~|~^......~..~~~~|~^.........~~~~|~^.........~~~~",
        );
        let geology = Geology::survey(&surface);
        let water = |index| geology.suitability(DepositKind::Water, index);
        assert!(geology.is_lakeshore(37), "next to the lake");
        assert!(!geology.is_lakeshore(10), "next to the world ocean");
        assert_eq!(water(37), 1.0);
        assert!(water(2) > water(6), "below the range over the open plain");
        assert!(water(6) > water(10), "inland over the salty coast");
    }

    #[test]
    fn test_ice_and_water_hold_nothing() {
        let surface = surface_from_art("*_~-");
        let geology = Geology::survey(&surface);
        for kind in DepositKind::ALL {
            for index in 0..4 {
                assert_eq!(geology.suitability(kind, index), 0.0, "{kind:?} on {index}");
            }
        }
    }

    #[test]
    fn test_the_dry_belt_lies_in_the_subtropics() {
        // 21 rows: latitude steps of 0.1
        let surface = surface_from_art(&vec!["."; 21].join("|"));
        let dry: Vec<bool> = (0..21).map(|row| Geology::is_dry(&surface, row)).collect();
        let expected: Vec<bool> = (0..21)
            .map(|row: i32| matches!((row - 10).abs(), 3))
            .collect();
        assert_eq!(dry, expected);
    }
}
