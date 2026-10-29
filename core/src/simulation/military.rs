use crate::hex_map::coordinates::CubeCoords;

use super::{
    economy::{ConstructionCosts, MaintenanceCosts},
    ids::{HqUnitId, LeaderId, NationId, SimulationID, UnitId, WithSimulationID},
    properties::SimulationPropertyStorage,
};

/// Template of a military unit, as defined by a script.
#[derive(Debug)]
pub struct UnitTemplate {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
    r#type: String,
    cost: ConstructionCosts,
    upkeep: MaintenanceCosts,
    /// Movement points a unit of this template gets at the start of every turn.
    movement: u8,
    /// How far a unit of this template sees, in tiles.
    sight: u8,
    attributes: SimulationPropertyStorage,
}

impl UnitTemplate {
    pub fn new(id: SimulationID, r#type: String, movement: u8, sight: u8) -> Self {
        assert!(matches!(id, SimulationID::Abstract(_)));
        Self {
            id,
            r#type,
            cost: ConstructionCosts::new(),
            upkeep: MaintenanceCosts::new(),
            movement,
            sight,
            attributes: SimulationPropertyStorage::new(),
        }
    }

    pub fn r#type(&self) -> &str {
        &self.r#type
    }

    pub fn cost(&self) -> &ConstructionCosts {
        &self.cost
    }

    pub fn upkeep(&self) -> &MaintenanceCosts {
        &self.upkeep
    }

    pub fn movement(&self) -> u8 {
        self.movement
    }

    pub fn sight(&self) -> u8 {
        self.sight
    }

    pub fn attributes(&self) -> &SimulationPropertyStorage {
        &self.attributes
    }
}

impl WithSimulationID for UnitTemplate {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A military unit.
#[derive(Debug)]
pub struct Unit {
    /// Template it was recruited from. Must be a `SimulationID::Abstract`.
    template: SimulationID,
    owner: NationId,
    position: CubeCoords,
    health_points: u16,
    /// Movement points left this turn, refilled by `TurnPhase::Finalize`.
    movement_left: u8,
}

impl Unit {
    pub fn recruit(template: SimulationID, owner: NationId, position: CubeCoords) -> Self {
        Self {
            template,
            owner,
            position,
            health_points: u16::MAX,
            movement_left: 0,
        }
    }

    pub fn template(&self) -> &SimulationID {
        &self.template
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn set_position(&mut self, position: CubeCoords) {
        self.position = position;
    }

    pub fn health_points(&self) -> u16 {
        self.health_points
    }

    pub fn movement_left(&self) -> u8 {
        self.movement_left
    }

    pub fn spend_movement(&mut self, points: u8) {
        self.movement_left = self.movement_left.saturating_sub(points);
    }

    pub fn refill_movement(&mut self, points: u8) {
        self.movement_left = points;
    }
}

/// A HeadQuarters military unit.
///
/// The chain of command is held by ID: a HQ owning its superior and its subordinates
/// would make the hierarchy a tree of boxes that cannot be rearranged.
#[derive(Debug)]
pub struct HqUnit {
    owner: NationId,
    position: CubeCoords,
    leader: Option<LeaderId>,
    superior: Option<HqUnitId>,
    subordinates: Vec<HqUnitId>,
    attributes: SimulationPropertyStorage,
    attached_units: Vec<UnitId>,
}

impl HqUnit {
    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn leader(&self) -> Option<LeaderId> {
        self.leader
    }

    pub fn superior(&self) -> Option<HqUnitId> {
        self.superior
    }

    pub fn subordinates(&self) -> &[HqUnitId] {
        &self.subordinates
    }

    pub fn attributes(&self) -> &SimulationPropertyStorage {
        &self.attributes
    }

    pub fn attached_units(&self) -> &[UnitId] {
        &self.attached_units
    }
}
