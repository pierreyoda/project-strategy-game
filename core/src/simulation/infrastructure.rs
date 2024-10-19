use crate::hex_map::layers::dynamic::HexMapTileInfrastructure;

use super::{
    economy::MaintenanceCosts,
    ids::{SimulationID, WithSimulationID},
};

/// Infrastructure somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Infrastructure {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    r#type: String,
    maintenance_costs: MaintenanceCosts,
}

impl WithSimulationID for Infrastructure {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileInfrastructure for Infrastructure {}
