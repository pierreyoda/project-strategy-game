use crate::{
    hex_map::coordinates::CubeCoords,
    simulation::ids::{LeaderId, NationId, PopulationGroupId},
};

#[derive(Debug)]
pub struct Settlement {
    name: String,
    owner: NationId,
    position: CubeCoords,
    leader: Option<LeaderId>,
    population: Vec<PopulationGroupId>,
}

impl Settlement {
    pub fn found(name: String, owner: NationId, position: CubeCoords) -> Self {
        Self {
            name,
            owner,
            position,
            leader: None,
            population: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn leader(&self) -> Option<LeaderId> {
        self.leader
    }

    pub fn set_leader(&mut self, leader: Option<LeaderId>) {
        self.leader = leader;
    }

    pub fn population(&self) -> &[PopulationGroupId] {
        &self.population
    }

    pub fn add_population_group(&mut self, group: PopulationGroupId) {
        self.population.push(group);
    }
}
