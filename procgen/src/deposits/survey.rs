//! A survey of a planet's deposits against the realism and balance criteria, in the same terms as
//! the planet's own `PlanetSurvey`: one number per criterion, a target for each.
//!
//! - **E, realism**: whether each kind lies on the ground it belongs to, clustered as real ore
//!   bodies and oil fields are, at a sensible density, with few corrections laid on purpose.
//! - **F, balance**: whether every nation finds what it is guaranteed, starts not much poorer than
//!   the others, and has to reach out for the scarce kinds that nobody monopolizes.

use std::{collections::VecDeque, fmt};

use project_x_core::game::state::TileIndex;

use super::{
    DepositOrigin, GeneratedDeposits,
    balance::{Assessor, gini, overall},
    geology::{BASIN_CLEARANCE, FOOTHILLS_REACH, Geology, SPRINGS_REACH},
    guarantees::{GUARANTEED_KINDS, Zone, capital_distances, territory_owners},
    settings::{DepositKind, DepositsGeneratorSettings},
};
use crate::{
    map::{Criterion, PlanetSurface, Target, TerrainKind, Unit, score},
    nations::GeneratedNations,
};

/// Two metals deposits this close belong to the same belt.
const BELT_REACH: u32 = 3;

/// Everything the survey measured.
#[derive(Clone, Debug, PartialEq)]
pub struct DepositsSurvey {
    // E: realism
    pub metals_in_ranges: f64,
    pub metals_in_belts: f64,
    pub oil_in_basins: f64,
    pub water_by_springs: f64,
    pub water_in_dry_belt: f64,
    pub uranium_per_metals: f64,
    /// Clark-Evans ratio of all the deposits: under 1.0 clustered, over 1.0 spread out evenly.
    pub clustering: f64,
    pub on_ice: f64,
    pub per_100_land: f64,
    pub corrections_share: f64,
    pub nations_needing_guarantees: f64,
    // F: balance
    pub guarantees_met: f64,
    pub metals_distance_spread: f64,
    pub water_distance_spread: f64,
    pub poorest_over_richest: f64,
    pub endowment_gini: f64,
    pub nations_with_oil: f64,
    pub nations_with_uranium: f64,
    pub scarce_outside_territories: f64,
    pub biggest_oil_holding: f64,
    pub biggest_uranium_holding: f64,
}

impl DepositsSurvey {
    pub fn of(
        surface: &PlanetSurface,
        nations: &GeneratedNations,
        deposits: &GeneratedDeposits,
        settings: &DepositsGeneratorSettings,
    ) -> Self {
        let geology = Geology::survey(surface);
        let all = deposits.deposits();
        // sorted, to be searched
        let tiles_of = |kind: DepositKind| -> Vec<TileIndex> {
            let mut tiles: Vec<TileIndex> = deposits
                .of_kind(kind)
                .map(|deposit| deposit.tile())
                .collect();
            tiles.sort_unstable();
            tiles
        };
        let share_of = |tiles: &[TileIndex], test: &dyn Fn(TileIndex) -> bool| {
            if tiles.is_empty() {
                f64::NAN
            } else {
                tiles.iter().filter(|&&index| test(index)).count() as f64 / tiles.len() as f64
            }
        };
        let land = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.is_land(index))
            .count();

        // E: realism
        let metals = tiles_of(DepositKind::Metals);
        let metals_in_ranges = share_of(&metals, &|index| {
            matches!(
                surface.terrain_at(index),
                TerrainKind::Hills | TerrainKind::Mountains
            ) || geology.mountain_distance(index) <= FOOTHILLS_REACH
        });
        let metals_in_belts = share_of(&metals, &|index| {
            nearest(
                surface,
                index,
                |other| other != index && metals.binary_search(&other).is_ok(),
                BELT_REACH,
            )
            .is_some()
        });
        let oil = tiles_of(DepositKind::Oil);
        let oil_in_basins = share_of(&oil, &|index| {
            surface.terrain_at(index) == TerrainKind::Flat
                && geology.mountain_distance(index) >= BASIN_CLEARANCE
        });
        let water = tiles_of(DepositKind::Water);
        let water_by_springs = share_of(&water, &|index| {
            geology.relief_distance(index) <= SPRINGS_REACH || geology.is_lakeshore(index)
        });
        let water_in_dry_belt = share_of(&water, &|index| Geology::is_dry(surface, index));
        let uranium = tiles_of(DepositKind::Uranium);
        let uranium_per_metals = if metals.is_empty() {
            f64::NAN
        } else {
            uranium.len() as f64 / metals.len() as f64
        };
        let clustering = clark_evans(surface, deposits, land);
        let all_tiles: Vec<TileIndex> = all.iter().map(|deposit| deposit.tile()).collect();
        let on_ice = share_of(&all_tiles, &|index| {
            matches!(
                surface.terrain_at(index),
                TerrainKind::Ice | TerrainKind::RoughIce
            )
        });
        let per_100_land = if land == 0 {
            f64::NAN
        } else {
            all.len() as f64 * 100.0 / land as f64
        };
        let corrections_share = if all.is_empty() {
            f64::NAN
        } else {
            all.iter()
                .filter(|deposit| deposit.origin() != DepositOrigin::Natural)
                .count() as f64
                / all.len() as f64
        };
        let count = nations.nations().len();
        let per_nation = |value: usize| {
            if count == 0 {
                f64::NAN
            } else {
                value as f64 / count as f64
            }
        };
        let nations_needing_guarantees = per_nation(deposits.report().guarantees.nations_helped);

        // F: balance
        let owners = territory_owners(surface, nations);
        let guaranteed = nations
            .nations()
            .iter()
            .filter(|nation| {
                let zone = Zone::of(surface, nations, &owners, nation);
                GUARANTEED_KINDS.iter().all(|&(kind, count)| {
                    deposits
                        .of_kind(kind)
                        .filter(|deposit| zone.contains(deposit.tile()))
                        .count()
                        >= count
                })
            })
            .count();
        let guarantees_met = per_nation(guaranteed);

        let distances: Vec<Vec<u32>> = nations
            .nations()
            .iter()
            .map(|nation| {
                let capital = surface
                    .index(nation.capital())
                    .expect("a capital is on the map");
                capital_distances(surface, capital)
            })
            .collect();
        let spread_to = |kind: DepositKind| {
            let nearest: Vec<u32> = distances
                .iter()
                .filter_map(|from| {
                    deposits
                        .of_kind(kind)
                        .map(|d| from[d.tile() as usize])
                        .min()
                })
                .collect();
            match (nearest.iter().min(), nearest.iter().max()) {
                (Some(min), Some(max)) => f64::from(max - min),
                _ => f64::NAN,
            }
        };
        let metals_distance_spread = spread_to(DepositKind::Metals);
        let water_distance_spread = spread_to(DepositKind::Water);

        let endowments = Assessor::new(surface, nations, settings).endowments(all);
        let totals: Vec<f64> = endowments.iter().map(overall).collect();
        let richest = totals.iter().copied().fold(f64::NAN, f64::max);
        let poorest = totals.iter().copied().fold(f64::NAN, f64::min);
        let poorest_over_richest = if richest > 0.0 {
            poorest / richest
        } else {
            f64::NAN
        };
        let endowment_gini = gini(&totals);

        let sphere_holdings = |kind: DepositKind| {
            let mut held = vec![0usize; count];
            for deposit in deposits.of_kind(kind) {
                if let Some(owner) = nations.spheres().owner(deposit.tile())
                    && let Some(position) = nations
                        .nations()
                        .iter()
                        .position(|nation| nation.id() == owner)
                {
                    held[position] += 1;
                }
            }
            held
        };
        let oil_held = sphere_holdings(DepositKind::Oil);
        let uranium_held = sphere_holdings(DepositKind::Uranium);
        let nations_with_oil = per_nation(oil_held.iter().filter(|&&held| held > 0).count());
        let nations_with_uranium =
            per_nation(uranium_held.iter().filter(|&&held| held > 0).count());
        let biggest_holding = |held: &[usize], total: usize| {
            if total == 0 {
                f64::NAN
            } else {
                held.iter().copied().max().unwrap_or(0) as f64 / total as f64
            }
        };
        let biggest_oil_holding = biggest_holding(&oil_held, oil.len());
        let biggest_uranium_holding = biggest_holding(&uranium_held, uranium.len());
        let scarce: Vec<TileIndex> = oil.iter().chain(&uranium).copied().collect();
        let scarce_outside_territories =
            share_of(&scarce, &|index| owners[index as usize].is_none());

        Self {
            metals_in_ranges,
            metals_in_belts,
            oil_in_basins,
            water_by_springs,
            water_in_dry_belt,
            uranium_per_metals,
            clustering,
            on_ice,
            per_100_land,
            corrections_share,
            nations_needing_guarantees,
            guarantees_met,
            metals_distance_spread,
            water_distance_spread,
            poorest_over_richest,
            endowment_gini,
            nations_with_oil,
            nations_with_uranium,
            scarce_outside_territories,
            biggest_oil_holding,
            biggest_uranium_holding,
        }
    }

    /// The criteria, in the order of the list they come from.
    pub fn criteria(&self) -> Vec<Criterion> {
        use Target::{AtLeast, AtMost, Within};
        use Unit::{Ratio, Share, Tiles};
        let criterion = |id, name, value, unit, target| Criterion {
            id,
            name,
            value,
            unit,
            target,
        };
        vec![
            criterion(
                "E1",
                "metals in or by the ranges",
                self.metals_in_ranges,
                Share,
                AtLeast(0.75),
            ),
            criterion(
                "E2",
                "metals in belts",
                self.metals_in_belts,
                Share,
                AtLeast(0.6),
            ),
            criterion(
                "E3",
                "oil in the basins",
                self.oil_in_basins,
                Share,
                AtLeast(0.8),
            ),
            criterion(
                "E4",
                "water by springs or lakes",
                self.water_by_springs,
                Share,
                AtLeast(0.6),
            ),
            criterion(
                "E4",
                "water in the dry belt",
                self.water_in_dry_belt,
                Share,
                AtMost(0.15),
            ),
            criterion(
                "E5",
                "uranium per metals",
                self.uranium_per_metals,
                Ratio,
                Within(0.15, 0.35),
            ),
            criterion(
                "E6",
                "clustering (Clark-Evans)",
                self.clustering,
                Ratio,
                Within(0.3, 0.85),
            ),
            criterion("E7", "deposits on ice", self.on_ice, Share, AtMost(0.05)),
            criterion(
                "E8",
                "deposits per 100 land tiles",
                self.per_100_land,
                Ratio,
                Within(8.0, 14.0),
            ),
            criterion(
                "E9",
                "deposits laid on purpose",
                self.corrections_share,
                Share,
                AtMost(0.10),
            ),
            criterion(
                "E9",
                "nations needing a guarantee",
                self.nations_needing_guarantees,
                Share,
                AtMost(0.4),
            ),
            criterion(
                "F1",
                "nations with their guarantees",
                self.guarantees_met,
                Share,
                AtLeast(1.0),
            ),
            criterion(
                "F2",
                "spread of the distance to metals",
                self.metals_distance_spread,
                Tiles,
                AtMost(3.0),
            ),
            criterion(
                "F2",
                "spread of the distance to water",
                self.water_distance_spread,
                Tiles,
                AtMost(3.0),
            ),
            criterion(
                "F3",
                "poorest nation over the richest",
                self.poorest_over_richest,
                Ratio,
                AtLeast(0.5),
            ),
            criterion(
                "F4",
                "endowment Gini",
                self.endowment_gini,
                Ratio,
                AtMost(0.2),
            ),
            criterion(
                "F5",
                "nations with oil",
                self.nations_with_oil,
                Share,
                Within(0.4, 0.85),
            ),
            criterion(
                "F5",
                "nations with uranium",
                self.nations_with_uranium,
                Share,
                Within(0.25, 0.7),
            ),
            criterion(
                "F6",
                "oil and uranium out of every territory",
                self.scarce_outside_territories,
                Share,
                AtLeast(0.8),
            ),
            criterion(
                "F7",
                "biggest sphere's oil",
                self.biggest_oil_holding,
                Share,
                AtMost(0.4),
            ),
            criterion(
                "F7",
                "biggest sphere's uranium",
                self.biggest_uranium_holding,
                Share,
                AtMost(0.4),
            ),
        ]
    }

    /// Criteria met over criteria judged.
    pub fn score(&self) -> (usize, usize) {
        score(&self.criteria())
    }
}

impl fmt::Display for DepositsSurvey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for criterion in self.criteria() {
            writeln!(f, "{criterion}")?;
        }
        let (met, judged) = self.score();
        write!(f, "{met} of {judged} criteria met")
    }
}

/// Hexes from a tile to the nearest one `is_target` picks, if any within `reach`.
fn nearest(
    surface: &PlanetSurface,
    from: TileIndex,
    is_target: impl Fn(TileIndex) -> bool,
    reach: u32,
) -> Option<u32> {
    let mut distance = vec![u32::MAX; surface.tiles_count()];
    distance[from as usize] = 0;
    let mut queue = VecDeque::from([from]);
    while let Some(index) = queue.pop_front() {
        let reached = distance[index as usize];
        if reached > 0 && is_target(index) {
            return Some(reached);
        }
        if reached == reach {
            continue;
        }
        for neighbor in surface.neighbors(index) {
            if distance[neighbor as usize] == u32::MAX {
                distance[neighbor as usize] = reached + 1;
                queue.push_back(neighbor);
            }
        }
    }
    None
}

/// The Clark-Evans ratio of the deposits: the mean distance from each to its nearest neighbor,
/// over the mean a random spray of the same density over the land would give. `NaN` under two
/// deposits.
///
/// On a hex grid, the random mean is not the plane's `0.5 / √density`: distances are whole
/// numbers of hexes. With each tile holding a deposit with probability `p`, the nearest one is at
/// least `d` hexes away when the `3(d-1)² + 3(d-1)` tiles closer are all empty, and the mean is the
/// sum of those chances over `d`.
fn clark_evans(surface: &PlanetSurface, deposits: &GeneratedDeposits, land: usize) -> f64 {
    let all = deposits.deposits();
    if all.len() < 2 || land == 0 {
        return f64::NAN;
    }
    let mut per_tile = vec![0u16; surface.tiles_count()];
    for deposit in all {
        per_tile[deposit.tile() as usize] += 1;
    }
    let total: u32 = all
        .iter()
        .map(|deposit| {
            if per_tile[deposit.tile() as usize] > 1 {
                0
            } else {
                nearest(
                    surface,
                    deposit.tile(),
                    |index| per_tile[index as usize] > 0,
                    u32::MAX,
                )
                .unwrap_or(0)
            }
        })
        .sum();
    let observed = f64::from(total) / all.len() as f64;
    let empty = 1.0 - (all.len() as f64 / land as f64).min(1.0);
    let expected: f64 = (0..)
        .map(|closer: i32| empty.powi(3 * closer * closer + 3 * closer))
        .take_while(|&chance| chance > 1e-9)
        .sum();
    observed / expected.max(1.0)
}

#[cfg(test)]
mod tests {
    use project_x_core::game::{
        rng::{GameRng, RngPurpose},
        state::TileIndex,
    };
    use rand::RngExt;

    use super::{DepositsSurvey, clark_evans};
    use crate::{
        deposits::{
            DepositOrigin, DepositsGeneratorSettings, GeneratedDeposit, GeneratedDeposits,
            settings::DepositKind,
            tests::{planet, settle},
        },
        map::{Criterion, Target, survey::tests::tabulate},
    };

    fn deposits_on(tiles: &[TileIndex]) -> GeneratedDeposits {
        GeneratedDeposits {
            deposits: tiles
                .iter()
                .map(|&tile| GeneratedDeposit {
                    kind: DepositKind::Metals,
                    tile,
                    richness: 1,
                    origin: DepositOrigin::Natural,
                })
                .collect(),
            report: Default::default(),
        }
    }

    /// Surveys the deposits of the default planet over many seeds, with 8 nations. Run with:
    /// `cargo test -p project_x_procgen view_survey --release -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn view_survey_of_deposits() {
        let seeds: Vec<u32> = (0..32).map(|i| i * 7919 + 3).collect();
        let surveys: Vec<Vec<Criterion>> = seeds
            .iter()
            .map(|&seed| {
                let surface = planet(seed, 80, 52);
                let (nations, deposits) = settle(&surface, u64::from(seed), 8);
                let settings = DepositsGeneratorSettings::new(u64::from(seed));
                DepositsSurvey::of(&surface, &nations, &deposits, &settings).criteria()
            })
            .collect();
        println!("deposits: {} seeds, 80 x 52, 8 nations\n", seeds.len());
        print!("{}", tabulate(&surveys));
    }

    #[test]
    fn test_clark_evans_tells_clusters_from_a_random_spray() {
        let surface = planet(3, 80, 52);
        let land: Vec<TileIndex> = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.is_land(index))
            .collect();
        let mut rng = GameRng::new(1, RngPurpose::DepositGeneration);
        let mut spray: Vec<TileIndex> = (0..150)
            .map(|_| land[rng.random_range(0..land.len())])
            .collect();
        spray.sort_unstable();
        spray.dedup();
        let random = clark_evans(&surface, &deposits_on(&spray), land.len());
        // a heap: every deposit next to another
        let heap: Vec<TileIndex> = land.iter().copied().take(150).collect();
        let heaped = clark_evans(&surface, &deposits_on(&heap), land.len());
        println!("random spray {random:.2}, heap {heaped:.2}");
        // the coasts cut neighbors off: a random spray over real land reads a little over 1.0
        assert!((0.9..1.35).contains(&random), "random spray {random}");
        assert!(heaped < 0.6, "heap {heaped}");
    }

    #[test]
    fn test_deposits_survey_of_a_settled_planet() {
        let surface = planet(3, 40, 26);
        let (nations, deposits) = settle(&surface, 3, 6);
        let survey = DepositsSurvey::of(
            &surface,
            &nations,
            &deposits,
            &DepositsGeneratorSettings::new(3),
        );
        println!("{survey}");
        assert_eq!(survey.guarantees_met, 1.0);
        assert_eq!(survey.on_ice, 0.0);
        assert!(survey.criteria().iter().all(|c| c.target != Target::Info));
        let (met, judged) = survey.score();
        assert!(met <= judged && judged > 0);
    }
}
