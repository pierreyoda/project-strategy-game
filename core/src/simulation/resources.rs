use crate::hex_map::layers::natural::HexMapTileDeposit;

use super::ids::{SimulationID, WithSimulationID};

#[derive(Debug)]
pub enum Resource {
    Credits,
    Water,
    Food,
    Electricity,
    Metals,
    Oil,
    Uranium,
    // TODO: CustomResource for scripts
}

#[derive(Debug)]
pub struct ResourceDeposit {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    resource: Resource,
    quantity: u16,
}

impl WithSimulationID for ResourceDeposit {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

impl HexMapTileDeposit for ResourceDeposit {}

impl ResourceDeposit {
    pub fn consume(&mut self, amount: u16) -> bool {
        if amount <= self.quantity {
            self.quantity -= amount;
            true
        } else {
            false
        }
    }
}
