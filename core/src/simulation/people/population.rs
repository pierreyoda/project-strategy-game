//! People simulation at a group aggregate scale.

use crate::simulation::ids::{SimulationID, WithSimulationID};

/// A population group is an abstraction to represent the collective specificities and impact
/// (eg. goods consumption, or voting tendencies).
#[derive(Debug)]
pub struct PopulationGroup {
    /// Must be `SimulationID::EntityID`.
    id: SimulationID,
}

impl WithSimulationID for PopulationGroup {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}
