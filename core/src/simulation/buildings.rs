use std::collections::HashMap;

use crate::hex_map::layers::dynamic::HexMapTileBuilding;

use super::{
    economy::{ConstructionCosts, MaintenanceCosts},
    ids::{SimulationID, WithSimulationID},
    resources::{Resource, ResourceQuantity},
};

/// Resources output of a building, per turn.
pub type BuildingProduction = HashMap<Resource, ResourceQuantity>;

/// Template of a building.
#[derive(Debug)]
pub struct BuildingTemplate {
    /// Must be `SimulationID::SimulationAbstractID`.
    id: SimulationID,
    r#type: String,
    cost: ConstructionCosts,
    maintenance_costs: MaintenanceCosts,
    production: BuildingProduction,
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
