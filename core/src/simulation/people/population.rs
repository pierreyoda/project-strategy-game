//! People simulation at a group aggregate scale.

use crate::simulation::{economy::MaintenanceCosts, ids::SettlementId};

/// A population group is an abstraction to represent the collective specificities and impact
/// (eg. goods consumption, or voting tendencies).
#[derive(Debug)]
pub struct PopulationGroup {
    settlement: SettlementId,
    size: u32,
    upkeep: MaintenanceCosts,
}

impl PopulationGroup {
    pub fn settlement(&self) -> SettlementId {
        self.settlement
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn upkeep(&self) -> &MaintenanceCosts {
        &self.upkeep
    }
}
