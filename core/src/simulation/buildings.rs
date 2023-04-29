use crate::hex_map::layers::dynamic::HexMapTileBuilding;

use super::ids::{SimulationID, WithSimulationID};

/// A building somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Building {
    id: SimulationID,
}

impl WithSimulationID for Building {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileBuilding for Building {}
