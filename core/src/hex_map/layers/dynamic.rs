//! The dynamic layer of an `HexMap` will involve geographically static, artificially built objects like buildings.
//!
//! A tile only ever *references* what stands on it: the entities themselves live in the `World`'s
//! stores. Owning them here would give every entity two owners, and would make a tile impossible to
//! hand out for the cheap, copy-free projection the client reads.

use crate::simulation::ids::{BuildingId, InfrastructureId, NationId, SettlementId, SupplyNodeId};

#[derive(Debug, Default)]
pub struct HexMapArtificialTileData {
    owner: Option<NationId>,
    supply_node: Option<SupplyNodeId>,
    infrastructure: Vec<InfrastructureId>,
    settlement: Option<SettlementId>,
    buildings: Vec<BuildingId>,
}

impl HexMapArtificialTileData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn owner(&self) -> Option<NationId> {
        self.owner
    }

    pub fn set_owner(&mut self, owner: Option<NationId>) {
        self.owner = owner;
    }

    pub fn supply_node(&self) -> Option<SupplyNodeId> {
        self.supply_node
    }

    pub fn set_supply_node(&mut self, supply_node: Option<SupplyNodeId>) {
        self.supply_node = supply_node;
    }

    pub fn infrastructure(&self) -> &[InfrastructureId] {
        &self.infrastructure
    }

    pub fn add_infrastructure(&mut self, infrastructure: InfrastructureId) {
        self.infrastructure.push(infrastructure);
    }

    pub fn settlement(&self) -> Option<SettlementId> {
        self.settlement
    }

    pub fn set_settlement(&mut self, settlement: Option<SettlementId>) {
        self.settlement = settlement;
    }

    pub fn buildings(&self) -> &[BuildingId] {
        &self.buildings
    }

    pub fn add_building(&mut self, building: BuildingId) {
        self.buildings.push(building);
    }
}
