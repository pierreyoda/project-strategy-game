use crate::{
    game::{
        events::{EventBuffer, GameEvent},
        state::{World, WorldMut},
    },
    hex_map::coordinates::CubeCoords,
};

use super::{
    economy::{ConstructionCosts, MaintenanceCosts},
    ids::{HqUnitId, LeaderId, NationId, SimulationID, UnitId, WithSimulationID},
    properties::SimulationPropertyStorage,
    strength::{Damage, MAX_UNIT_COMPANIES, Recovery, Strength, StrengthOutcome},
};

/// Template of a military unit, as defined by a script.
#[derive(Debug)]
pub struct UnitTemplate {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
    r#type: String,
    cost: ConstructionCosts,
    upkeep: MaintenanceCosts,
    /// Movement points a unit of this template gets at the start of every turn.
    movement: u8,
    /// How far a unit of this template sees, in tiles.
    sight: u8,
    /// Companies a unit of this template is recruited with, from 1 to `MAX_UNIT_COMPANIES`.
    companies: u8,
    /// How its suppressed companies recover at the end of every turn.
    recovery: Recovery,
    attributes: SimulationPropertyStorage,
}

impl UnitTemplate {
    pub fn new(
        id: SimulationID,
        r#type: String,
        movement: u8,
        sight: u8,
        companies: u8,
        recovery: Recovery,
    ) -> Self {
        assert!(matches!(id, SimulationID::Abstract(_)));
        assert!((1..=MAX_UNIT_COMPANIES).contains(&companies));
        Self {
            id,
            r#type,
            cost: ConstructionCosts::new(),
            upkeep: MaintenanceCosts::new(),
            movement,
            sight,
            companies,
            recovery,
            attributes: SimulationPropertyStorage::new(),
        }
    }

    pub fn r#type(&self) -> &str {
        &self.r#type
    }

    pub fn cost(&self) -> &ConstructionCosts {
        &self.cost
    }

    pub fn upkeep(&self) -> &MaintenanceCosts {
        &self.upkeep
    }

    pub fn movement(&self) -> u8 {
        self.movement
    }

    pub fn sight(&self) -> u8 {
        self.sight
    }

    pub fn companies(&self) -> u8 {
        self.companies
    }

    pub fn recovery(&self) -> Recovery {
        self.recovery
    }

    pub fn attributes(&self) -> &SimulationPropertyStorage {
        &self.attributes
    }
}

impl WithSimulationID for UnitTemplate {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// A military unit.
#[derive(Debug)]
pub struct Unit {
    /// Template it was recruited from. Must be a `SimulationID::Abstract`.
    template: SimulationID,
    owner: NationId,
    position: CubeCoords,
    strength: Strength,
    /// Movement points left this turn, refilled by `TurnPhase::Finalize`.
    movement_left: u8,
}

impl Unit {
    /// A unit of the template, at full strength.
    pub fn recruit(template: &UnitTemplate, owner: NationId, position: CubeCoords) -> Self {
        Self {
            template: template.id().clone(),
            owner,
            position,
            strength: Strength::full(template.companies()),
            movement_left: 0,
        }
    }

    pub fn template(&self) -> &SimulationID {
        &self.template
    }

    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn set_position(&mut self, position: CubeCoords) {
        self.position = position;
    }

    pub fn strength(&self) -> Strength {
        self.strength
    }

    pub fn strength_mut(&mut self) -> &mut Strength {
        &mut self.strength
    }

    pub fn movement_left(&self) -> u8 {
        self.movement_left
    }

    pub fn spend_movement(&mut self, points: u8) {
        self.movement_left = self.movement_left.saturating_sub(points);
    }

    pub fn refill_movement(&mut self, points: u8) {
        self.movement_left = points;
    }
}

/// Deal a blow to a unit: a unit destroyed by it is removed from the world, and its owner told.
/// `None` for a unit that does not exist.
pub fn damage_unit(
    world: &mut WorldMut<'_>,
    unit: UnitId,
    damage: Damage,
    events: &mut EventBuffer,
) -> Option<StrengthOutcome> {
    let outcome = world.unit_mut(unit)?.strength_mut().take(damage);
    if outcome == StrengthOutcome::Destroyed {
        let destroyed = world.remove_unit(unit)?;
        events.push(GameEvent::UnitDestroyed {
            unit,
            nation: destroyed.owner(),
            tile: world
                .world()
                .tile_index(destroyed.position())
                .unwrap_or_default(),
        });
    }
    Some(outcome)
}

/// Bring back every unit's suppressed companies, as its template says: at the end of a turn.
pub fn recover_suppression(world: &mut WorldMut<'_>) {
    let recoveries = suppressed_units(world.world());
    for (id, recovery) in recoveries {
        if let Some(unit) = world.unit_mut(id) {
            unit.strength_mut().recover(recovery);
        }
    }
}

/// The units with suppressed companies, and how each recovers. A unit whose template is unknown
/// recovers them all.
fn suppressed_units(world: &World) -> Vec<(UnitId, Recovery)> {
    world
        .units()
        .iter()
        .filter(|(_, unit)| unit.strength().suppressed() > 0)
        .map(|(id, unit)| {
            let recovery = world
                .unit_template(unit.template())
                .map_or(Recovery::All, UnitTemplate::recovery);
            (id, recovery)
        })
        .collect()
}

/// A HeadQuarters military unit.
///
/// The chain of command is held by ID: a HQ owning its superior and its subordinates
/// would make the hierarchy a tree of boxes that cannot be rearranged.
#[derive(Debug)]
pub struct HqUnit {
    owner: NationId,
    position: CubeCoords,
    leader: Option<LeaderId>,
    superior: Option<HqUnitId>,
    subordinates: Vec<HqUnitId>,
    attributes: SimulationPropertyStorage,
    attached_units: Vec<UnitId>,
}

impl HqUnit {
    pub fn owner(&self) -> NationId {
        self.owner
    }

    pub fn position(&self) -> CubeCoords {
        self.position
    }

    pub fn leader(&self) -> Option<LeaderId> {
        self.leader
    }

    pub fn superior(&self) -> Option<HqUnitId> {
        self.superior
    }

    pub fn subordinates(&self) -> &[HqUnitId] {
        &self.subordinates
    }

    pub fn attributes(&self) -> &SimulationPropertyStorage {
        &self.attributes
    }

    pub fn attached_units(&self) -> &[UnitId] {
        &self.attached_units
    }
}
