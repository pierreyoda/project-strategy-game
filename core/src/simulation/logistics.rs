use crate::hex_map::coordinates::CubeCoords;

use super::resources::ResourceDataStorage;

#[derive(Debug)]
pub struct SupplyNode {
    position: CubeCoords,
    level: u8,
    resources: ResourceDataStorage,
}

impl SupplyNode {
    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn resources(&self) -> &ResourceDataStorage {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceDataStorage {
        &mut self.resources
    }
}
