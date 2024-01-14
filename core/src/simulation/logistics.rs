use crate::hex_map::layers::dynamic::HexMapTileSupplyNode;

use super::{
    ids::{SimulationID, WithSimulationID},
    resources::Resource,
};

#[derive(Debug)]
pub struct ResourceStockpile {
    resource: Resource,
    quantity: u16,
}

#[derive(Debug)]
pub struct SupplyNode {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    level: u8,
    stockpile: Vec<ResourceStockpile>,
}

impl WithSimulationID for SupplyNode {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileSupplyNode for SupplyNode {}
