use std::collections::BTreeMap;

use crate::hex_map::coordinates::CubeCoords;

use super::ids::SimulationID;

pub type ResourceQuantity = u16;

/// `Ord` as well as `Hash`: resources are iterated during turn resolution, and a resolution whose
/// outcome depends on iteration order is not reproducible from a seed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
pub struct ResourceData {
    quantity: ResourceQuantity,
    /// Per-turn income.
    income: Option<ResourceQuantity>,
    /// Per-turn depletion.
    depletion: Option<ResourceQuantity>,
}

impl ResourceData {
    pub fn quantity(&self) -> ResourceQuantity {
        self.quantity
    }

    pub fn income(&self) -> Option<ResourceQuantity> {
        self.income
    }

    pub fn depletion(&self) -> Option<ResourceQuantity> {
        self.depletion
    }
}

pub type ResourceDataStorage = BTreeMap<Resource, ResourceData>;

pub trait ResourceDataStore {
    fn set_income(&mut self, resource: Resource, income: ResourceQuantity) -> bool;
    fn set_depletion(&mut self, resource: Resource, depletion: ResourceQuantity) -> bool;
    fn replenish(&mut self, resource: Resource, amount: ResourceQuantity);
    fn consume(&mut self, resource: Resource, amount: ResourceQuantity) -> bool;
    /// Quantity currently held, zero if the resource was never stored.
    fn quantity_of(&self, resource: Resource) -> ResourceQuantity;
    /// Called every turn. If a resource lacks the quantity compared to its per-turn depletion, return its key.
    fn update(&mut self) -> Option<Resource>;
}

impl ResourceDataStore for ResourceDataStorage {
    fn set_income(&mut self, resource: Resource, income: ResourceQuantity) -> bool {
        if let Some(resource_datum) = self.get_mut(&resource) {
            resource_datum.income = Some(income);
            true
        } else {
            false
        }
    }

    fn set_depletion(&mut self, resource: Resource, depletion: ResourceQuantity) -> bool {
        if let Some(resource_datum) = self.get_mut(&resource) {
            resource_datum.depletion = Some(depletion);
            true
        } else {
            false
        }
    }

    fn replenish(&mut self, resource: Resource, amount: ResourceQuantity) {
        if let Some(resource_datum) = self.get_mut(&resource) {
            resource_datum.quantity += amount;
        } else {
            self.insert(
                resource,
                ResourceData {
                    quantity: amount,
                    income: None,
                    depletion: None,
                },
            );
        }
    }

    fn consume(&mut self, resource: Resource, amount: ResourceQuantity) -> bool {
        if let Some(resource_datum) = self.get_mut(&resource) {
            if resource_datum.quantity > amount {
                resource_datum.quantity -= amount;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn quantity_of(&self, resource: Resource) -> ResourceQuantity {
        self.get(&resource).map_or(0, ResourceData::quantity)
    }

    fn update(&mut self) -> Option<Resource> {
        for (resource, resource_datum) in self.iter_mut() {
            resource_datum.quantity += resource_datum.income.unwrap_or(0);
            if let Some(depletion) = resource_datum.depletion {
                if depletion > resource_datum.quantity {
                    return Some(*resource);
                }
                resource_datum.quantity -= depletion;
            }
        }
        None
    }
}

/// A natural resource deposit on a tile, referenced by the tile through its `DepositId`.
#[derive(Debug)]
pub struct ResourceDeposit {
    /// What the deposit holds. Must be a `SimulationID::Abstract`.
    definition: SimulationID,
    position: CubeCoords,
    resources: ResourceDataStorage,
}

impl ResourceDeposit {
    pub fn definition(&self) -> &SimulationID {
        &self.definition
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn resources(&self) -> &ResourceDataStorage {
        &self.resources
    }

    pub fn resources_mut(&mut self) -> &mut ResourceDataStorage {
        &mut self.resources
    }
}

#[cfg(test)]
mod tests {
    use crate::simulation::resources::Resource;

    use super::{ResourceDataStorage, ResourceDataStore};

    fn build_mock_data_storage() -> ResourceDataStorage {
        let mut resources = ResourceDataStorage::new();
        resources.replenish(Resource::Water, 100);
        resources.replenish(Resource::Food, 250);
        resources.replenish(Resource::Credits, 50);
        resources
    }

    #[test]
    fn test_resource_data_storage_operations() {
        let mut resources = build_mock_data_storage();

        assert_eq!(resources.get(&Resource::Water).unwrap().quantity, 100);
        assert!(resources.consume(Resource::Water, 50));
        assert_eq!(resources.get(&Resource::Water).unwrap().quantity, 50);
        assert!(!resources.consume(Resource::Food, 300));
        assert_eq!(resources.get(&Resource::Food).unwrap().quantity, 250);
        assert_eq!(resources.quantity_of(Resource::Water), 50);
        assert_eq!(resources.quantity_of(Resource::Uranium), 0);
    }

    #[test]
    fn test_resource_data_storage_update() {
        let mut resources = build_mock_data_storage();
        assert!(!resources.set_income(Resource::Metals, 10));
        assert!(!resources.set_depletion(Resource::Metals, 10));
        assert!(resources.set_income(Resource::Water, 10));
        assert!(resources.set_depletion(Resource::Water, 5));

        assert_eq!(resources.update(), None);
        assert_eq!(resources.get(&Resource::Water).unwrap().quantity, 105);

        assert!(resources.set_depletion(Resource::Food, 251));
        assert_eq!(resources.update(), Some(Resource::Food));
        assert_eq!(resources.get(&Resource::Food).unwrap().quantity, 250);

        assert!(resources.set_depletion(Resource::Food, 0));
        assert!(resources.set_income(Resource::Credits, 1));
        assert!(resources.set_depletion(Resource::Credits, 50));
        assert_eq!(resources.update(), None);
        assert_eq!(resources.get(&Resource::Credits).unwrap().quantity, 1);
    }

    /// `update` stops on the first resource that comes short: which one that is must not depend on
    /// the iteration order of the storage.
    #[test]
    fn test_resource_data_storage_update_is_deterministic() {
        let shortages: Vec<Option<Resource>> = (0..8)
            .map(|_| {
                let mut resources = build_mock_data_storage();
                assert!(resources.set_depletion(Resource::Water, 200));
                assert!(resources.set_depletion(Resource::Food, 300));
                resources.update()
            })
            .collect();
        // `Credits` < `Water` < `Food` in declaration order, and `Credits` is not short
        assert!(shortages.iter().all(|&s| s == Some(Resource::Water)));
    }
}
