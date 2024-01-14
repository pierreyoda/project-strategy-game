use crate::hex_map::layers::dynamic::HexMapTileSettlement;

use super::{
    ids::SimulationID,
    people::{leaders::Leader, population::PopulationGroup},
};

#[derive(Debug)]
pub struct Settlement {
    /// Must be `SimulationID::SimulationMapEntityID`.
    id: SimulationID,
    name: String,
    leader: Option<Leader>,
    population: Vec<PopulationGroup>,
}

impl HexMapTileSettlement for Settlement {}
