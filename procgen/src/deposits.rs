//! Deposits generation: the water, metals, oil and uranium in the ground of a generated planet.
//!
//! Three passes over the same row-major tiles as the client's buffers, none of them touching a
//! `World`:
//! 1. **natural**: clusters laid where the geology suits each kind, from the surface alone - the
//!    same planet gets the same geology whatever the nations on it, which are founded after it,
//!    their capitals clear of every deposit (see `settle`);
//! 2. **guarantees**: every nation finds water and metals within `GUARANTEE_REACH` of its turn-1
//!    borders, whatever the geology dealt it;
//! 3. **balance**: the poorest nations get topped up with water and metals.
//!
//! The last two only add deposits, never on a capital, and mark them, so the survey can count what
//! the corrections cost the realism. The output is data for the caller to write into the world.

use project_x_core::game::{
    rng::{GameRng, RngPurpose},
    state::TileIndex,
};

pub use self::{
    balance::{Assessor, Endowment, HALF_WORTH_DISTANCE, TOPPED_UP_KINDS, kind_weight},
    geology::Geology,
    guarantees::{GUARANTEE_REACH, GUARANTEED_KINDS, GuaranteeReport, Zone},
    settings::{DepositKind, DepositsGeneratorSettings, KindSettings, PerKind},
    survey::DepositsSurvey,
};
use crate::{
    map::PlanetSurface,
    nations::{GeneratedNations, NationsGenerator, NationsGeneratorSettings},
};

mod balance;
mod geology;
mod guarantees;
mod placement;
pub mod renderers;
mod settings;
mod survey;

/// Why a deposit is where it is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum DepositOrigin {
    /// Laid by the geology.
    Natural,
    /// Laid so that a nation finds what it is guaranteed.
    Guaranteed,
    /// Laid to lift a resources-poor nation towards the others.
    Balancing,
}

/// A deposit, as generated.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GeneratedDeposit {
    kind: DepositKind,
    tile: TileIndex,
    /// Its stock - or for water, which springs back, its recharge per turn.
    richness: u16,
    origin: DepositOrigin,
}

impl GeneratedDeposit {
    pub fn kind(&self) -> DepositKind {
        self.kind
    }

    pub fn tile(&self) -> TileIndex {
        self.tile
    }

    pub fn richness(&self) -> u16 {
        self.richness
    }

    pub fn origin(&self) -> DepositOrigin {
        self.origin
    }
}

/// How the generation went.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DepositsReport {
    /// Natural deposits laid, per kind.
    pub natural: PerKind<usize>,
    /// How many times the seed spacing of each kind relaxed.
    pub relaxations: PerKind<u32>,
    pub guarantees: GuaranteeReport,
    /// Deposits laid by the balance pass.
    pub top_ups: usize,
}

/// The deposits of a planet.
#[derive(Clone, Debug)]
pub struct GeneratedDeposits {
    /// The natural ones first, then the guaranteed ones, then the top-ups.
    deposits: Vec<GeneratedDeposit>,
    report: DepositsReport,
}

impl GeneratedDeposits {
    pub fn deposits(&self) -> &[GeneratedDeposit] {
        &self.deposits
    }

    pub fn report(&self) -> &DepositsReport {
        &self.report
    }

    /// The deposits of one kind.
    pub fn of_kind(&self, kind: DepositKind) -> impl Iterator<Item = &GeneratedDeposit> {
        self.deposits
            .iter()
            .filter(move |deposit| deposit.kind == kind)
    }

    /// The deposits on a tile: one at most, but for a guarantee that had nowhere else to go.
    pub fn on(&self, index: TileIndex) -> impl Iterator<Item = &GeneratedDeposit> {
        self.deposits
            .iter()
            .filter(move |deposit| deposit.tile == index)
    }
}

#[derive(Debug)]
pub struct DepositsGenerator {
    settings: DepositsGeneratorSettings,
}

impl DepositsGenerator {
    pub fn from_settings(settings: DepositsGeneratorSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &DepositsGeneratorSettings {
        &self.settings
    }

    /// The natural deposits of a planet: from its surface alone, before any nation is founded.
    pub fn generate_natural(&self, surface: &PlanetSurface) -> GeneratedDeposits {
        self.lay_naturally(surface, &Geology::survey(surface))
    }

    /// The deposits of a settled planet: its natural ones, then the guarantees and top-ups the
    /// nations need. Neither goes onto a capital.
    pub fn settle(
        &self,
        surface: &PlanetSurface,
        natural: GeneratedDeposits,
        nations: &GeneratedNations,
    ) -> GeneratedDeposits {
        let settings = &self.settings;
        let geology = Geology::survey(surface);
        let GeneratedDeposits {
            deposits,
            mut report,
        } = natural;
        let mut ledger = placement::Ledger::new(surface.tiles_count());
        for deposit in deposits {
            ledger.lay(deposit);
        }
        ledger.reserve_capitals(surface, nations);
        report.guarantees =
            guarantees::guarantee(surface, &geology, nations, settings, &mut ledger);
        report.top_ups = balance::top_up(surface, &geology, nations, settings, &mut ledger);
        GeneratedDeposits {
            deposits: ledger.deposits,
            report,
        }
    }

    fn lay_naturally(&self, surface: &PlanetSurface, geology: &Geology) -> GeneratedDeposits {
        let settings = &self.settings;
        let mut rng = GameRng::new(settings.get_seed(), RngPurpose::DepositGeneration);
        let mut ledger = placement::Ledger::new(surface.tiles_count());
        let relaxations =
            placement::place_naturally(surface, geology, settings, &mut rng, &mut ledger);
        let mut natural = PerKind::default();
        for deposit in &ledger.deposits {
            *natural.of_mut(deposit.kind) += 1;
        }
        GeneratedDeposits {
            deposits: ledger.deposits,
            report: DepositsReport {
                natural,
                relaxations,
                ..DepositsReport::default()
            },
        }
    }
}

/// Settle a planet: its natural deposits, then its nations, clear of them, then the deposits the
/// nations are guaranteed or topped up with. In that order, so that no capital sits on a deposit
/// and the natural deposits still owe nothing to the nations.
pub fn settle(
    surface: &PlanetSurface,
    nations_settings: &NationsGeneratorSettings,
    deposits_settings: &DepositsGeneratorSettings,
) -> (GeneratedNations, GeneratedDeposits) {
    let generator = DepositsGenerator::from_settings(deposits_settings.clone());
    let natural = generator.generate_natural(surface);
    let taken: Vec<TileIndex> = natural.deposits().iter().map(|d| d.tile()).collect();
    let nations = NationsGenerator::from_settings(nations_settings.clone())
        .generate_clear_of(surface, &taken);
    let deposits = generator.settle(surface, natural, &nations);
    (nations, deposits)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        DepositOrigin, DepositsGenerator, DepositsGeneratorSettings, GUARANTEED_KINDS,
        GeneratedDeposits, Zone, guarantees::territory_owners,
    };
    use crate::{
        map::{MapGenerator, MapGeneratorSettings, PlanetSurface},
        nations::{GeneratedNations, NationsGeneratorSettings},
    };

    pub(crate) fn planet(seed: u32, columns: i16, rows: i16) -> PlanetSurface {
        let settings = MapGeneratorSettings::new(seed, columns, rows);
        let map = MapGenerator::from_settings(MapGeneratorSettings::new(seed, columns, rows))
            .generate(|_| {});
        PlanetSurface::classify(&map, &settings)
    }

    pub(crate) fn settle(
        surface: &PlanetSurface,
        seed: u64,
        nations: u16,
    ) -> (GeneratedNations, GeneratedDeposits) {
        super::settle(
            surface,
            &NationsGeneratorSettings::new(seed, nations),
            &DepositsGeneratorSettings::new(seed),
        )
    }

    /// Whether every nation finds every guaranteed kind within its zone.
    pub(crate) fn guarantees_hold(
        surface: &PlanetSurface,
        nations: &GeneratedNations,
        deposits: &GeneratedDeposits,
    ) -> bool {
        let owners = territory_owners(surface, nations);
        nations.nations().iter().all(|nation| {
            let zone = Zone::of(surface, nations, &owners, nation);
            GUARANTEED_KINDS.iter().all(|&(kind, count)| {
                deposits
                    .of_kind(kind)
                    .filter(|d| zone.contains(d.tile()))
                    .count()
                    >= count
            })
        })
    }

    #[test]
    fn test_deposits_generator_lays_deposits_on_a_planet() {
        let surface = planet(3, 40, 26);
        let (nations, deposits) = settle(&surface, 3, 6);
        assert!(!deposits.deposits().is_empty());
        assert!(
            deposits
                .deposits()
                .iter()
                .all(|d| surface.is_land(d.tile()))
        );
        assert!(guarantees_hold(&surface, &nations, &deposits));
        let report = deposits.report();
        let natural: usize = super::DepositKind::ALL
            .iter()
            .map(|&kind| *report.natural.of(kind))
            .sum();
        assert_eq!(
            natural + report.guarantees.laid + report.top_ups,
            deposits.deposits().len()
        );
    }

    #[test]
    fn test_deposits_generator_is_deterministic() {
        let surface = planet(42, 32, 20);
        let (_, first) = settle(&surface, 1, 4);
        let (_, again) = settle(&surface, 1, 4);
        assert_eq!(first.deposits(), again.deposits());
    }

    #[test]
    fn test_natural_deposits_do_not_depend_on_the_nations() {
        let surface = planet(5, 32, 20);
        let natural = |deposits: &GeneratedDeposits| {
            deposits
                .deposits()
                .iter()
                .filter(|d| d.origin() == DepositOrigin::Natural)
                .copied()
                .collect::<Vec<_>>()
        };
        let (_, few) = settle(&surface, 9, 2);
        let (_, many) = settle(&surface, 9, 12);
        let none = DepositsGenerator::from_settings(DepositsGeneratorSettings::new(9))
            .generate_natural(&surface);
        assert_eq!(natural(&few), natural(&many));
        assert_eq!(natural(&few), none.deposits());
        assert_eq!(none.report().guarantees.laid, 0);
    }

    /// The guarantees are a promise: every nation, on every seed, however many share the planet.
    #[test]
    fn test_guarantees_hold_on_every_seed() {
        for seed in 0..12u32 {
            let surface = planet(seed * 7919 + 3, 40, 26);
            for nations in [1, 8, 24] {
                let (generated, deposits) = settle(&surface, u64::from(seed), nations);
                assert!(
                    guarantees_hold(&surface, &generated, &deposits),
                    "seed {seed}, {nations} nations"
                );
            }
        }
    }
}
