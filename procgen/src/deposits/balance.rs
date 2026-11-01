//! How rich each nation starts, and the top-ups that keep the poorest from falling too far behind.
//!
//! Only additive: thinning out a rich nation's deposits would make the geology depend on politics.
//! Only water and metals: oil and uranium are meant to be scarce and fought over.

use project_x_core::game::state::TileIndex;

use super::{
    DepositOrigin, GeneratedDeposit,
    geology::Geology,
    guarantees::capital_distances,
    placement::{Ledger, assured_richness},
    settings::{DepositKind, DepositsGeneratorSettings, PerKind},
};
use crate::{map::PlanetSurface, nations::GeneratedNations};

/// A deposit this many hexes from the capital counts for half one next door.
pub const HALF_WORTH_DISTANCE: f64 = 8.0;

/// The kinds the top-ups may add.
pub const TOPPED_UP_KINDS: [DepositKind; 2] = [DepositKind::Water, DepositKind::Metals];

/// How much each kind weighs in a nation's overall endowment.
pub fn kind_weight(kind: DepositKind) -> f64 {
    match kind {
        DepositKind::Water | DepositKind::Metals => 1.0,
        DepositKind::Oil => 0.75,
        DepositKind::Uranium => 0.5,
    }
}

/// What a nation's sphere holds, per kind: the richness of each deposit over the most its kind
/// can hold, weighed down by its distance to the capital.
pub type Endowment = PerKind<f64>;

pub fn overall(endowment: &Endowment) -> f64 {
    DepositKind::ALL
        .iter()
        .map(|&kind| kind_weight(kind) * endowment.of(kind))
        .sum()
}

/// Measures the endowments of the nations, in their order.
pub struct Assessor<'a> {
    nations: &'a GeneratedNations,
    settings: &'a DepositsGeneratorSettings,
    /// Hexes from each nation's capital, per tile.
    from_capital: Vec<Vec<u32>>,
}

impl<'a> Assessor<'a> {
    pub fn new(
        surface: &PlanetSurface,
        nations: &'a GeneratedNations,
        settings: &'a DepositsGeneratorSettings,
    ) -> Self {
        let from_capital = nations
            .nations()
            .iter()
            .map(|nation| {
                let capital = surface
                    .index(nation.capital())
                    .expect("a capital is on the map");
                capital_distances(surface, capital)
            })
            .collect();
        Self {
            nations,
            settings,
            from_capital,
        }
    }

    /// How much a deposit on the tile counts for the nation, from 1.0 on the capital down.
    pub fn worth(&self, nation: usize, index: TileIndex) -> f64 {
        let distance = f64::from(self.from_capital[nation][index as usize]);
        1.0 / (1.0 + distance / HALF_WORTH_DISTANCE)
    }

    pub fn endowments(&self, deposits: &[GeneratedDeposit]) -> Vec<Endowment> {
        let mut endowments = vec![Endowment::default(); self.nations.nations().len()];
        for deposit in deposits {
            let Some(owner) = self.nations.spheres().owner(deposit.tile) else {
                continue;
            };
            let Some(nation) = self
                .nations
                .nations()
                .iter()
                .position(|nation| nation.id() == owner)
            else {
                continue;
            };
            let most = f64::from(self.settings.kind(deposit.kind).richness_max.max(1));
            *endowments[nation].of_mut(deposit.kind) +=
                f64::from(deposit.richness) / most * self.worth(nation, deposit.tile);
        }
        endowments
    }
}

/// The median of some values, `NaN` when there are none.
pub fn median(values: impl Iterator<Item = f64>) -> f64 {
    let mut values: Vec<f64> = values.collect();
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}

/// Top up, one deposit at a time, the poorest nation while it is under the balance floor of the
/// median endowment: with the kind it lacks the most against the others, on the free tile of its
/// sphere where that kind is worth the most. Returns how many deposits it laid.
pub fn top_up(
    surface: &PlanetSurface,
    geology: &Geology,
    nations: &GeneratedNations,
    settings: &DepositsGeneratorSettings,
    ledger: &mut Ledger,
) -> usize {
    let assessor = Assessor::new(surface, nations, settings);
    let count = nations.nations().len();
    let mut used = vec![0; count];
    let mut laid = 0;
    loop {
        let endowments = assessor.endowments(&ledger.deposits);
        let totals: Vec<f64> = endowments.iter().map(overall).collect();
        let floor = settings.get_balance_floor() * median(totals.iter().copied());
        let Some(poorest) = (0..count)
            .filter(|&nation| used[nation] < settings.get_top_ups_per_nation())
            .filter(|&nation| totals[nation] < floor)
            .min_by(|&a, &b| totals[a].total_cmp(&totals[b]).then(a.cmp(&b)))
        else {
            return laid;
        };

        let lack = |kind: DepositKind| {
            let typical = median(endowments.iter().map(|endowment| *endowment.of(kind)));
            if typical > 0.0 {
                endowments[poorest].of(kind) / typical
            } else {
                1.0
            }
        };
        let kind = TOPPED_UP_KINDS
            .into_iter()
            .min_by(|&a, &b| lack(a).total_cmp(&lack(b)))
            .expect("some kind to top up");
        let value =
            |index: TileIndex| geology.suitability(kind, index) * assessor.worth(poorest, index);
        let best = nations.nations()[poorest]
            .sphere()
            .iter()
            .copied()
            .filter(|&index| ledger.is_free(index) && geology.suitability(kind, index) > 0.0)
            .max_by(|&a, &b| value(a).total_cmp(&value(b)).then(b.cmp(&a)));
        let Some(tile) = best else {
            // nowhere left in its sphere: it stays as it is
            used[poorest] = usize::MAX;
            continue;
        };
        ledger.lay(GeneratedDeposit {
            kind,
            tile,
            richness: assured_richness(settings.kind(kind), geology.suitability(kind, tile)),
            origin: DepositOrigin::Balancing,
        });
        used[poorest] += 1;
        laid += 1;
    }
}

/// The Gini coefficient of some values: 0.0 when all are equal, towards 1.0 when one holds all.
pub fn gini(values: &[f64]) -> f64 {
    let total: f64 = values.iter().sum();
    if values.is_empty() || total <= 0.0 {
        return f64::NAN;
    }
    let mut differences = 0.0;
    for a in values {
        for b in values {
            differences += (a - b).abs();
        }
    }
    differences / (2.0 * values.len() as f64 * total)
}

#[cfg(test)]
mod tests {
    use super::{gini, median, top_up};
    use crate::{
        deposits::{
            DepositOrigin, GeneratedDeposit,
            geology::Geology,
            placement::Ledger,
            settings::{DepositKind, DepositsGeneratorSettings},
        },
        map::surface::tests::surface_from_art,
        nations::{NationsGenerator, NationsGeneratorSettings},
    };

    #[test]
    fn test_median_and_gini() {
        assert_eq!(median([3.0, 1.0, 2.0].into_iter()), 2.0);
        assert_eq!(median([4.0, 1.0, 2.0, 3.0].into_iter()), 2.5);
        assert!(median(std::iter::empty()).is_nan());
        assert_eq!(gini(&[1.0, 1.0, 1.0]), 0.0);
        assert!((gini(&[0.0, 0.0, 0.0, 1.0]) - 0.75).abs() < 1e-9);
    }

    #[test]
    fn test_top_up_lifts_the_poorest_nation_only() {
        // two nations on one plain, every deposit on the western one's side
        let surface = surface_from_art(&["~..................~"; 6].join("|"));
        let nations = NationsGenerator::from_settings(
            NationsGeneratorSettings::new(0, 2).with_site_jitter(1),
        )
        .generate(&surface);
        let geology = Geology::survey(&surface);
        let settings = DepositsGeneratorSettings::default();
        let mut ledger = Ledger::new(surface.tiles_count());
        let rich = nations.nations()[0].sphere().to_vec();
        for &tile in rich.iter().take(4) {
            ledger.lay(GeneratedDeposit {
                kind: DepositKind::Metals,
                tile,
                richness: 800,
                origin: DepositOrigin::Natural,
            });
        }
        let laid = top_up(&surface, &geology, &nations, &settings, &mut ledger);
        assert!(laid > 0 && laid <= settings.get_top_ups_per_nation());
        for deposit in &ledger.deposits[4..] {
            assert_eq!(deposit.origin, DepositOrigin::Balancing);
            assert!(nations.nations()[1].sphere().contains(&deposit.tile));
            assert!(matches!(
                deposit.kind,
                DepositKind::Water | DepositKind::Metals
            ));
        }
    }
}
