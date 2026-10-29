//! The static layer of an `HexMap` involve naturally-evolved properties that do not fundamentally change during gameplay but is set at world generation.
//!
//! Some properties may still be affected during gameplay though, for instance the initial accessible quantity of a resource diminishing from mining activity.
//!
//! As in the dynamic layer, deposits are referenced by ID: they live in the `World`'s stores.

use crate::simulation::ids::DepositId;

#[derive(Debug, Default)]
pub struct HexMapNaturalTileData {
    deposits: Vec<DepositId>,
}

impl HexMapNaturalTileData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deposits(&self) -> &[DepositId] {
        &self.deposits
    }

    pub fn add_deposit(&mut self, deposit: DepositId) {
        self.deposits.push(deposit);
    }
}
