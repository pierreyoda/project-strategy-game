use crate::hex_map::layers::dynamic::HexMapTileSupplyNode;

use super::{
    ids::{SimulationID, WithSimulationID},
    resources::ResourceDataStorage,
};

#[derive(Debug)]
pub struct SupplyNode {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    level: u8,
    resources: ResourceDataStorage,
}

impl WithSimulationID for SupplyNode {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileSupplyNode for SupplyNode {}
