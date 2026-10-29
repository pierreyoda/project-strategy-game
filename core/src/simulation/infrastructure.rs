use crate::hex_map::coordinates::CubeCoords;

use super::{economy::MaintenanceCosts, ids::SimulationID};

/// Infrastructure somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Infrastructure {
    /// Template it was built from. Must be a `SimulationID::Abstract`.
    template: SimulationID,
    position: CubeCoords,
    maintenance_costs: MaintenanceCosts,
}

impl Infrastructure {
    pub fn template(&self) -> &SimulationID {
        &self.template
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn maintenance_costs(&self) -> &MaintenanceCosts {
        &self.maintenance_costs
    }
}
