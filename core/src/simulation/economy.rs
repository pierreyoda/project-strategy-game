use std::collections::BTreeMap;

use super::resources::{Resource, ResourceQuantity};

// `BTreeMap` rather than `HashMap`: costs are iterated during turn resolution, and a resolution
// whose outcome depends on iteration order is not reproducible from a seed.

/// Construction cost.
pub type ConstructionCosts = BTreeMap<Resource, ResourceQuantity>;

/// Maintenance cost, per turn.
pub type MaintenanceCosts = BTreeMap<Resource, ResourceQuantity>;
