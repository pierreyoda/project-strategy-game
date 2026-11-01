use project_x_core::{game::rng::GameSeed, simulation::resources::Resource};

/// What a deposit holds: the resources found in the ground rather than produced.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DepositKind {
    Water,
    Metals,
    Oil,
    Uranium,
}

impl DepositKind {
    pub const ALL: [Self; 4] = [Self::Water, Self::Metals, Self::Oil, Self::Uranium];

    /// The order the natural placement goes in: the rarest first, so that the common kinds
    /// fill the tiles left rather than crowd out the few sites the rare ones have.
    pub const PLACEMENT_ORDER: [Self; 4] = [Self::Uranium, Self::Oil, Self::Metals, Self::Water];

    /// The resource it yields.
    pub fn resource(self) -> Resource {
        match self {
            Self::Water => Resource::Water,
            Self::Metals => Resource::Metals,
            Self::Oil => Resource::Oil,
            Self::Uranium => Resource::Uranium,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// A value per kind of deposit.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PerKind<T> {
    pub water: T,
    pub metals: T,
    pub oil: T,
    pub uranium: T,
}

impl<T> PerKind<T> {
    pub fn of(&self, kind: DepositKind) -> &T {
        match kind {
            DepositKind::Water => &self.water,
            DepositKind::Metals => &self.metals,
            DepositKind::Oil => &self.oil,
            DepositKind::Uranium => &self.uranium,
        }
    }

    pub fn of_mut(&mut self, kind: DepositKind) -> &mut T {
        match kind {
            DepositKind::Water => &mut self.water,
            DepositKind::Metals => &mut self.metals,
            DepositKind::Oil => &mut self.oil,
            DepositKind::Uranium => &mut self.uranium,
        }
    }
}

/// How one kind of deposit is spread over the planet.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct KindSettings {
    /// Deposits per 100 land tiles.
    pub density: f64,
    /// Deposits per cluster, from the seed of the cluster on.
    pub cluster_min: usize,
    pub cluster_max: usize,
    /// Radius around its seed a cluster grows within, in hexes.
    pub cluster_reach: u32,
    /// Minimum distance between two cluster seeds of this kind, in hexes.
    pub seed_spacing: u32,
    /// Range of a deposit's richness: its stock, or its recharge per turn for water.
    pub richness_min: u16,
    pub richness_max: u16,
}

impl KindSettings {
    /// The richness halfway through the range: the least a guaranteed deposit holds.
    pub fn richness_median(&self) -> u16 {
        self.richness_min + (self.richness_max - self.richness_min) / 2
    }
}

/// Settings of the deposits generator.
#[derive(Clone, Debug)]
pub struct DepositsGeneratorSettings {
    seed: GameSeed,
    kinds: PerKind<KindSettings>,
    /// Exponent on the suitability of a tile when drawing among them: the higher, the more
    /// strictly the deposits keep to the best ground.
    selectivity: i32,
    /// Below this suitability, a tile never draws a deposit naturally.
    min_suitability: f64,
    /// Factor the seed spacing relaxes by when no tile satisfies it.
    spacing_relaxation: f64,
    /// The balance pass tops up any nation under this share of the median endowment...
    balance_floor: f64,
    /// ...with at most this many deposits per nation.
    top_ups_per_nation: usize,
}

/// Tuned against the `WorldSurvey`, jointly with the map's and the nations' defaults, every
/// mandatory criterion held on every seed: on 80 seeds the tuning never saw, at 80 × 52 with 8
/// nations, the settlement meets about 90% of its criteria and the world about 81%.
impl Default for DepositsGeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            kinds: PerKind {
                water: KindSettings {
                    density: 4.0,
                    cluster_min: 1,
                    cluster_max: 2,
                    cluster_reach: 1,
                    seed_spacing: 5,
                    richness_min: 4,
                    richness_max: 12,
                },
                metals: KindSettings {
                    density: 4.0,
                    cluster_min: 1,
                    cluster_max: 3,
                    cluster_reach: 1,
                    seed_spacing: 5,
                    richness_min: 200,
                    richness_max: 800,
                },
                oil: KindSettings {
                    density: 1.5,
                    cluster_min: 3,
                    cluster_max: 6,
                    cluster_reach: 1,
                    seed_spacing: 12,
                    richness_min: 150,
                    richness_max: 600,
                },
                uranium: KindSettings {
                    density: 0.75,
                    cluster_min: 3,
                    cluster_max: 4,
                    cluster_reach: 1,
                    seed_spacing: 12,
                    richness_min: 50,
                    richness_max: 200,
                },
            },
            selectivity: 3,
            // only fair ground draws a deposit naturally: no oil in the foothills, no metals
            // on the coastal plains
            min_suitability: 0.2,
            spacing_relaxation: 0.75,
            balance_floor: 0.875,
            top_ups_per_nation: 8,
        }
    }
}

impl DepositsGeneratorSettings {
    pub fn new(seed: GameSeed) -> Self {
        Self {
            seed,
            ..Self::default()
        }
    }

    pub fn with_seed(self, seed: GameSeed) -> Self {
        Self { seed, ..self }
    }

    pub fn with_kind(mut self, kind: DepositKind, settings: KindSettings) -> Self {
        assert!(settings.density >= 0.0, "density >= 0");
        assert!(
            0 < settings.cluster_min && settings.cluster_min <= settings.cluster_max,
            "0 < cluster_min <= cluster_max"
        );
        assert!(
            settings.richness_min <= settings.richness_max,
            "richness_min <= richness_max"
        );
        *self.kinds.of_mut(kind) = settings;
        self
    }

    pub fn with_selectivity(self, selectivity: i32) -> Self {
        assert!(selectivity >= 0, "selectivity >= 0");
        Self {
            selectivity,
            ..self
        }
    }

    pub fn with_balance_floor(self, balance_floor: f64) -> Self {
        assert!(
            (0.0..=1.0).contains(&balance_floor),
            "0 <= balance_floor <= 1"
        );
        Self {
            balance_floor,
            ..self
        }
    }

    pub fn with_top_ups_per_nation(self, top_ups_per_nation: usize) -> Self {
        Self {
            top_ups_per_nation,
            ..self
        }
    }

    pub fn kind(&self, kind: DepositKind) -> &KindSettings {
        self.kinds.of(kind)
    }

    settings_scalar_accessors! {
        get_seed: seed, GameSeed,
        get_selectivity: selectivity, i32,
        get_min_suitability: min_suitability, f64,
        get_spacing_relaxation: spacing_relaxation, f64,
        get_balance_floor: balance_floor, f64,
        get_top_ups_per_nation: top_ups_per_nation, usize,
    }
}

#[cfg(test)]
mod tests {
    use super::{DepositKind, DepositsGeneratorSettings, KindSettings};

    #[test]
    fn test_deposit_kinds_index_their_per_kind_values() {
        let settings = DepositsGeneratorSettings::default();
        for kind in DepositKind::ALL {
            assert_eq!(DepositKind::ALL[kind.index()], kind);
            assert!(settings.kind(kind).richness_median() >= settings.kind(kind).richness_min);
        }
        let mut order = DepositKind::PLACEMENT_ORDER;
        order.sort();
        assert_eq!(order, DepositKind::ALL, "every kind placed once");
    }

    #[test]
    #[should_panic(expected = "0 < cluster_min <= cluster_max")]
    fn test_settings_reject_an_empty_cluster() {
        let kind = KindSettings {
            cluster_min: 0,
            ..*DepositsGeneratorSettings::default().kind(DepositKind::Oil)
        };
        let _ = DepositsGeneratorSettings::default().with_kind(DepositKind::Oil, kind);
    }
}
