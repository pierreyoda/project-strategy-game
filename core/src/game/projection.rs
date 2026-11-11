//! What one nation is shown of the world.
//!
//! Two shapes, one rule: a projection is always built **for a viewing nation**, and never contains
//! anything that nation cannot see. Shipping the whole world and hiding it client-side would be
//! simpler and would survive exactly until someone opens the devtools.
//!
//! The buffers are structure-of-arrays on purpose. They cross into JavaScript as typed arrays, one
//! copy each, and a 1024x1024 map is a million tiles: an object per tile is not an option. The dense
//! IDs of `simulation::ids` are what makes this cheap, since an ID is already an index.

use crate::{
    game::{
        GameTurnScalar,
        journal::{StateVersion, TurnChanges},
        state::{TileIndex, World},
    },
    simulation::{
        buildings::{Building, BuildingLevel},
        economy,
        fog::TileVisibility,
        ids::{BuildingId, DepositId, NationId, SettlementId, SimulationID, UnitId},
        military::Unit,
        resources::{Resource, ResourceData, ResourceDataStore, ResourceDeposit},
        settlements::{Settlement, SettlementStage},
    },
};
use std::collections::BTreeMap;

/// The "no nation" value of the owner buffers: the sentinel `NationId` reserves (see `EntityIndex`).
pub const NO_NATION: u16 = u16::MAX;

/// The kind of a building whose template is not registered, which only a corrupt world produces.
pub const UNKNOWN_BUILDING_KIND: u8 = u8::MAX;

/// The kind of a unit whose template is not registered, likewise.
pub const UNKNOWN_UNIT_KIND: u8 = u8::MAX;

/// The resource of a deposit holding none, which only a corrupt world produces.
pub const UNKNOWN_RESOURCE: u8 = u8::MAX;

/// The turns-left value of a deposit nothing draws on.
pub const NO_TURNS_LEFT: u16 = u16::MAX;

/// What a unit template tells the client: enough to name a unit and to show its figures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitTemplateMeta {
    pub r#type: String,
    pub movement: u8,
    pub sight: u8,
    /// Companies a unit is recruited with.
    pub companies: u8,
}

/// What a building template tells the client: what to order it by, what it is called, what it
/// extracts, and every level - the sprite it is drawn with, and the figures the panels show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildingTemplateMeta {
    /// What a construct order names it by: its ID, which a scenario may give a type of another's.
    pub id: String,
    pub r#type: String,
    pub deposit: Option<Resource>,
    pub levels: Vec<BuildingLevel>,
}

/// Bits of the per-building state buffer.
pub mod building_states {
    /// Its first construction is under way: it is level 0, and works nothing yet.
    pub const UNDER_CONSTRUCTION: u8 = 1 << 0;
    /// An upgrade is under way: it works at its level meanwhile.
    pub const UPGRADING: u8 = 1 << 1;
    /// Its maintenance went unpaid: it makes nothing. Only ever set for its owner.
    pub const HALTED: u8 = 1 << 2;
}

/// Bits of the per-tile flags buffer.
pub mod tile_flags {
    pub const SETTLEMENT: u8 = 1 << 0;
    pub const BUILDING: u8 = 1 << 1;
    pub const SUPPLY_NODE: u8 = 1 << 2;
    pub const INFRASTRUCTURE: u8 = 1 << 3;
    pub const DEPOSIT: u8 = 1 << 4;
}

/// Past this share of the map being dirty, a delta costs more than a snapshot.
const FULL_SNAPSHOT_DIRTY_RATIO: f32 = 0.2;

/// A stockpile, and what the next turn does to it: `income - depletion` is the change it will see.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceMeta {
    pub resource: Resource,
    pub quantity: u16,
    /// Gained next turn: the nation's income, and what its buildings make (see `economy::forecast`).
    pub income: u16,
    /// Lost next turn: the nation's depletion, and its buildings' maintenance.
    pub depletion: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NationMeta {
    pub id: u16,
    pub name: String,
    /// Stockpiles, only for the viewer: what another nation holds is not public knowledge.
    pub resources: Vec<ResourceMeta>,
}

/// The small, irregular part of a projection, which travels as JSON rather than as a buffer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotMeta {
    pub version: StateVersion,
    pub turn: GameTurnScalar,
    pub viewer: u16,
    pub active_nation: u16,
    pub nations: Vec<NationMeta>,
    /// The registered building templates, in the fixed order the `building_kinds` buffers index
    /// into. Templates are script-defined, so the list is the game's rather than a hardcoded enum;
    /// it is small and never changes during a playthrough.
    pub building_templates: Vec<BuildingTemplateMeta>,
    /// The registered unit templates, in the fixed order the `unit_kinds` buffers index into. Same
    /// reasoning as `building_templates`.
    pub unit_templates: Vec<UnitTemplateMeta>,
}

/// Everything the viewer can see, from scratch.
#[derive(Clone, Debug)]
pub struct WorldSnapshot {
    pub meta: SnapshotMeta,
    pub columns: i16,
    pub rows: i16,
    /// In meters, zeroed on tiles the viewer has never explored.
    pub elevations: Vec<i16>,
    /// `TileVisibility` discriminants.
    pub visibility: Vec<u8>,
    /// Owning nation per tile, `NO_NATION` where none or unknown.
    pub tile_owner: Vec<u16>,
    /// Nation whose sphere of influence holds each tile (see `WorldGenesis`), `NO_NATION` where
    /// none or unknown: the viewer's own sphere everywhere, a rival's only once the tile is explored.
    pub tile_sphere: Vec<u16>,
    /// `tile_flags` bits.
    pub tile_flags: Vec<u8>,
    pub unit_ids: Vec<u32>,
    pub unit_tiles: Vec<u32>,
    pub unit_owners: Vec<u16>,
    /// `Strength`: every company, then those suppressed and those lost - the viewer's own units'
    /// and a rival's alike, since what is in sight is in sight.
    pub unit_companies: Vec<u8>,
    pub unit_suppressed: Vec<u8>,
    pub unit_lost: Vec<u8>,
    /// Indices into the meta's `unit_templates`, or `UNKNOWN_UNIT_KIND`.
    pub unit_kinds: Vec<u8>,
    /// Movement points left this turn.
    pub unit_movement_left: Vec<u8>,
    pub settlement_ids: Vec<u32>,
    pub settlement_tiles: Vec<u32>,
    pub settlement_owners: Vec<u16>,
    /// `SettlementStage` discriminants.
    pub settlement_stages: Vec<u8>,
    pub building_ids: Vec<u32>,
    pub building_tiles: Vec<u32>,
    pub building_owners: Vec<u16>,
    /// Indices into the meta's `building_templates`, or `UNKNOWN_BUILDING_KIND`.
    pub building_kinds: Vec<u8>,
    /// `Building::level`: 0 while it is first being built.
    pub building_levels: Vec<u8>,
    /// `building_states` bits: its works, and for the viewer's own whether it is halted.
    pub building_states: Vec<u8>,
    /// Turns its works have left, and take in all; zero without works, and for a rival's.
    pub building_turns_left: Vec<u16>,
    pub building_turns_total: Vec<u16>,
    pub deposits: DepositsBuffers,
}

/// The deposits the viewer knows of, as parallel arrays: every deposit on an explored tile.
///
/// Known rather than watched, like settlements: a deposit does not walk away, and the ground the
/// viewer has explored is the ground it has surveyed. TODO: a survey of its own, once there is one.
#[derive(Clone, Debug, Default)]
pub struct DepositsBuffers {
    pub ids: Vec<u32>,
    pub tiles: Vec<u32>,
    /// `Resource` discriminants, in the order of the client's `RESOURCES`, or `UNKNOWN_RESOURCE`.
    pub resources: Vec<u8>,
    /// `ResourceDeposit::level`: 1 to `DEPOSIT_LEVELS`, 0 once exhausted.
    pub levels: Vec<u8>,
    /// The stock left, zero for a deposit that springs back instead.
    pub quantities: Vec<u16>,
    /// What it recharges per turn, zero for a stock.
    pub recharges: Vec<u16>,
    /// `ResourceDeposit::turns_left`, or `NO_TURNS_LEFT`.
    pub turns_left: Vec<u16>,
}

impl DepositsBuffers {
    fn push(&mut self, id: DepositId, tile: TileIndex, deposit: &ResourceDeposit) {
        let resource = deposit.resource();
        let data = resource.and_then(|resource| deposit.resources().get(&resource));
        self.ids.push(id.raw());
        self.tiles.push(tile);
        self.resources
            .push(resource.map_or(UNKNOWN_RESOURCE, |resource| resource as u8));
        self.levels.push(deposit.level());
        self.quantities.push(data.map_or(0, ResourceData::quantity));
        self.recharges
            .push(data.and_then(ResourceData::income).unwrap_or(0));
        self.turns_left
            .push(deposit.turns_left().unwrap_or(NO_TURNS_LEFT));
    }
}

/// What changed for the viewer since a given version.
///
/// The tile arrays are parallel to `tile_indices`, not to the map.
#[derive(Clone, Debug)]
pub struct WorldDelta {
    pub meta: SnapshotMeta,
    pub tile_indices: Vec<TileIndex>,
    pub elevations: Vec<i16>,
    pub visibility: Vec<u8>,
    pub tile_owner: Vec<u16>,
    pub tile_sphere: Vec<u16>,
    pub tile_flags: Vec<u8>,
    pub unit_ids: Vec<u32>,
    pub unit_tiles: Vec<u32>,
    pub unit_owners: Vec<u16>,
    /// `Strength`: every company, then those suppressed and those lost - the viewer's own units'
    /// and a rival's alike, since what is in sight is in sight.
    pub unit_companies: Vec<u8>,
    pub unit_suppressed: Vec<u8>,
    pub unit_lost: Vec<u8>,
    /// Indices into the meta's `unit_templates`, or `UNKNOWN_UNIT_KIND`.
    pub unit_kinds: Vec<u8>,
    /// Movement points left this turn.
    pub unit_movement_left: Vec<u8>,
    /// Units the viewer must drop: destroyed, or simply no longer in sight.
    pub units_removed: Vec<u32>,
    pub settlement_ids: Vec<u32>,
    pub settlement_tiles: Vec<u32>,
    pub settlement_owners: Vec<u16>,
    /// `SettlementStage` discriminants.
    pub settlement_stages: Vec<u8>,
    pub settlements_removed: Vec<u32>,
    pub building_ids: Vec<u32>,
    pub building_tiles: Vec<u32>,
    pub building_owners: Vec<u16>,
    /// Indices into the meta's `building_templates`, or `UNKNOWN_BUILDING_KIND`.
    pub building_kinds: Vec<u8>,
    /// `Building::level`: 0 while it is first being built.
    pub building_levels: Vec<u8>,
    /// `building_states` bits: its works, and for the viewer's own whether it is halted.
    pub building_states: Vec<u8>,
    /// Turns its works have left, and take in all; zero without works, and for a rival's.
    pub building_turns_left: Vec<u16>,
    pub building_turns_total: Vec<u16>,
    pub buildings_removed: Vec<u32>,
    /// Deposits on the tiles that changed and that the viewer knows: those just explored, and those
    /// an extractor drew on - `deposit_mut` marks the tile. None is ever removed: one run dry stays,
    /// at level 0.
    pub deposits: DepositsBuffers,
}

/// Either a catch-up, or an order to start over.
///
/// Which one it is is **core's** decision, not the client's: one rule, in one place.
#[derive(Clone, Debug)]
pub enum Projection {
    Delta(WorldDelta),
    Full(WorldSnapshot),
}

/// Build everything the viewer can see.
pub fn snapshot(world: &World, viewer: NationId, version: StateVersion) -> WorldSnapshot {
    let shape = world.shape();
    let tiles_count = world.tiles_count();
    let mut snapshot = WorldSnapshot {
        meta: meta(world, viewer, version),
        columns: shape.columns(),
        rows: shape.rows(),
        elevations: vec![0; tiles_count],
        visibility: vec![TileVisibility::Unexplored as u8; tiles_count],
        tile_owner: vec![NO_NATION; tiles_count],
        tile_sphere: vec![NO_NATION; tiles_count],
        tile_flags: vec![0; tiles_count],
        unit_ids: Vec::new(),
        unit_tiles: Vec::new(),
        unit_owners: Vec::new(),
        unit_companies: Vec::new(),
        unit_suppressed: Vec::new(),
        unit_lost: Vec::new(),
        unit_kinds: Vec::new(),
        unit_movement_left: Vec::new(),
        settlement_ids: Vec::new(),
        settlement_tiles: Vec::new(),
        settlement_owners: Vec::new(),
        settlement_stages: Vec::new(),
        building_ids: Vec::new(),
        building_tiles: Vec::new(),
        building_owners: Vec::new(),
        building_kinds: Vec::new(),
        building_levels: Vec::new(),
        building_states: Vec::new(),
        building_turns_left: Vec::new(),
        building_turns_total: Vec::new(),
        deposits: DepositsBuffers::default(),
    };

    for (index, coords) in shape.coords().enumerate() {
        let visibility = visibility_of(world, viewer, index);
        snapshot.visibility[index] = visibility as u8;
        snapshot.tile_sphere[index] = sphere_of(world, viewer, index as TileIndex, visibility);
        if visibility == TileVisibility::Unexplored {
            continue;
        }
        let Some(tile) = world.tile(coords) else {
            continue;
        };
        snapshot.elevations[index] = tile.elevation();
        snapshot.tile_owner[index] = tile
            .artificial()
            .owner()
            .map_or(NO_NATION, |owner| owner.raw());
        snapshot.tile_flags[index] = flags_of(world, coords);
    }

    let unit_kinds = unit_kinds(world);
    for (id, unit) in world.units().iter() {
        let Some(index) = world.tile_index(unit.position()) else {
            continue;
        };
        // a unit is only ever seen where the viewer is actually looking
        if visibility_of(world, viewer, index as usize) != TileVisibility::Visible {
            continue;
        }
        snapshot.unit_ids.push(id.raw());
        snapshot.unit_tiles.push(index);
        snapshot.unit_owners.push(unit.owner().raw());
        let strength = unit.strength();
        snapshot.unit_companies.push(strength.companies());
        snapshot.unit_suppressed.push(strength.suppressed());
        snapshot.unit_lost.push(strength.lost());
        snapshot.unit_kinds.push(unit_kind_of(&unit_kinds, unit));
        snapshot.unit_movement_left.push(unit.movement_left());
    }

    for (id, settlement) in world.settlements().iter() {
        let Some(index) = world.tile_index(settlement.position()) else {
            continue;
        };
        // a settlement stays on the map once seen: it is not going anywhere
        if visibility_of(world, viewer, index as usize) == TileVisibility::Unexplored {
            continue;
        }
        snapshot.settlement_ids.push(id.raw());
        snapshot.settlement_tiles.push(index);
        snapshot.settlement_owners.push(settlement.owner().raw());
        snapshot
            .settlement_stages
            .push(stage_of(world, id, settlement) as u8);
    }

    let kinds = building_kinds(world);
    for (id, building) in world.buildings().iter() {
        let Some(index) = world.tile_index(building.position()) else {
            continue;
        };
        // like a settlement: once seen, it stays on the map
        if visibility_of(world, viewer, index as usize) == TileVisibility::Unexplored {
            continue;
        }
        snapshot.building_ids.push(id.raw());
        snapshot.building_tiles.push(index);
        snapshot.building_owners.push(building.owner().raw());
        snapshot.building_kinds.push(kind_of(&kinds, building));
        snapshot.building_levels.push(building.level());
        let (state, turns_left, turns_total) = building_state(building, viewer);
        snapshot.building_states.push(state);
        snapshot.building_turns_left.push(turns_left);
        snapshot.building_turns_total.push(turns_total);
    }

    for (id, deposit) in world.deposits().iter() {
        let Some(index) = world.tile_index(deposit.position()) else {
            continue;
        };
        if visibility_of(world, viewer, index as usize) != TileVisibility::Unexplored {
            snapshot.deposits.push(id, index, deposit);
        }
    }

    snapshot
}

/// Build the catch-up for a viewer that last saw `changes`' starting version.
pub fn delta(world: &World, viewer: NationId, changes: &TurnChanges) -> WorldDelta {
    let mut delta = WorldDelta {
        meta: meta(world, viewer, changes.version()),
        tile_indices: Vec::with_capacity(changes.tiles_count()),
        elevations: Vec::with_capacity(changes.tiles_count()),
        visibility: Vec::with_capacity(changes.tiles_count()),
        tile_owner: Vec::with_capacity(changes.tiles_count()),
        tile_sphere: Vec::with_capacity(changes.tiles_count()),
        tile_flags: Vec::with_capacity(changes.tiles_count()),
        unit_ids: Vec::new(),
        unit_tiles: Vec::new(),
        unit_owners: Vec::new(),
        unit_companies: Vec::new(),
        unit_suppressed: Vec::new(),
        unit_lost: Vec::new(),
        unit_kinds: Vec::new(),
        unit_movement_left: Vec::new(),
        units_removed: Vec::new(),
        settlement_ids: Vec::new(),
        settlement_tiles: Vec::new(),
        settlement_owners: Vec::new(),
        settlement_stages: Vec::new(),
        settlements_removed: Vec::new(),
        building_ids: Vec::new(),
        building_tiles: Vec::new(),
        building_owners: Vec::new(),
        building_kinds: Vec::new(),
        building_levels: Vec::new(),
        building_states: Vec::new(),
        building_turns_left: Vec::new(),
        building_turns_total: Vec::new(),
        buildings_removed: Vec::new(),
        deposits: DepositsBuffers::default(),
    };

    let dirty_tiles: Vec<TileIndex> = changes.tiles().collect();
    for &index in &dirty_tiles {
        let Some(coords) = world.tile_coords(index) else {
            continue;
        };
        let visibility = visibility_of(world, viewer, index as usize);
        delta.tile_indices.push(index);
        delta.visibility.push(visibility as u8);
        delta
            .tile_sphere
            .push(sphere_of(world, viewer, index, visibility));
        if visibility == TileVisibility::Unexplored {
            delta.elevations.push(0);
            delta.tile_owner.push(NO_NATION);
            delta.tile_flags.push(0);
            continue;
        }
        let elevation = world.tile(coords).map_or(0, |tile| tile.elevation());
        let owner = world
            .tile(coords)
            .and_then(|tile| tile.artificial().owner())
            .map_or(NO_NATION, |owner| owner.raw());
        delta.elevations.push(elevation);
        delta.tile_owner.push(owner);
        delta.tile_flags.push(flags_of(world, coords));
    }

    // Entities the turn touched, plus every entity standing on a tile whose visibility moved: a
    // unit that did not budge while the viewer lost sight of it still has to leave the client's map.
    let mut units: Vec<UnitId> = changes.units().collect();
    let mut settlements: Vec<SettlementId> = changes.settlements().collect();
    let mut buildings: Vec<BuildingId> = changes.buildings().collect();
    for &index in &dirty_tiles {
        for (id, unit) in world.units().iter() {
            if world.tile_index(unit.position()) == Some(index) && !units.contains(&id) {
                units.push(id);
            }
        }
        for (id, settlement) in world.settlements().iter() {
            if world.tile_index(settlement.position()) == Some(index) && !settlements.contains(&id)
            {
                settlements.push(id);
            }
        }
        for (id, building) in world.buildings().iter() {
            if world.tile_index(building.position()) == Some(index) && !buildings.contains(&id) {
                buildings.push(id);
            }
        }
    }
    units.sort_unstable();
    settlements.sort_unstable();
    buildings.sort_unstable();

    for &index in &dirty_tiles {
        if visibility_of(world, viewer, index as usize) == TileVisibility::Unexplored {
            continue;
        }
        let Some(tile) = world
            .tile_coords(index)
            .and_then(|coords| world.tile(coords))
        else {
            continue;
        };
        for &id in tile.natural().deposits() {
            if let Some(deposit) = world.deposits().get(id) {
                delta.deposits.push(id, index, deposit);
            }
        }
    }

    let unit_kinds = unit_kinds(world);
    for id in units {
        match world.units().get(id) {
            Some(unit) => {
                let index = world.tile_index(unit.position());
                let visible = index.is_some_and(|index| {
                    visibility_of(world, viewer, index as usize) == TileVisibility::Visible
                });
                if visible {
                    delta.unit_ids.push(id.raw());
                    delta.unit_tiles.push(index.unwrap_or_default());
                    delta.unit_owners.push(unit.owner().raw());
                    let strength = unit.strength();
                    delta.unit_companies.push(strength.companies());
                    delta.unit_suppressed.push(strength.suppressed());
                    delta.unit_lost.push(strength.lost());
                    delta.unit_kinds.push(unit_kind_of(&unit_kinds, unit));
                    delta.unit_movement_left.push(unit.movement_left());
                } else {
                    delta.units_removed.push(id.raw());
                }
            }
            None => delta.units_removed.push(id.raw()),
        }
    }

    for id in settlements {
        match world.settlements().get(id) {
            Some(settlement) => {
                let index = world.tile_index(settlement.position());
                let known = index.is_some_and(|index| {
                    visibility_of(world, viewer, index as usize) != TileVisibility::Unexplored
                });
                if known {
                    delta.settlement_ids.push(id.raw());
                    delta.settlement_tiles.push(index.unwrap_or_default());
                    delta.settlement_owners.push(settlement.owner().raw());
                    delta
                        .settlement_stages
                        .push(stage_of(world, id, settlement) as u8);
                } else {
                    delta.settlements_removed.push(id.raw());
                }
            }
            None => delta.settlements_removed.push(id.raw()),
        }
    }

    let kinds = building_kinds(world);
    for id in buildings {
        match world.buildings().get(id) {
            Some(building) => {
                let index = world.tile_index(building.position());
                let known = index.is_some_and(|index| {
                    visibility_of(world, viewer, index as usize) != TileVisibility::Unexplored
                });
                if known {
                    delta.building_ids.push(id.raw());
                    delta.building_tiles.push(index.unwrap_or_default());
                    delta.building_owners.push(building.owner().raw());
                    delta.building_kinds.push(kind_of(&kinds, building));
                    delta.building_levels.push(building.level());
                    let (state, turns_left, turns_total) = building_state(building, viewer);
                    delta.building_states.push(state);
                    delta.building_turns_left.push(turns_left);
                    delta.building_turns_total.push(turns_total);
                } else {
                    delta.buildings_removed.push(id.raw());
                }
            }
            None => delta.buildings_removed.push(id.raw()),
        }
    }

    delta
}

/// Whether a delta of this many dirty tiles is still worth sending.
pub fn delta_is_worth_it(world: &World, changes: &TurnChanges) -> bool {
    let tiles_count = world.tiles_count();
    tiles_count == 0
        || (changes.tiles_count() as f32) < (tiles_count as f32 * FULL_SNAPSHOT_DIRTY_RATIO)
}

/// How far this settlement has grown: its nation's seat, or whatever its people amount to.
fn stage_of(world: &World, id: SettlementId, settlement: &Settlement) -> SettlementStage {
    let is_capital = world
        .nations()
        .get(settlement.owner())
        .is_some_and(|nation| nation.capital() == Some(id));
    if is_capital {
        return SettlementStage::Capital;
    }
    SettlementStage::for_population(world.population_of(settlement))
}

/// The rank of every unit template, which is its kind in the buffers (see `SnapshotMeta`).
fn unit_kinds(world: &World) -> BTreeMap<&SimulationID, u8> {
    world
        .unit_templates()
        .enumerate()
        .map(|(rank, (id, _))| (id, rank as u8))
        .collect()
}

fn unit_kind_of(kinds: &BTreeMap<&SimulationID, u8>, unit: &Unit) -> u8 {
    kinds
        .get(unit.template())
        .copied()
        .unwrap_or(UNKNOWN_UNIT_KIND)
}

/// The rank of every building template, which is its kind in the buffers (see `SnapshotMeta`).
fn building_kinds(world: &World) -> BTreeMap<&SimulationID, u8> {
    world
        .building_templates()
        .enumerate()
        .map(|(rank, (id, _))| (id, rank as u8))
        .collect()
}

fn kind_of(kinds: &BTreeMap<&SimulationID, u8>, building: &Building) -> u8 {
    kinds
        .get(building.template())
        .copied()
        .unwrap_or(UNKNOWN_BUILDING_KIND)
}

/// What the viewer is shown of a building's works and upkeep: its `building_states` bits, and its
/// works' turns left and total.
///
/// All of it for its own buildings; for a rival's only that works are under way - scaffolding can be
/// seen from across the border, how long it has to go and whether its upkeep is paid cannot.
fn building_state(building: &Building, viewer: NationId) -> (u8, u16, u16) {
    let mut state = 0;
    if building.works().is_some() {
        state |= if building.is_operating() {
            building_states::UPGRADING
        } else {
            building_states::UNDER_CONSTRUCTION
        };
    }
    if building.owner() != viewer {
        return (state, 0, 0);
    }
    if building.halted() {
        state |= building_states::HALTED;
    }
    let (turns_left, turns_total) = building
        .works()
        .map_or((0, 0), |works| (works.turns_left(), works.turns_total()));
    (state, turns_left, turns_total)
}

/// The sphere of influence the viewer is shown on a tile.
///
/// The viewer's own sphere is shown everywhere from turn 1 - it is where they are meant to grow,
/// and keeping it from them would make the generator's work invisible to the one it was done for.
/// A rival's only shows once the tile is explored, like its borders: an unexplored tile reveals
/// nothing, whoever it was meant for.
fn sphere_of(world: &World, viewer: NationId, index: TileIndex, visibility: TileVisibility) -> u16 {
    match world.genesis().sphere_of(index) {
        Some(nation) if nation == viewer || visibility != TileVisibility::Unexplored => {
            nation.raw()
        }
        _ => NO_NATION,
    }
}

fn visibility_of(world: &World, viewer: NationId, index: usize) -> TileVisibility {
    world
        .fog_of(viewer)
        .map_or(TileVisibility::Unexplored, |fog| fog.visibility(index))
}

fn flags_of(world: &World, coords: crate::hex_map::coordinates::CubeCoords) -> u8 {
    let Some(tile) = world.tile(coords) else {
        return 0;
    };
    let (natural, artificial) = (tile.natural(), tile.artificial());
    let mut flags = 0;
    if artificial.settlement().is_some() {
        flags |= tile_flags::SETTLEMENT;
    }
    if !artificial.buildings().is_empty() {
        flags |= tile_flags::BUILDING;
    }
    if artificial.supply_node().is_some() {
        flags |= tile_flags::SUPPLY_NODE;
    }
    if !artificial.infrastructure().is_empty() {
        flags |= tile_flags::INFRASTRUCTURE;
    }
    if !natural.deposits().is_empty() {
        flags |= tile_flags::DEPOSIT;
    }
    flags
}

fn meta(world: &World, viewer: NationId, version: StateVersion) -> SnapshotMeta {
    SnapshotMeta {
        version,
        turn: world.turn(),
        viewer: viewer.raw(),
        active_nation: world.active_nation().raw(),
        nations: world
            .nations()
            .iter()
            .map(|(id, nation)| NationMeta {
                id: id.raw(),
                name: nation.name().to_string(),
                // only the viewer's own stockpiles: what a rival holds is not public knowledge
                resources: if id == viewer {
                    economy::forecast(world, id)
                        .into_iter()
                        .map(|(resource, forecast)| ResourceMeta {
                            resource,
                            quantity: nation.resources().quantity_of(resource),
                            income: forecast.gained,
                            depletion: forecast.spent,
                        })
                        .collect()
                } else {
                    Vec::new()
                },
            })
            .collect(),
        building_templates: world
            .building_templates()
            .map(|(id, template)| BuildingTemplateMeta {
                id: match id {
                    SimulationID::Abstract(name) | SimulationID::Property(name) => name.clone(),
                },
                r#type: template.r#type().to_string(),
                deposit: template.deposit(),
                levels: template.levels().to_vec(),
            })
            .collect(),
        unit_templates: world
            .unit_templates()
            .map(|(_, template)| UnitTemplateMeta {
                r#type: template.r#type().to_string(),
                movement: template.movement(),
                sight: template.sight(),
                companies: template.companies(),
            })
            .collect(),
    }
}
