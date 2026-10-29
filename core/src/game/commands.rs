//! What is asked of the world, and why it can be refused.
//!
//! A command is the *only* way the world changes outside of the systems' own per-turn passes. It is
//! also the only thing an AI produces: `NationController::plan` returns commands, which go through
//! the validation and application below exactly like a player's do.
//!
//! That is deliberate. One path means one set of rules, an AI that cannot cheat by construction, a
//! turn that replays from `seed + command log`, and a headless test that drives the game the way a
//! player does.

use crate::{
    game::{
        events::{EventBuffer, GameEvent},
        state::WorldMut,
    },
    hex_map::coordinates::CubeCoords,
    simulation::{
        buildings::Building,
        ids::{EntityRef, NationId, SettlementId, SimulationID, UnitId},
        resources::{Resource, ResourceDataStore, ResourceQuantity},
        settlements::Settlement,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameCommand {
    /// Walk a unit along a path of adjacent tiles, one movement point per step.
    MoveUnit { unit: UnitId, path: Vec<CubeCoords> },
    /// Found a settlement where the founding unit stands. The unit is spent.
    FoundSettlement { founder: UnitId, name: String },
    /// Build in a settlement, paid for out of the nation's stockpile.
    ConstructBuilding {
        settlement: SettlementId,
        template: SimulationID,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandRejection {
    /// Sent by a nation that is not the one being played.
    NotYourTurn {
        active: NationId,
    },
    UnknownEntity(EntityRef),
    UnknownDefinition(SimulationID),
    /// The entity exists, but belongs to someone else.
    NotYours(EntityRef),
    IllegalMove {
        reason: &'static str,
    },
    TileOccupied,
    InsufficientResources {
        resource: Resource,
        missing: ResourceQuantity,
    },
    /// Rejected by a rule that has not been written yet.
    NotImplemented(&'static str),
}

impl std::fmt::Display for CommandRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotYourTurn { active } => write!(f, "it is {active}'s turn"),
            Self::UnknownEntity(entity) => write!(f, "no such entity: {entity}"),
            Self::UnknownDefinition(id) => write!(f, "no such definition: {id}"),
            Self::NotYours(entity) => write!(f, "{entity} belongs to another nation"),
            Self::IllegalMove { reason } => write!(f, "illegal move: {reason}"),
            Self::TileOccupied => write!(f, "the tile is already occupied"),
            Self::InsufficientResources { resource, missing } => {
                write!(f, "{missing} more {resource:?} needed")
            }
            Self::NotImplemented(what) => write!(f, "not implemented yet: {what}"),
        }
    }
}

/// Radius, in tiles, a new settlement claims around itself.
const SETTLEMENT_CLAIM_RADIUS: u32 = 1;

/// Validate and apply a command on behalf of `issuer`.
///
/// Validation happens entirely before the first mutation: a command either applies whole or changes
/// nothing, so a rejected command can never leave the world half-updated.
pub fn apply(
    command: &GameCommand,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    match command {
        GameCommand::MoveUnit { unit, path } => move_unit(*unit, path, issuer, world, events),
        GameCommand::FoundSettlement { founder, name } => {
            found_settlement(*founder, name, issuer, world, events)
        }
        GameCommand::ConstructBuilding {
            settlement,
            template,
        } => construct_building(*settlement, template, issuer, world, events),
    }
}

fn move_unit(
    unit: UnitId,
    path: &[CubeCoords],
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    if path.is_empty() {
        return Err(CommandRejection::IllegalMove {
            reason: "empty path",
        });
    }

    let (origin, movement_left) = {
        let state = world.world();
        let moved = state
            .units()
            .get(unit)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Unit(unit)))?;
        if moved.owner() != issuer {
            return Err(CommandRejection::NotYours(EntityRef::Unit(unit)));
        }
        (moved.position(), moved.movement_left())
    };

    // TODO: path.len will need to be replaced by path.total_movement_cost to account for terrain
    if path.len() > movement_left as usize {
        return Err(CommandRejection::IllegalMove {
            reason: "not enough movement points left",
        });
    }

    // every step has to be a real tile, adjacent to the one before it
    let state = world.world();
    let shape = state.shape();
    let mut current = origin;
    for step in path {
        let step_index = state
            .tile_index(*step)
            .ok_or(CommandRejection::IllegalMove {
                reason: "the path leaves the map",
            })?;
        let adjacent = shape
            .neighbors(current)
            .any(|neighbor| state.tile_index(neighbor) == Some(step_index));
        if !adjacent {
            return Err(CommandRejection::IllegalMove {
                reason: "the path skips a tile",
            });
        }
        current = *step;
    }

    let from = world.world().tile_index(origin).unwrap_or_default();
    let to = world.world().tile_index(current).unwrap_or_default();
    let steps = path.len() as u8;

    // the tile left behind and the one arrived on both have to be redrawn
    world.mark_tile(origin);
    world.mark_tile(current);
    let moved = world
        .unit_mut(unit)
        .ok_or(CommandRejection::UnknownEntity(EntityRef::Unit(unit)))?;
    moved.set_position(current);
    moved.spend_movement(steps);

    events.push(GameEvent::UnitMoved { unit, from, to });
    Ok(())
}

fn found_settlement(
    founder: UnitId,
    name: &str,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    let position = {
        let state = world.world();
        let unit = state
            .units()
            .get(founder)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Unit(founder)))?;
        if unit.owner() != issuer {
            return Err(CommandRejection::NotYours(EntityRef::Unit(founder)));
        }
        let position = unit.position();
        let tile = state.tile(position).ok_or(CommandRejection::IllegalMove {
            reason: "off the map",
        })?;
        if tile.artificial().settlement().is_some() {
            return Err(CommandRejection::TileOccupied);
        }
        position
    };

    let tile_index = world.world().tile_index(position).unwrap_or_default();
    let settlement = world.insert_settlement(Settlement::found(name.to_string(), issuer, position));
    // the founding unit is spent
    world.remove_unit(founder);

    // claim the ring around the new settlement
    let claimed: Vec<CubeCoords> = world
        .world()
        .tiles_within(position, SETTLEMENT_CLAIM_RADIUS)
        .into_iter()
        .filter_map(|index| world.world().tile_coords(index))
        .collect();
    for coords in claimed {
        if let Some(tile) = world.tile_mut(coords)
            && tile.artificial().owner().is_none()
        {
            tile.artificial_mut().set_owner(Some(issuer));
        }
    }

    events.push(GameEvent::SettlementFounded {
        settlement,
        nation: issuer,
        tile: tile_index,
    });
    events.push(GameEvent::Log {
        nation: Some(issuer),
        message: format!("{name} was founded."),
    });
    Ok(())
}

fn construct_building(
    settlement: SettlementId,
    template: &SimulationID,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    let (position, cost) = {
        let state = world.world();
        let target = state
            .settlements()
            .get(settlement)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Settlement(
                settlement,
            )))?;
        if target.owner() != issuer {
            return Err(CommandRejection::NotYours(EntityRef::Settlement(
                settlement,
            )));
        }
        let definition = state
            .building_template(template)
            .ok_or_else(|| CommandRejection::UnknownDefinition(template.clone()))?;
        let nation = state
            .nations()
            .get(issuer)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Nation(issuer)))?;
        // the whole bill is checked before a single resource is spent
        for (&resource, &amount) in definition.cost() {
            let held = nation.resources().quantity_of(resource);
            if held < amount {
                return Err(CommandRejection::InsufficientResources {
                    resource,
                    missing: amount - held,
                });
            }
        }
        (target.position(), definition.cost().clone())
    };

    if let Some(nation) = world.nation_mut(issuer) {
        for (&resource, &amount) in &cost {
            nation.resources_mut().consume(resource, amount);
        }
    }
    let building = world.insert_building(Building::construct(
        template.clone(),
        issuer,
        settlement,
        position,
    ));

    events.push(GameEvent::BuildingConstructed {
        building,
        settlement,
    });
    Ok(())
}
