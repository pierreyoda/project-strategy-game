use crate::hex_map::layers::dynamic::HexMapTileBuilding;

use super::{
    economy::MaintenanceCost,
    ids::{SimulationID, WithSimulationID},
    resources::Resource,
};

#[derive(Debug)]
pub struct BuildingProduction {
    resource: Resource,
    amount: u16,
}

/// A building somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Building {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    r#type: String,
    health_points: u16,
    maintenance_costs: Vec<MaintenanceCost>,
    production: Vec<BuildingProduction>,
}

impl WithSimulationID for Building {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileBuilding for Building {}
