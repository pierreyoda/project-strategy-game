use crate::hex_map::coordinates::HexMapCoordinates;

use super::{
    economy::{ConstructionCost, MaintenanceCost},
    ids::{SimulationID, WithSimulationID},
    people::leaders::Leader,
    properties::SimulationPropertyStorage,
};

/// Template of a military unit.
#[derive(Debug)]
pub struct UnitTemplate {
    /// Must be `SimulationID::SimulationAbstractID`.
    id: SimulationID,
    r#type: String,
    cost: Vec<ConstructionCost>,
    upkeep: Vec<MaintenanceCost>,
    attributes: SimulationPropertyStorage,
}

/// A military unit.
#[derive(Debug)]
pub struct Unit<'a> {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    position: HexMapCoordinates,
    template: &'a UnitTemplate,
    health_points: u16,
}

impl<'a> WithSimulationID for Unit<'a> {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A HeadQuarters military unit.
#[derive(Debug)]
pub struct HqUnit<'a> {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    position: HexMapCoordinates,
    leader: Option<Leader>,
    superior: Option<Box<HqUnit<'a>>>,
    subordinates: Option<Vec<Box<HqUnit<'a>>>>,
    attributes: SimulationPropertyStorage,
    attached_units: Vec<Unit<'a>>,
}

impl<'a> WithSimulationID for HqUnit<'a> {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}
