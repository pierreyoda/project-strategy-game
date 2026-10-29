use crate::simulation::resources::ResourceDataStorage;

use super::ids::{LeaderId, SettlementId, SimulationID};

#[derive(Debug)]
pub struct Nation {
    /// Script-side identity of the nation (its definition), as opposed to its runtime `NationId`.
    /// Must be a `SimulationID::Abstract`.
    definition: SimulationID,
    name: String,
    leader: Option<LeaderId>,
    capital: Option<SettlementId>,
    settlements: Vec<SettlementId>,
    resources: ResourceDataStorage,
}

impl Nation {
    pub fn new(definition: SimulationID, name: String) -> Self {
        assert!(matches!(definition, SimulationID::Abstract(_)));
        Self {
            definition,
            name,
            leader: None,
            capital: None,
            settlements: Vec::new(),
            resources: ResourceDataStorage::new(),
        }
    }

    pub fn definition(&self) -> &SimulationID {
        &self.definition
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn leader(&self) -> Option<LeaderId> {
        self.leader
    }

    pub fn capital(&self) -> Option<SettlementId> {
        self.capital
    }

    pub fn settlements(&self) -> &[SettlementId] {
        &self.settlements
    }

    /// Register a settlement as this nation's, making it the capital if it has none yet.
    pub fn add_settlement(&mut self, settlement: SettlementId) {
        if self.capital.is_none() {
            self.capital = Some(settlement);
        }
        self.settlements.push(settlement);
    }

    pub fn resources(&self) -> &ResourceDataStorage {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceDataStorage {
        &mut self.resources
    }
}
