use crate::hex_map::coordinates::HexMapCoordinates;

use super::{
    ids::{SimulationID, WithSimulationID},
    people::leaders::Leader,
};

/// A military unit.
#[derive(Debug)]
pub struct Unit {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    position: HexMapCoordinates,
}

impl WithSimulationID for Unit {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A HeadQuarters military unit.
#[derive(Debug)]
pub struct HqUnit {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    leader: Option<Leader>,
    position: HexMapCoordinates,
}

impl WithSimulationID for HqUnit {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}
