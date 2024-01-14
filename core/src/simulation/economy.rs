use super::resources::Resource;

/// Maintenance cost, per turn.
#[derive(Debug)]
pub struct MaintenanceCost {
    resource: Resource,
    amount: u16,
}
