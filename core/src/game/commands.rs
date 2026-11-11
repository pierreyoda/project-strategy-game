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
        state::{World, WorldMut},
    },
    hex_map::coordinates::CubeCoords,
    simulation::{
        buildings::Building,
        districts::{DISTRICT_DISTANCE, District},
        economy::{self, ConstructionCosts},
        ids::{BuildingId, EntityRef, NationId, SettlementId, SimulationID, UnitId},
        movement::{self, StepRefusal},
        resources::{Resource, ResourceQuantity},
        settlements::Settlement,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameCommand {
    /// Walk a unit along a path of adjacent tiles, each step costing the ground's entry cost
    /// (see `simulation::movement`). Refused as a whole if any step is.
    MoveUnit { unit: UnitId, path: Vec<CubeCoords> },
    /// Found a settlement where the founding unit stands. The unit is spent.
    FoundSettlement { founder: UnitId, name: String },
    /// Order a building on a tile of the issuer's own land: level 1's cost is paid at once, and its
    /// turns pass before the building works. One that extracts nothing stands on a laid-out
    /// district and belongs to its settlement; an extractor stands on its deposit, and belongs to
    /// the district's settlement if there is one, to the nearest settlement if not.
    ConstructBuilding {
        template: SimulationID,
        tile: CubeCoords,
    },
    /// Order a building's next level: that level's cost at once, then its turns, during which the
    /// building keeps working at the level it has.
    UpgradeBuilding { building: BuildingId },
    /// Lay out a district for one of the issuer's settlements on a free tile of its land next to
    /// it: the scenario's terms are paid at once, and their turns pass before it can be built on.
    LayOutDistrict {
        settlement: SettlementId,
        tile: CubeCoords,
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
    /// Not the issuer's land, water, or a tile already built on.
    IllegalPlacement {
        reason: &'static str,
    },
    /// The template extracts a resource the tile holds no deposit of - or none with anything left.
    MissingDeposit {
        resource: Resource,
    },
    /// A building needs a settlement to belong to, and the issuer has none.
    NoSettlement,
    AtMaxLevel,
    /// Works are already under way on the building: one level at a time.
    WorksUnderWay,
    /// A building that extracts nothing stands on a laid-out district, and the tile has none.
    NoDistrict,
    /// The scenario's rules give the settlement no district: none defined, or none for it now.
    DistrictRefused(String),
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
            Self::IllegalPlacement { reason } => write!(f, "cannot build here: {reason}"),
            Self::MissingDeposit { resource } => {
                write!(f, "it needs a {resource:?} deposit to stand on")
            }
            Self::NoSettlement => write!(f, "a building needs a settlement to belong to"),
            Self::AtMaxLevel => write!(f, "the building is at its last level"),
            Self::WorksUnderWay => write!(f, "works are already under way"),
            Self::NoDistrict => write!(f, "it needs a district laid out on the tile"),
            Self::DistrictRefused(reason) => write!(f, "no district: {reason}"),
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
        GameCommand::ConstructBuilding { template, tile } => {
            construct_building(template, *tile, issuer, world, events)
        }
        GameCommand::UpgradeBuilding { building } => {
            upgrade_building(*building, issuer, world, events)
        }
        GameCommand::LayOutDistrict { settlement, tile } => {
            lay_out_district(*settlement, *tile, issuer, world, events)
        }
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

    // every step is judged by the rules the reachability preview runs, so the two agree
    let state = world.world();
    let occupied = movement::occupied_tiles(state, Some(unit));
    let mut current = origin;
    let mut left = movement_left;
    let mut spent = 0u8;
    for step in path {
        match movement::step_cost(state, &occupied, current, *step, left) {
            Ok((cost, entered)) => {
                left -= cost;
                spent += cost;
                current = entered;
            }
            Err(StepRefusal::Occupied) => return Err(CommandRejection::TileOccupied),
            Err(refusal) => {
                return Err(CommandRejection::IllegalMove {
                    reason: refusal.reason(),
                });
            }
        }
    }

    let from = world.world().tile_index(origin).unwrap_or_default();
    let to = world.world().tile_index(current).unwrap_or_default();
    let steps = spent;

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
    template: &SimulationID,
    tile: CubeCoords,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    let (position, settlement, cost, turns) = {
        let state = world.world();
        let definition = state
            .building_template(template)
            .ok_or_else(|| CommandRejection::UnknownDefinition(template.clone()))?;
        let off_map = CommandRejection::IllegalPlacement {
            reason: "the tile is off the map",
        };
        let position = state.shape().wrap(tile).ok_or(off_map.clone())?;
        let ground = state.tile(position).ok_or(off_map)?;
        if ground.artificial().owner() != Some(issuer) {
            return Err(CommandRejection::IllegalPlacement {
                reason: "the tile is not the nation's land",
            });
        }
        if ground.natural().terrain().is_water() {
            return Err(CommandRejection::IllegalPlacement {
                reason: "a building cannot stand on water",
            });
        }
        // one building per tile: the client's single building slot
        if !ground.artificial().buildings().is_empty() {
            return Err(CommandRejection::IllegalPlacement {
                reason: "the tile is already built on",
            });
        }
        let district = ground.artificial().district();
        if district.is_some_and(|district| !district.is_laid_out()) {
            return Err(CommandRejection::IllegalPlacement {
                reason: "a district is still being laid out on the tile",
            });
        }
        if definition.deposit().is_none() && district.is_none() {
            return Err(CommandRejection::NoDistrict);
        }
        // judged against the truth, like every order: an extractor on ground the nation never
        // looked at is refused on what is there, not on what it knows
        if let Some(resource) = definition.deposit() {
            let holds = ground
                .natural()
                .deposits()
                .iter()
                .filter_map(|&deposit| state.deposits().get(deposit))
                .any(|deposit| deposit.resource() == Some(resource) && deposit.amount() > 0);
            if !holds {
                return Err(CommandRejection::MissingDeposit { resource });
            }
        }
        let settlement = match district {
            Some(district) => {
                let settlement = district.settlement();
                if state
                    .settlements()
                    .get(settlement)
                    .is_none_or(|seat| seat.owner() != issuer)
                {
                    return Err(CommandRejection::NotYours(EntityRef::Settlement(
                        settlement,
                    )));
                }
                settlement
            }
            None => {
                nearest_settlement(state, issuer, position).ok_or(CommandRejection::NoSettlement)?
            }
        };
        let level = definition.first_level();
        affordable(state, issuer, &level.cost)?;
        (position, settlement, level.cost.clone(), level.turns)
    };

    pay(world, issuer, &cost);
    let building = world.insert_building(Building::order(
        template.clone(),
        issuer,
        settlement,
        position,
        turns,
    ));

    events.push(GameEvent::ConstructionStarted {
        nation: issuer,
        building,
        settlement,
    });
    Ok(())
}

fn upgrade_building(
    building: BuildingId,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    let (level, cost, turns) = {
        let state = world.world();
        let target = state
            .buildings()
            .get(building)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Building(
                building,
            )))?;
        if target.owner() != issuer {
            return Err(CommandRejection::NotYours(EntityRef::Building(building)));
        }
        // a building still under its construction has works too: it is raised once it stands
        if target.works().is_some() {
            return Err(CommandRejection::WorksUnderWay);
        }
        let definition = state
            .building_template(target.template())
            .ok_or_else(|| CommandRejection::UnknownDefinition(target.template().clone()))?;
        let level = target
            .level()
            .checked_add(1)
            .ok_or(CommandRejection::AtMaxLevel)?;
        let next = definition
            .level(level)
            .ok_or(CommandRejection::AtMaxLevel)?;
        affordable(state, issuer, &next.cost)?;
        (level, next.cost.clone(), next.turns)
    };

    pay(world, issuer, &cost);
    if let Some(target) = world.building_mut(building) {
        target.start_upgrade(turns);
    }

    events.push(GameEvent::UpgradeStarted {
        nation: issuer,
        building,
        level,
    });
    Ok(())
}

fn lay_out_district(
    settlement: SettlementId,
    tile: CubeCoords,
    issuer: NationId,
    world: &mut WorldMut<'_>,
    events: &mut EventBuffer,
) -> Result<(), CommandRejection> {
    let (position, terms, name) = {
        let state = world.world();
        let seat = state
            .settlements()
            .get(settlement)
            .ok_or(CommandRejection::UnknownEntity(EntityRef::Settlement(
                settlement,
            )))?;
        if seat.owner() != issuer {
            return Err(CommandRejection::NotYours(EntityRef::Settlement(
                settlement,
            )));
        }
        let off_map = CommandRejection::IllegalPlacement {
            reason: "the tile is off the map",
        };
        let position = state.shape().wrap(tile).ok_or(off_map.clone())?;
        let ground = state.tile(position).ok_or(off_map)?;
        if state.shape().distance(seat.position(), position) != DISTRICT_DISTANCE {
            return Err(CommandRejection::IllegalPlacement {
                reason: "a district is laid out next to its settlement",
            });
        }
        if ground.artificial().owner() != Some(issuer) {
            return Err(CommandRejection::IllegalPlacement {
                reason: "the tile is not the nation's land",
            });
        }
        if ground.natural().terrain().is_water() {
            return Err(CommandRejection::IllegalPlacement {
                reason: "a district cannot be laid out on water",
            });
        }
        if ground.artificial().district().is_some() {
            return Err(CommandRejection::IllegalPlacement {
                reason: "a district is already laid out on the tile",
            });
        }
        if ground.artificial().settlement().is_some() || !ground.artificial().buildings().is_empty()
        {
            return Err(CommandRejection::IllegalPlacement {
                reason: "the tile is already built on",
            });
        }
        let terms = state
            .district_terms(settlement)
            .map_err(CommandRejection::DistrictRefused)?;
        affordable(state, issuer, &terms.cost)?;
        (position, terms, seat.name().to_string())
    };

    pay(world, issuer, &terms.cost);
    if let Some(ground) = world.tile_mut(position) {
        ground
            .artificial_mut()
            .set_district(Some(District::order(settlement, terms.turns)));
    }

    events.push(GameEvent::Log {
        nation: Some(issuer),
        message: format!("{name} lays out a district."),
    });
    Ok(())
}

/// The issuer's settlement nearest a tile, the shortest way round the map, ties to the lowest ID.
fn nearest_settlement(state: &World, issuer: NationId, tile: CubeCoords) -> Option<SettlementId> {
    let shape = state.shape();
    state
        .settlements()
        .iter()
        .filter(|(_, settlement)| settlement.owner() == issuer)
        .min_by_key(|(id, settlement)| (shape.distance(settlement.position(), tile), *id))
        .map(|(id, _)| id)
}

/// Refuse a bill the issuer cannot pay in full: it is checked before a single resource is spent.
fn affordable(
    state: &World,
    issuer: NationId,
    cost: &ConstructionCosts,
) -> Result<(), CommandRejection> {
    let nation = state
        .nations()
        .get(issuer)
        .ok_or(CommandRejection::UnknownEntity(EntityRef::Nation(issuer)))?;
    match economy::shortfall(nation.resources(), cost) {
        Some((resource, missing)) => {
            Err(CommandRejection::InsufficientResources { resource, missing })
        }
        None => Ok(()),
    }
}

/// Pay a bill `affordable` accepted.
fn pay(world: &mut WorldMut<'_>, issuer: NationId, cost: &ConstructionCosts) {
    if let Some(nation) = world.nation_mut(issuer) {
        economy::take(nation.resources_mut(), cost);
    }
}
