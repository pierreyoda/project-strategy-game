use crate::hex_map::layers::dynamic::HexMapTileBuilding;

use super::{
    economy::{ConstructionCost, MaintenanceCost},
    ids::{SimulationID, WithSimulationID},
    resources::Resource,
};

/// Resource output of a building, per turn.
#[derive(Debug)]
pub struct BuildingProduction {
    resource: Resource,
    amount: u16,
}

/// Template of a building.
#[derive(Debug)]
pub struct BuildingTemplate {
    /// Must be `SimulationID::SimulationAbstractID`.
    id: SimulationID,
    r#type: String,
    cost: Vec<ConstructionCost>,
    maintenance_costs: Vec<MaintenanceCost>,
    production: Vec<BuildingProduction>,
}

/// A building somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Building<'a> {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    template: &'a BuildingTemplate,
    health_points: u16,
}

impl<'a> WithSimulationID for Building<'a> {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl<'a> HexMapTileBuilding for Building<'a> {}
