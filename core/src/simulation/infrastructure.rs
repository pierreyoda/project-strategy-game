use crate::hex_map::layers::dynamic::HexMapTileInfrastructure;

use super::{
    economy::MaintenanceCost,
    ids::{SimulationID, WithSimulationID},
};

/// Infrastructure somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Infrastructure {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    r#type: String,
    maintenance_costs: Vec<MaintenanceCost>,
}

impl WithSimulationID for Infrastructure {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileInfrastructure for Infrastructure {}
