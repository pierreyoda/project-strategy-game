use super::{
    ids::{SimulationID, WithSimulationID},
    people::leaders::Leader,
};

#[derive(Debug)]
pub struct Nation {
    /// Must be `SimulationID::SimulationAbstractID`.
    id: SimulationID,
    name: String,
    leader: Leader,
}

impl WithSimulationID for Nation {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}
