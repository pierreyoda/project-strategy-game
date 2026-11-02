use std::collections::BTreeMap;

use crate::hex_map::coordinates::CubeCoords;

use super::{
    economy::{ConstructionCosts, MaintenanceCosts},
    ids::{NationId, SettlementId, SimulationID},
    resources::{Resource, ResourceQuantity},
};

/// Resources output of a building, per turn.
///
/// Ordered, like the costs in `economy`: it is iterated during turn resolution.
pub type BuildingProduction = BTreeMap<Resource, ResourceQuantity>;

/// The most levels a template can have: levels are numbered in a `u8`, and 0 is taken by a building
/// still being built.
pub const MAX_BUILDING_LEVELS: usize = u8::MAX as usize;

/// One level of a building template: everything that changes with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingLevel {
    /// What reaching this level costs: the construction for level 1, an upgrade above.
    pub cost: ConstructionCosts,
    /// Turns the works to reach it take, at least 1.
    pub turns: u16,
    /// Per turn, while it works at this level.
    pub maintenance: MaintenanceCosts,
    /// Per turn, while it works at this level.
    pub production: BuildingProduction,
    /// What the client draws the level with: a key into its art, opaque to the simulation.
    pub sprite: String,
}

/// Template of a building, as defined by a script: a list of levels.
#[derive(Debug)]
pub struct BuildingTemplate {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
    r#type: String,
    /// The resource it extracts, out of a deposit of it on its tile.
    deposit: Option<Resource>,
    /// Level n is `levels[n - 1]`.
    levels: Vec<BuildingLevel>,
}

impl BuildingTemplate {
    pub fn new(
        id: SimulationID,
        r#type: String,
        deposit: Option<Resource>,
        levels: Vec<BuildingLevel>,
    ) -> Self {
        assert!(matches!(id, SimulationID::Abstract(_)));
        assert!(
            (1..=MAX_BUILDING_LEVELS).contains(&levels.len()),
            "a building template has from 1 to {MAX_BUILDING_LEVELS} levels"
        );
        assert!(
            levels.iter().all(|level| level.turns >= 1),
            "reaching a level takes at least a turn"
        );
        Self {
            id,
            r#type,
            deposit,
            levels,
        }
    }

    pub fn r#type(&self) -> &str {
        &self.r#type
    }

    pub fn deposit(&self) -> Option<Resource> {
        self.deposit
    }

    pub fn levels(&self) -> &[BuildingLevel] {
        &self.levels
    }

    /// Level `level`, counted from 1: `None` for 0 - a building still being built - and past the
    /// last one.
    pub fn level(&self, level: u8) -> Option<&BuildingLevel> {
        self.levels.get(usize::from(level).checked_sub(1)?)
    }

    /// Level 1, which every template has: what building one costs, and takes.
    pub fn first_level(&self) -> &BuildingLevel {
        &self.levels[0]
    }

    pub fn max_level(&self) -> u8 {
        // at most `MAX_BUILDING_LEVELS`, asserted on construction
        self.levels.len() as u8
    }
}

impl super::ids::WithSimulationID for BuildingTemplate {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A construction or an upgrade under way, towards the building's next level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Works {
    turns_left: u16,
    turns_total: u16,
}

impl Works {
    pub fn new(turns: u16) -> Self {
        assert!(turns >= 1, "works take at least a turn");
        Self {
            turns_left: turns,
            turns_total: turns,
        }
    }

    /// Turns until it is done, from 1: the turn it reaches 1 is its last.
    pub fn turns_left(&self) -> u16 {
        self.turns_left
    }

    pub fn turns_total(&self) -> u16 {
        self.turns_total
    }
}

/// A building somewhere on the world map. Not supposed to be moved.
#[derive(Debug)]
pub struct Building {
    /// Template it was built from. Must be a `SimulationID::Abstract`.
    template: SimulationID,
    owner: NationId,
    settlement: SettlementId,
    position: CubeCoords,
    health_points: u16,
    /// The level it works at, from 1 - or 0 while it is first being built. What the map draws it
    /// as, so the simulation's call.
    level: u8,
    /// The construction or upgrade under way, towards `level + 1`: one level at a time.
    works: Option<Works>,
    /// Whether its maintenance went unpaid the last time the economy ran.
    halted: bool,
}

impl Building {
    /// A building finished on the spot, at level 1.
    ///
    /// TODO: what the construct order still builds, until it orders buildings that take their
    /// level-1 turns instead (step 2 of `docs/architecture/buildings.md`).
    pub fn construct(
        template: SimulationID,
        owner: NationId,
        settlement: SettlementId,
        position: CubeCoords,
    ) -> Self {
        Self {
            level: 1,
            works: None,
            ..Self::order(template, owner, settlement, position, 1)
        }
    }

    /// A building just ordered: level 0, its construction under way for `turns`.
    pub fn order(
        template: SimulationID,
        owner: NationId,
        settlement: SettlementId,
        position: CubeCoords,
        turns: u16,
    ) -> Self {
        Self {
            template,
            owner,
            settlement,
            position,
            health_points: u16::MAX,
            level: 0,
            works: Some(Works::new(turns)),
            halted: false,
        }
    }

    pub fn template(&self) -> &SimulationID {
        &self.template
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn settlement(&self) -> SettlementId {
        self.settlement
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn health_points(&self) -> u16 {
        self.health_points
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    /// Whether it is built at all: a building under its first construction makes and pays
    /// nothing. One being upgraded still works, at its current level.
    pub fn is_operating(&self) -> bool {
        self.level >= 1
    }

    pub fn works(&self) -> Option<&Works> {
        self.works.as_ref()
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn set_halted(&mut self, halted: bool) {
        self.halted = halted;
    }

    /// Start the works towards its next level. Whether it may - its owner's order, a level left
    /// above it, no works already under way - is the command's to check.
    pub fn start_upgrade(&mut self, turns: u16) {
        debug_assert!(self.works.is_none(), "one level at a time");
        self.works = Some(Works::new(turns));
    }

    /// One turn of works. Returns whether they completed, which is when the level goes up.
    pub fn advance_works(&mut self) -> bool {
        let Some(works) = self.works.as_mut() else {
            return false;
        };
        works.turns_left -= 1;
        if works.turns_left > 0 {
            return false;
        }
        self.works = None;
        self.level += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        hex_map::coordinates::OffsetCoords,
        simulation::{
            ids::{EntityIndex, NationId, SettlementId, SimulationID},
            resources::Resource,
        },
    };

    use super::{Building, BuildingLevel, BuildingTemplate, Works};

    fn level(turns: u16, metals: u16) -> BuildingLevel {
        BuildingLevel {
            cost: Default::default(),
            turns,
            maintenance: Default::default(),
            production: [(Resource::Metals, metals)].into(),
            sprite: "mine".to_string(),
        }
    }

    fn mine(levels: Vec<BuildingLevel>) -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id("mine"),
            "mine".to_string(),
            Some(Resource::Metals),
            levels,
        )
    }

    fn ordered(turns: u16) -> Building {
        Building::order(
            SimulationID::new_abstract_id("mine"),
            NationId::from_index(0),
            SettlementId::from_index(0),
            OffsetCoords::new(1, 1).into(),
            turns,
        )
    }

    #[test]
    fn test_a_template_counts_its_levels_from_one() {
        let template = mine(vec![level(3, 5), level(4, 8)]);
        assert_eq!(template.max_level(), 2);
        assert_eq!(
            template.level(0),
            None,
            "a building being built has no level yet"
        );
        assert_eq!(template.level(1), Some(template.first_level()));
        assert_eq!(template.level(2).map(|level| level.turns), Some(4));
        assert_eq!(template.level(3), None);
        assert_eq!(template.deposit(), Some(Resource::Metals));
    }

    #[test]
    #[should_panic(expected = "from 1 to")]
    fn test_a_template_has_at_least_one_level() {
        mine(Vec::new());
    }

    #[test]
    #[should_panic(expected = "at least a turn")]
    fn test_a_level_takes_at_least_a_turn() {
        mine(vec![level(0, 5)]);
    }

    #[test]
    fn test_an_ordered_building_works_once_its_construction_is_done() {
        let mut building = ordered(2);
        assert_eq!(building.level(), 0);
        assert!(!building.is_operating());
        assert_eq!(building.works().map(Works::turns_left), Some(2));

        assert!(!building.advance_works(), "one turn of two");
        assert_eq!(building.works().map(Works::turns_left), Some(1));
        assert!(building.advance_works(), "and the last");
        assert_eq!(building.level(), 1);
        assert!(building.is_operating());
        assert_eq!(building.works(), None);
        assert!(!building.advance_works(), "nothing left to advance");
    }

    #[test]
    fn test_an_upgrading_building_keeps_working_at_its_level() {
        let mut building = ordered(1);
        building.advance_works();

        building.start_upgrade(3);
        assert_eq!(building.level(), 1);
        assert!(
            building.is_operating(),
            "it works at level 1 while it is raised to 2"
        );
        assert_eq!(
            building
                .works()
                .map(|works| (works.turns_left(), works.turns_total())),
            Some((3, 3))
        );
        for _ in 0..2 {
            assert!(!building.advance_works());
        }
        assert!(building.advance_works());
        assert_eq!(building.level(), 2);
    }

    #[test]
    fn test_a_building_constructed_on_the_spot_is_level_one() {
        let building = Building::construct(
            SimulationID::new_abstract_id("mine"),
            NationId::from_index(0),
            SettlementId::from_index(0),
            OffsetCoords::new(1, 1).into(),
        );
        assert_eq!(building.level(), 1);
        assert_eq!(building.works(), None);
        assert!(!building.halted());
    }
}
