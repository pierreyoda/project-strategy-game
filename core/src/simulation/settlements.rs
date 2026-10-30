use crate::{
    hex_map::coordinates::CubeCoords,
    simulation::ids::{LeaderId, NationId, PopulationGroupId},
};

/// How far a settlement has grown, which is what the client draws it as.
///
/// A size, with one exception: `Capital` is a role. A nation's first settlement is its capital
/// however small it is (see `Nation::add_settlement`), because what the map has to show there is
/// "this is where they rule from", not "this is where most of them live".
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum SettlementStage {
    Outpost = 0,
    Colony = 1,
    Town = 2,
    City = 3,
    Capital = 4,
}

impl SettlementStage {
    /// The smallest population reaching each stage, largest first. `Capital` is not in here: it is
    /// not a size, and `Outpost` is what anything smaller than the last entry comes to.
    const THRESHOLDS: [(u32, Self); 3] = [
        (20_000, Self::City),
        (5_000, Self::Town),
        (1_000, Self::Colony),
    ];

    /// The stage a settlement of this many people has grown to, its nation's capital excepted.
    pub fn for_population(people: u32) -> Self {
        Self::THRESHOLDS
            .iter()
            .find(|&&(threshold, _)| people >= threshold)
            .map_or(Self::Outpost, |&(_, stage)| stage)
    }
}

#[derive(Debug)]
pub struct Settlement {
    name: String,
    owner: NationId,
    position: CubeCoords,
    leader: Option<LeaderId>,
    population: Vec<PopulationGroupId>,
}

impl Settlement {
    pub fn found(name: String, owner: NationId, position: CubeCoords) -> Self {
        Self {
            name,
            owner,
            position,
            leader: None,
            population: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn leader(&self) -> Option<LeaderId> {
        self.leader
    }

    pub fn set_leader(&mut self, leader: Option<LeaderId>) {
        self.leader = leader;
    }

    pub fn population(&self) -> &[PopulationGroupId] {
        &self.population
    }

    pub fn add_population_group(&mut self, group: PopulationGroupId) {
        self.population.push(group);
    }
}

#[cfg(test)]
mod tests {
    use super::SettlementStage;

    #[test]
    fn test_a_settlement_grows_through_every_stage_but_the_capital() {
        assert_eq!(SettlementStage::for_population(0), SettlementStage::Outpost);
        assert_eq!(
            SettlementStage::for_population(999),
            SettlementStage::Outpost
        );
        assert_eq!(
            SettlementStage::for_population(1_000),
            SettlementStage::Colony
        );
        assert_eq!(
            SettlementStage::for_population(4_999),
            SettlementStage::Colony
        );
        assert_eq!(
            SettlementStage::for_population(5_000),
            SettlementStage::Town
        );
        assert_eq!(
            SettlementStage::for_population(19_999),
            SettlementStage::Town
        );
        assert_eq!(
            SettlementStage::for_population(20_000),
            SettlementStage::City
        );
        assert_eq!(
            SettlementStage::for_population(u32::MAX),
            SettlementStage::City
        );
    }
}
