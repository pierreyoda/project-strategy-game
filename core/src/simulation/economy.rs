use super::resources::Resource;

/// Construction cost, per turn.
#[derive(Debug)]
pub struct ConstructionCost {
    resource: Resource,
    amount: u16,
}

/// Maintenance cost, per turn.
#[derive(Debug)]
pub struct MaintenanceCost {
    resource: Resource,
    amount: u16,
}
