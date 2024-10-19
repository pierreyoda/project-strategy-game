use std::collections::HashMap;

use super::resources::{Resource, ResourceQuantity};

/// Construction cost.
pub type ConstructionCosts = HashMap<Resource, ResourceQuantity>;

/// Maintenance cost, per turn.
pub type MaintenanceCosts = HashMap<Resource, ResourceQuantity>;
