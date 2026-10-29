use std::collections::BTreeMap;

use crate::hex_map::coordinates::CubeCoords;

use super::{
    economy::{ConstructionCosts, MaintenanceCosts},
    ids::{NationId, SettlementId, SimulationID},
    resources::{Resource, ResourceQuantity},
};

/// Resources output of a building, per turn.
///
/// Ordered, like the costs in `economy`: it is iterated during turn resolution.
pub type BuildingProduction = BTreeMap<Resource, ResourceQuantity>;

/// Template of a building, as defined by a script.
#[derive(Debug)]
pub struct BuildingTemplate {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
    r#type: String,
    cost: ConstructionCosts,
    maintenance_costs: MaintenanceCosts,
    production: BuildingProduction,
}

impl BuildingTemplate {
    pub fn new(
        id: SimulationID,
        r#type: String,
        cost: ConstructionCosts,
        maintenance_costs: MaintenanceCosts,
        production: BuildingProduction,
    ) -> Self {
        assert!(matches!(id, SimulationID::Abstract(_)));
        Self {
            id,
            r#type,
            cost,
            maintenance_costs,
            production,
        }
    }

    pub fn r#type(&self) -> &str {
        &self.r#type
    }

    pub fn cost(&self) -> &ConstructionCosts {
        &self.cost
    }

    pub fn maintenance_costs(&self) -> &MaintenanceCosts {
        &self.maintenance_costs
    }

    pub fn production(&self) -> &BuildingProduction {
        &self.production
    }
}

impl super::ids::WithSimulationID for BuildingTemplate {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A building somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Building {
    /// Template it was built from. Must be a `SimulationID::Abstract`.
    template: SimulationID,
    owner: NationId,
    settlement: SettlementId,
    position: CubeCoords,
    health_points: u16,
}

impl Building {
    pub fn construct(
        template: SimulationID,
        owner: NationId,
        settlement: SettlementId,
        position: CubeCoords,
    ) -> Self {
        Self {
            template,
            owner,
            settlement,
            position,
            health_points: u16::MAX,
        }
    }

    pub fn template(&self) -> &SimulationID {
        &self.template
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn settlement(&self) -> SettlementId {
        self.settlement
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn health_points(&self) -> u16 {
        self.health_points
    }
}
