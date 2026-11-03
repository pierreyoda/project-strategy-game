//! The deposits geology alone puts down: clusters seeded where the ground suits each kind best,
//! grown over the best ground around their seed.

use project_x_core::game::{rng::GameRng, state::TileIndex};
use rand::RngExt;

use super::{
    DepositOrigin, GeneratedDeposit,
    geology::Geology,
    settings::{DepositKind, DepositsGeneratorSettings, KindSettings, PerKind},
};
use crate::{map::PlanetSurface, nations::GeneratedNations};

/// A cluster grows onto ground at least this suitable, as a share of its seed's.
const CLUSTER_GROUND_SHARE: f64 = 0.5;

/// The deposits as they are being laid, one per tile at most.
pub struct Ledger {
    pub deposits: Vec<GeneratedDeposit>,
    /// Records which tiles already have a deposit, or are reserved.
    occupied: Vec<bool>,
}

impl Ledger {
    pub fn new(tiles_count: usize) -> Self {
        Self {
            deposits: Vec::new(),
            occupied: vec![false; tiles_count],
        }
    }

    pub fn is_free(&self, index: TileIndex) -> bool {
        !self.occupied[index as usize]
    }

    /// Keep the capitals free of any deposit to come.
    pub fn reserve_capitals(&mut self, surface: &PlanetSurface, nations: &GeneratedNations) {
        for nation in nations.nations() {
            let capital = surface
                .index(nation.capital())
                .expect("a capital is on the map");
            self.occupied[capital as usize] = true;
        }
    }

    /// Lay a deposit. Only a guarantee with nowhere else to go lays one on a taken tile.
    pub fn lay(&mut self, deposit: GeneratedDeposit) {
        self.occupied[deposit.tile as usize] = true;
        self.deposits.push(deposit);
    }
}

/// Lay the natural deposits of every kind, the rarest first. Returns how many times the seed
/// spacing of each kind had to relax.
pub fn place_naturally(
    surface: &PlanetSurface,
    geology: &Geology,
    settings: &DepositsGeneratorSettings,
    rng: &mut GameRng,
    ledger: &mut Ledger,
) -> PerKind<u32> {
    let land: Vec<TileIndex> = (0..surface.tiles_count() as TileIndex)
        .filter(|&index| surface.is_land(index))
        .collect();
    let mut relaxations = PerKind::default();
    for kind in DepositKind::PLACEMENT_ORDER {
        *relaxations.of_mut(kind) = Placement {
            surface,
            geology,
            settings,
            kind,
            kind_settings: settings.kind(kind),
            rng: &mut *rng,
            ledger: &mut *ledger,
        }
        .run(&land);
    }
    relaxations
}

struct Placement<'a> {
    surface: &'a PlanetSurface,
    geology: &'a Geology,
    settings: &'a DepositsGeneratorSettings,
    kind: DepositKind,
    kind_settings: &'a KindSettings,
    rng: &'a mut GameRng,
    ledger: &'a mut Ledger,
}

impl Placement<'_> {
    fn run(mut self, land: &[TileIndex]) -> u32 {
        let target = (self.kind_settings.density * land.len() as f64 / 100.0).round() as usize;
        let mut spacing = self.kind_settings.seed_spacing;
        let mut relaxations = 0;
        let mut nearest_seed = vec![u32::MAX; self.surface.tiles_count()];
        let mut placed = 0;
        while placed < target {
            let candidates: Vec<TileIndex> = land
                .iter()
                .copied()
                .filter(|&index| nearest_seed[index as usize] >= spacing && self.fits(index))
                .collect();
            let Some(seed) = self.pick(&candidates) else {
                if spacing <= 1 {
                    break;
                }
                let relaxed = (f64::from(spacing) * self.settings.get_spacing_relaxation()).floor();
                spacing = (relaxed as u32).clamp(1, spacing - 1);
                relaxations += 1;
                continue;
            };
            self.lay(seed);
            placed += 1;
            for (index, distance) in self.surface.within(seed, self.kind_settings.seed_spacing) {
                let nearest = &mut nearest_seed[index as usize];
                *nearest = (*nearest).min(distance);
            }

            // a cluster keeps to ground nearly as good as its seed's: an oil field stops at the
            // foothills rather than spill into them
            let floor = self.geology.suitability(self.kind, seed) * CLUSTER_GROUND_SHARE;
            let size = self
                .rng
                .random_range(self.kind_settings.cluster_min..=self.kind_settings.cluster_max);
            for _ in 1..size.min(target - placed + 1) {
                let around: Vec<TileIndex> = self
                    .surface
                    .within(seed, self.kind_settings.cluster_reach)
                    .into_iter()
                    .map(|(index, _)| index)
                    .filter(|&index| {
                        self.fits(index) && self.geology.suitability(self.kind, index) >= floor
                    })
                    .collect();
                let Some(tile) = self.pick(&around) else {
                    break;
                };
                self.lay(tile);
                placed += 1;
            }
        }
        relaxations
    }

    /// Whether the tile can take a deposit of the kind naturally.
    fn fits(&self, index: TileIndex) -> bool {
        self.ledger.is_free(index)
            && self.geology.suitability(self.kind, index) >= self.settings.get_min_suitability()
    }

    /// A tile drawn among the candidates, by their suitability raised to the selectivity.
    fn pick(&mut self, candidates: &[TileIndex]) -> Option<TileIndex> {
        let selectivity = self.settings.get_selectivity();
        let weight =
            |index: TileIndex| self.geology.suitability(self.kind, index).powi(selectivity);
        let total: f64 = candidates.iter().map(|&index| weight(index)).sum();
        if candidates.is_empty() || total <= 0.0 {
            return None;
        }
        let mut drawn = self.rng.random::<f64>() * total;
        for &index in candidates {
            drawn -= weight(index);
            if drawn < 0.0 {
                return Some(index);
            }
        }
        // rounding left a sliver past the last one
        candidates.last().copied()
    }

    fn lay(&mut self, tile: TileIndex) {
        let suitability = self.geology.suitability(self.kind, tile);
        let luck = 0.6 + 0.4 * self.rng.random::<f64>();
        let span = f64::from(self.kind_settings.richness_max - self.kind_settings.richness_min);
        let richness = self.kind_settings.richness_min + (span * suitability * luck).round() as u16;
        self.ledger.lay(GeneratedDeposit {
            kind: self.kind,
            tile,
            richness,
            origin: DepositOrigin::Natural,
        });
    }
}

/// The richness of a deposit laid on purpose rather than by chance - a guarantee, a top-up: never
/// under the middle of the kind's range, so that it is never a token.
pub fn assured_richness(kind_settings: &KindSettings, suitability: f64) -> u16 {
    let median = kind_settings.richness_median();
    let span = f64::from(kind_settings.richness_max - median);
    median + (span * suitability.clamp(0.0, 1.0)).round() as u16
}

#[cfg(test)]
mod tests {
    use project_x_core::game::rng::{GameRng, RngPurpose};

    use super::{Ledger, assured_richness, place_naturally};
    use crate::{
        deposits::{
            geology::Geology,
            settings::{DepositKind, DepositsGeneratorSettings},
        },
        map::{TerrainKind, surface::tests::surface_from_art},
    };

    fn lay(art: &str, seed: u64) -> (crate::map::PlanetSurface, Ledger) {
        let surface = surface_from_art(art);
        let geology = Geology::survey(&surface);
        let settings = DepositsGeneratorSettings::new(seed);
        let mut rng = GameRng::new(seed, RngPurpose::DepositGeneration);
        let mut ledger = Ledger::new(surface.tiles_count());
        place_naturally(&surface, &geology, &settings, &mut rng, &mut ledger);
        (surface, ledger)
    }

    fn continent() -> String {
        let row = "~^^:......................:^^~~";
        vec![row; 20].join("|")
    }

    #[test]
    fn test_natural_deposits_follow_the_density_one_per_tile() {
        let (surface, ledger) = lay(&continent(), 1);
        let land = (0..surface.tiles_count() as u32)
            .filter(|&index| surface.is_land(index))
            .count();
        let settings = DepositsGeneratorSettings::default();
        for kind in DepositKind::ALL {
            let expected = (settings.kind(kind).density * land as f64 / 100.0).round() as usize;
            let count = ledger.deposits.iter().filter(|d| d.kind == kind).count();
            assert_eq!(count, expected, "{kind:?}");
        }
        let mut tiles: Vec<u32> = ledger.deposits.iter().map(|d| d.tile).collect();
        tiles.sort_unstable();
        tiles.dedup();
        assert_eq!(tiles.len(), ledger.deposits.len(), "one deposit per tile");
        assert!(ledger.deposits.iter().all(|d| surface.is_land(d.tile)));
    }

    #[test]
    fn test_natural_deposits_keep_to_their_ground() {
        let (surface, ledger) = lay(&continent(), 2);
        let on = |kind: DepositKind, terrain: TerrainKind| {
            ledger
                .deposits
                .iter()
                .filter(|d| d.kind == kind && surface.terrain_at(d.tile) == terrain)
                .count()
        };
        let metals = ledger
            .deposits
            .iter()
            .filter(|d| d.kind == DepositKind::Metals)
            .count();
        let rough = on(DepositKind::Metals, TerrainKind::Mountains)
            + on(DepositKind::Metals, TerrainKind::Hills);
        assert!(
            rough * 2 > metals,
            "{rough} of {metals} metals in the ranges"
        );
        assert_eq!(on(DepositKind::Oil, TerrainKind::Mountains), 0);
    }

    #[test]
    fn test_natural_deposits_are_the_same_for_the_same_seed() {
        let (_, first) = lay(&continent(), 7);
        let (_, again) = lay(&continent(), 7);
        let (_, other) = lay(&continent(), 8);
        assert_eq!(first.deposits, again.deposits);
        assert_ne!(first.deposits, other.deposits);
    }

    #[test]
    fn test_natural_placement_stops_when_the_land_is_full() {
        // three tiles of land cannot take every deposit asked of them
        let (_, ledger) = lay("~.:^~", 0);
        assert!(ledger.deposits.len() <= 3);
    }

    #[test]
    fn test_assured_richness_starts_at_the_middle_of_the_range() {
        let kind = *DepositsGeneratorSettings::default().kind(DepositKind::Metals);
        assert_eq!(assured_richness(&kind, 0.0), kind.richness_median());
        assert_eq!(assured_richness(&kind, 1.0), kind.richness_max);
    }
}
