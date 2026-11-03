//! What every nation is sure to find at home, whatever the geology dealt it.

use std::collections::VecDeque;

use project_x_core::{game::state::TileIndex, simulation::ids::NationId};

use super::{
    DepositOrigin, GeneratedDeposit,
    geology::Geology,
    placement::{Ledger, assured_richness},
    settings::{DepositKind, DepositsGeneratorSettings},
};
use crate::{
    map::PlanetSurface,
    nations::{GeneratedNation, GeneratedNations},
};

/// How far outside a nation's turn-1 borders a guaranteed deposit may sit, in hexes.
pub const GUARANTEE_REACH: u32 = 2;

/// The kinds every nation is guaranteed within its zone, and how many of each.
pub const GUARANTEED_KINDS: [(DepositKind, usize); 2] =
    [(DepositKind::Water, 1), (DepositKind::Metals, 1)];

/// Where a nation's guarantees may lie: the land of its own landmass within `GUARANTEE_REACH` of
/// its turn-1 territory, never in a rival's territory.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    /// Within the nation's own sphere, or no one's; in row-major order.
    pub preferred: Vec<TileIndex>,
    /// Within a rival's sphere, only used when nothing else is left; in row-major order.
    pub fallback: Vec<TileIndex>,
}

impl Zone {
    pub fn of(
        surface: &PlanetSurface,
        nations: &GeneratedNations,
        territory_owner: &[Option<NationId>],
        nation: &GeneratedNation,
    ) -> Self {
        let mut distance = vec![u32::MAX; surface.tiles_count()];
        let mut queue = VecDeque::new();
        for &index in nation.territory() {
            distance[index as usize] = 0;
            queue.push_back(index);
        }
        let (mut preferred, mut fallback) = (Vec::new(), Vec::new());
        while let Some(index) = queue.pop_front() {
            let reached = distance[index as usize];
            let ours = territory_owner[index as usize].is_none_or(|owner| owner == nation.id());
            if ours && nations.landmasses().of(index) == Some(nation.landmass()) {
                match nations.spheres().owner(index) {
                    Some(owner) if owner != nation.id() => fallback.push(index),
                    _ => preferred.push(index),
                }
            }
            if reached == GUARANTEE_REACH {
                continue;
            }
            for neighbor in surface.neighbors(index) {
                if distance[neighbor as usize] == u32::MAX {
                    distance[neighbor as usize] = reached + 1;
                    queue.push_back(neighbor);
                }
            }
        }
        preferred.sort_unstable();
        fallback.sort_unstable();
        Self {
            preferred,
            fallback,
        }
    }

    pub fn contains(&self, index: TileIndex) -> bool {
        self.preferred.binary_search(&index).is_ok() || self.fallback.binary_search(&index).is_ok()
    }
}

/// The nation owning each tile on turn 1, row-major.
pub fn territory_owners(
    surface: &PlanetSurface,
    nations: &GeneratedNations,
) -> Vec<Option<NationId>> {
    let mut owners = vec![None; surface.tiles_count()];
    for nation in nations.nations() {
        for &index in nation.territory() {
            owners[index as usize] = Some(nation.id());
        }
    }
    owners
}

/// What the guarantees had to add.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct GuaranteeReport {
    /// Deposits laid by the guarantees.
    pub laid: usize,
    /// Nations that needed at least one.
    pub nations_helped: usize,
}

/// Lay, for every nation in turn, the guaranteed deposits its zone lacks: on the zone's best free
/// tile for the kind, the nearest to the capital on ties. The capitals are reserved, never free.
/// With every other tile of the zone taken, the deposit shares the best of them; only on a
/// one-tile island does it go onto the capital itself - the guarantee holds even there.
pub fn guarantee(
    surface: &PlanetSurface,
    geology: &Geology,
    nations: &GeneratedNations,
    settings: &DepositsGeneratorSettings,
    ledger: &mut Ledger,
) -> GuaranteeReport {
    let owners = territory_owners(surface, nations);
    let capitals: Vec<TileIndex> = nations
        .nations()
        .iter()
        .map(|nation| {
            surface
                .index(nation.capital())
                .expect("a capital is on the map")
        })
        .collect();
    let mut report = GuaranteeReport::default();
    for nation in nations.nations() {
        let zone = Zone::of(surface, nations, &owners, nation);
        let capital = surface
            .index(nation.capital())
            .expect("a capital is on the map");
        let from_capital = capital_distances(surface, capital);
        let mut helped = false;
        for (kind, count) in GUARANTEED_KINDS {
            let found = ledger
                .deposits
                .iter()
                .filter(|deposit| deposit.kind == kind && zone.contains(deposit.tile))
                .count();
            for _ in found..count {
                let best = |tiles: &[TileIndex], free_only: bool| {
                    tiles
                        .iter()
                        .copied()
                        .filter(|&index| {
                            if free_only {
                                ledger.is_free(index)
                            } else {
                                !capitals.contains(&index)
                            }
                        })
                        .max_by(|&a, &b| {
                            geology
                                .suitability(kind, a)
                                .total_cmp(&geology.suitability(kind, b))
                                .then(from_capital[b as usize].cmp(&from_capital[a as usize]))
                                .then(b.cmp(&a))
                        })
                };
                let tile = best(&zone.preferred, true)
                    .or_else(|| best(&zone.fallback, true))
                    .or_else(|| best(&zone.preferred, false))
                    .or_else(|| best(&zone.fallback, false))
                    .unwrap_or(capital);
                ledger.lay(GeneratedDeposit {
                    kind,
                    tile,
                    richness: assured_richness(
                        settings.kind(kind),
                        geology.suitability(kind, tile),
                    ),
                    origin: DepositOrigin::Guaranteed,
                });
                report.laid += 1;
                helped = true;
            }
        }
        report.nations_helped += usize::from(helped);
    }
    report
}

/// Hexes from the capital to every tile, `u32::MAX` beyond the poles' reach.
pub fn capital_distances(surface: &PlanetSurface, capital: TileIndex) -> Vec<u32> {
    let mut distance = vec![u32::MAX; surface.tiles_count()];
    for (index, d) in surface.within(capital, u32::MAX) {
        distance[index as usize] = d;
    }
    distance
}

#[cfg(test)]
mod tests {
    use project_x_core::simulation::ids::EntityIndex;

    use super::{GUARANTEE_REACH, GUARANTEED_KINDS, Zone, guarantee, territory_owners};
    use crate::{
        deposits::{
            DepositOrigin,
            geology::Geology,
            placement::Ledger,
            settings::{DepositKind, DepositsGeneratorSettings},
        },
        map::surface::tests::surface_from_art,
        nations::{NationsGenerator, NationsGeneratorSettings},
    };

    #[test]
    fn test_zone_reaches_past_the_borders_on_the_same_landmass_only() {
        // a plain, and an island two hexes off its east coast
        let surface = surface_from_art(
            "~.........~~.~|~.........~~.~|~.........~~.~|~.........~~.~|~.........~~.~",
        );
        let settings = NationsGeneratorSettings::new(0, 1)
            .with_border_radius(1)
            .with_site_jitter(1);
        let nations = NationsGenerator::from_settings(settings).generate(&surface);
        let owners = territory_owners(&surface, &nations);
        let nation = &nations.nations()[0];
        let zone = Zone::of(&surface, &nations, &owners, nation);

        let capital = surface.index(nation.capital()).unwrap();
        let within: Vec<u32> = surface
            .within(capital, 1 + GUARANTEE_REACH)
            .into_iter()
            .map(|(index, _)| index)
            .filter(|&index| surface.is_land(index) && index % 14 < 10)
            .collect();
        for index in within {
            assert!(zone.contains(index), "{index} within reach");
        }
        assert!(zone.fallback.is_empty(), "no rival");
        assert!(
            zone.preferred.iter().all(|&index| index % 14 != 12),
            "the island is across the water"
        );
    }

    #[test]
    fn test_every_nation_gets_its_guarantees_even_on_a_bare_plain() {
        // a featureless plain: metals have nowhere good to go
        let surface = surface_from_art(&["~..............~"; 8].join("|"));
        let nations = NationsGenerator::from_settings(
            // forests would move the capitals closer, and their zones would share deposits
            NationsGeneratorSettings::new(0, 3).with_forest_score(1.0),
        )
        .generate(&surface);
        let geology = Geology::survey(&surface);
        let mut ledger = Ledger::new(surface.tiles_count());
        ledger.reserve_capitals(&surface, &nations);
        let report = guarantee(
            &surface,
            &geology,
            &nations,
            &DepositsGeneratorSettings::default(),
            &mut ledger,
        );
        assert_eq!(report.nations_helped, 3);
        assert_eq!(report.laid, 6);
        let owners = territory_owners(&surface, &nations);
        for nation in nations.nations() {
            let zone = Zone::of(&surface, &nations, &owners, nation);
            for (kind, count) in GUARANTEED_KINDS {
                let found = ledger
                    .deposits
                    .iter()
                    .filter(|d| d.kind == kind && zone.contains(d.tile))
                    .count();
                assert!(found >= count, "{kind:?} for {}", nation.id().index());
            }
        }
        assert!(
            ledger
                .deposits
                .iter()
                .all(|d| d.origin == DepositOrigin::Guaranteed)
        );
    }

    #[test]
    fn test_guarantees_land_on_the_capital_of_a_one_tile_island() {
        let surface = surface_from_art("~~~~~|~~.~~|~~~~~");
        let settings = NationsGeneratorSettings::new(0, 1).with_min_landmass_weight(0.0);
        let nations = NationsGenerator::from_settings(settings).generate(&surface);
        let geology = Geology::survey(&surface);
        let mut ledger = Ledger::new(surface.tiles_count());
        ledger.reserve_capitals(&surface, &nations);
        guarantee(
            &surface,
            &geology,
            &nations,
            &DepositsGeneratorSettings::default(),
            &mut ledger,
        );
        let kinds: Vec<DepositKind> = ledger.deposits.iter().map(|d| d.kind).collect();
        assert_eq!(kinds, [DepositKind::Water, DepositKind::Metals]);
        assert!(ledger.deposits.iter().all(|d| d.tile == 7));
    }
}
