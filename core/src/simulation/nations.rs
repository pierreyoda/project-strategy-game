use super::{
    ids::{SimulationID, WithSimulationID},
    military::HqUnit,
    people::leaders::Leader,
    settlements::Settlement,
};

#[derive(Debug)]
pub struct Nation<'a> {
    /// Must be `SimulationID::SimulationAbstractID`.
    id: SimulationID,
    name: String,
    leader: Leader,
    headquarters: Vec<HqUnit<'a>>,
    capital: &'a Settlement,
    settlements: Vec<&'a Settlement>,
}

impl<'a> WithSimulationID for Nation<'a> {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}
