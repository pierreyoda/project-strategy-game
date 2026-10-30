//! The world, and the only door through which it is allowed to change.
//!
//! `World` owns every runtime entity in its stores; tiles, nations and commands only ever refer to
//! them by their dense ID. Nothing holds a reference into another entity, which is what lets the
//! whole thing be mutated turn after turn.
//!
//! Mutation goes through `WorldMut`, never through a bare `&mut World`: every write marks what it
//! touched in the journal's pending batch. A write that skips the gate is a change the client will
//! never be told about, and that class of bug is invisible until someone notices a stale hex.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    game::{GameTurnScalar, genesis::WorldGenesis, journal::TurnChanges},
    hex_map::{
        HexMap,
        coordinates::{CubeCoords, OffsetCoords},
        shape::RectangularHexMapShape,
        tile::HexMapTile,
    },
    simulation::{
        buildings::{Building, BuildingTemplate},
        fog::{FogOfWar, NationFog},
        ids::{
            BuildingId, DepositId, EntityIndex, EntityRef, HqUnitId, InfrastructureId, LeaderId,
            NationId, PopulationGroupId, SettlementId, SimulationID, SupplyNodeId, UnitId,
        },
        infrastructure::Infrastructure,
        logistics::SupplyNode,
        military::{HqUnit, Unit, UnitTemplate},
        nations::Nation,
        people::{leaders::Leader, population::PopulationGroup},
        resources::ResourceDeposit,
        settlements::Settlement,
        store::EntityStore,
    },
};

/// Row-major index of a tile, in the order of `RectangularHexMapShape::coords`.
///
/// This is the index the client's buffers are laid out along, and the one the journal records.
pub type TileIndex = u32;

#[derive(Debug)]
pub struct World {
    shape: RectangularHexMapShape,
    map: HexMap,
    /// Current turn (starts at 1).
    turn: GameTurnScalar,
    /// Nation whose orders are currently being taken.
    active_nation: NationId,
    nations: EntityStore<NationId, Nation>,
    settlements: EntityStore<SettlementId, Settlement>,
    units: EntityStore<UnitId, Unit>,
    hq_units: EntityStore<HqUnitId, HqUnit>,
    buildings: EntityStore<BuildingId, Building>,
    infrastructure: EntityStore<InfrastructureId, Infrastructure>,
    supply_nodes: EntityStore<SupplyNodeId, SupplyNode>,
    leaders: EntityStore<LeaderId, Leader>,
    population_groups: EntityStore<PopulationGroupId, PopulationGroup>,
    deposits: EntityStore<DepositId, ResourceDeposit>,
    /// Script-defined definitions, keyed by their `SimulationID::Abstract`.
    unit_templates: BTreeMap<SimulationID, UnitTemplate>,
    building_templates: BTreeMap<SimulationID, BuildingTemplate>,
    fog: FogOfWar,
    /// How the world was generated; empty when it was not.
    genesis: WorldGenesis,
}

impl World {
    /// Build a world over an already generated map.
    ///
    /// The map comes from `procgen`, which depends on `core` rather than the other way around: it is
    /// the caller - the WASM bridge, or a test - that generates it and hands it over.
    pub fn new(shape: RectangularHexMapShape, map: HexMap) -> Self {
        Self {
            shape,
            map,
            turn: 1,
            active_nation: NationId::from_index(0),
            nations: EntityStore::new(),
            settlements: EntityStore::new(),
            units: EntityStore::new(),
            hq_units: EntityStore::new(),
            buildings: EntityStore::new(),
            infrastructure: EntityStore::new(),
            supply_nodes: EntityStore::new(),
            leaders: EntityStore::new(),
            population_groups: EntityStore::new(),
            deposits: EntityStore::new(),
            unit_templates: BTreeMap::new(),
            building_templates: BTreeMap::new(),
            fog: FogOfWar::new(shape.tiles_count()),
            genesis: WorldGenesis::default(),
        }
    }

    pub fn genesis(&self) -> &WorldGenesis {
        &self.genesis
    }

    pub fn shape(&self) -> RectangularHexMapShape {
        self.shape
    }

    pub fn map(&self) -> &HexMap {
        &self.map
    }

    pub fn turn(&self) -> GameTurnScalar {
        self.turn
    }

    pub fn active_nation(&self) -> NationId {
        self.active_nation
    }

    pub fn tiles_count(&self) -> usize {
        self.shape.tiles_count()
    }

    /// Row-major index of a tile, or `None` beyond the poles.
    ///
    /// Coordinates are wrapped around east-west first, so a path that leaves the map to the east
    /// indexes the tile it comes back in on.
    pub fn tile_index(&self, coords: CubeCoords) -> Option<TileIndex> {
        let wrapped = self.shape.wrap(coords)?;
        let offset = OffsetCoords::from(wrapped);
        Some((offset.row() as u32) * self.shape.columns() as u32 + offset.col() as u32)
    }

    /// Coordinates of a tile from its row-major index.
    pub fn tile_coords(&self, index: TileIndex) -> Option<CubeCoords> {
        let columns = self.shape.columns() as u32;
        if index >= self.tiles_count() as u32 {
            return None;
        }
        Some(CubeCoords::from(OffsetCoords::new(
            (index % columns) as i16,
            (index / columns) as i16,
        )))
    }

    /// Every tile within `radius` of `center`, the center included, wrapping around east-west.
    ///
    /// Indices rather than coordinates: they deduplicate the wrap-around for free, and they are
    /// what both callers - claiming land around a settlement, and a unit's field of view - go on to
    /// use anyway.
    pub fn tiles_within(&self, center: CubeCoords, radius: u32) -> Vec<TileIndex> {
        let Some(center_index) = self.tile_index(center) else {
            return Vec::new();
        };
        let mut seen = BTreeSet::from([center_index]);
        let mut frontier = vec![center];
        for _ in 0..radius {
            let mut next = Vec::with_capacity(frontier.len() * 6);
            for coords in frontier.drain(..) {
                for neighbor in self.shape.neighbors(coords) {
                    if let Some(index) = self.tile_index(neighbor)
                        && seen.insert(index)
                    {
                        next.push(neighbor);
                    }
                }
            }
            frontier = next;
        }
        seen.into_iter().collect()
    }

    pub fn tile(&self, coords: CubeCoords) -> Option<&HexMapTile> {
        self.map.tile(&self.shape.wrap(coords)?)
    }

    pub fn nations(&self) -> &EntityStore<NationId, Nation> {
        &self.nations
    }

    pub fn settlements(&self) -> &EntityStore<SettlementId, Settlement> {
        &self.settlements
    }

    pub fn units(&self) -> &EntityStore<UnitId, Unit> {
        &self.units
    }

    pub fn hq_units(&self) -> &EntityStore<HqUnitId, HqUnit> {
        &self.hq_units
    }

    pub fn buildings(&self) -> &EntityStore<BuildingId, Building> {
        &self.buildings
    }

    pub fn infrastructure(&self) -> &EntityStore<InfrastructureId, Infrastructure> {
        &self.infrastructure
    }

    pub fn supply_nodes(&self) -> &EntityStore<SupplyNodeId, SupplyNode> {
        &self.supply_nodes
    }

    pub fn leaders(&self) -> &EntityStore<LeaderId, Leader> {
        &self.leaders
    }

    pub fn population_groups(&self) -> &EntityStore<PopulationGroupId, PopulationGroup> {
        &self.population_groups
    }

    pub fn deposits(&self) -> &EntityStore<DepositId, ResourceDeposit> {
        &self.deposits
    }

    pub fn unit_template(&self, id: &SimulationID) -> Option<&UnitTemplate> {
        self.unit_templates.get(id)
    }

    pub fn building_template(&self, id: &SimulationID) -> Option<&BuildingTemplate> {
        self.building_templates.get(id)
    }

    /// Every registered building template, in a fixed order: the projection numbers them by it.
    pub fn building_templates(&self) -> impl Iterator<Item = (&SimulationID, &BuildingTemplate)> {
        self.building_templates.iter()
    }

    pub fn fog(&self) -> &FogOfWar {
        &self.fog
    }

    pub fn fog_of(&self, nation: NationId) -> Option<&NationFog> {
        self.fog.of(nation)
    }

    /// Register a script-defined unit template. Definitions are set up before the game starts, so
    /// this does not go through the mutation gate.
    pub fn register_unit_template(&mut self, template: UnitTemplate) {
        use crate::simulation::ids::WithSimulationID;
        self.unit_templates.insert(template.id().clone(), template);
    }

    /// Register a script-defined building template. See `register_unit_template`.
    pub fn register_building_template(&mut self, template: BuildingTemplate) {
        use crate::simulation::ids::WithSimulationID;
        self.building_templates
            .insert(template.id().clone(), template);
    }

    /// Add a nation before the game starts, with its fog record.
    pub fn register_nation(&mut self, nation: Nation) -> NationId {
        let id = self.nations.insert(nation);
        self.fog.register(id);
        id
    }

    /// Place a unit while setting the world up, before any client is watching.
    ///
    /// Setup only, which is why it does not go through the mutation gate: nothing has a version to
    /// catch up from yet. Once the session is running, units are placed through `WorldMut`.
    pub fn place_unit(&mut self, unit: Unit) -> UnitId {
        self.units.insert(unit)
    }

    /// Record how the world was generated. Setup only, like `register_nation`: the genesis never
    /// changes afterwards, so there is nothing for the journal to track.
    pub fn set_genesis(&mut self, genesis: WorldGenesis) {
        assert!(
            genesis.is_empty() || genesis.tiles_count() == self.tiles_count(),
            "one sphere per tile"
        );
        self.genesis = genesis;
    }
}

/// The gate every mutation of the world goes through.
///
/// Handing out `&mut World` directly would work exactly once: the first system that writes without
/// marking what it wrote leaves the client rendering a world that no longer exists.
#[derive(Debug)]
pub struct WorldMut<'a> {
    world: &'a mut World,
    changes: &'a mut TurnChanges,
}

impl<'a> WorldMut<'a> {
    pub fn new(world: &'a mut World, changes: &'a mut TurnChanges) -> Self {
        Self { world, changes }
    }

    /// Read-only view, for the validation a mutation needs before it commits to anything.
    pub fn world(&self) -> &World {
        self.world
    }

    /// Mark a tile as needing to be resent, without changing it: what a visibility change does.
    pub fn mark_tile(&mut self, coords: CubeCoords) {
        if let Some(index) = self.world.tile_index(coords) {
            self.changes.mark_tile(index);
        }
    }

    pub fn mark_tile_index(&mut self, index: TileIndex) {
        self.changes.mark_tile(index);
    }

    pub fn tile_mut(&mut self, coords: CubeCoords) -> Option<&mut HexMapTile> {
        let wrapped = self.world.shape.wrap(coords)?;
        let index = self.world.tile_index(wrapped)?;
        self.changes.mark_tile(index);
        self.world.map.tile_mut(&wrapped)
    }

    pub fn nation_mut(&mut self, id: NationId) -> Option<&mut Nation> {
        let nation = self.world.nations.get_mut(id)?;
        self.changes.mark_nation(id);
        Some(nation)
    }

    pub fn settlement_mut(&mut self, id: SettlementId) -> Option<&mut Settlement> {
        let settlement = self.world.settlements.get_mut(id)?;
        self.changes.mark_settlement(id);
        Some(settlement)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        let unit = self.world.units.get_mut(id)?;
        self.changes.mark_unit(id);
        Some(unit)
    }

    pub fn insert_unit(&mut self, unit: Unit) -> UnitId {
        let position = unit.position();
        let id = self.world.units.insert(unit);
        self.changes.mark_unit(id);
        self.mark_tile(position);
        id
    }

    pub fn remove_unit(&mut self, id: UnitId) -> Option<Unit> {
        let unit = self.world.units.remove(id)?;
        self.changes.mark_removed(EntityRef::Unit(id));
        self.mark_tile(unit.position());
        Some(unit)
    }

    pub fn insert_settlement(&mut self, settlement: Settlement) -> SettlementId {
        let (position, owner) = (settlement.position(), settlement.owner());
        let id = self.world.settlements.insert(settlement);
        self.changes.mark_settlement(id);
        if let Some(tile) = self.tile_mut(position) {
            tile.artificial_mut().set_settlement(Some(id));
            tile.artificial_mut().set_owner(Some(owner));
        }
        if let Some(nation) = self.nation_mut(owner) {
            nation.add_settlement(id);
        }
        id
    }

    pub fn insert_building(&mut self, building: Building) -> BuildingId {
        let position = building.position();
        let id = self.world.buildings.insert(building);
        self.changes.mark_building(id);
        if let Some(tile) = self.tile_mut(position) {
            tile.artificial_mut().add_building(id);
        }
        id
    }

    /// The fog record of a nation, for the sighting pass.
    ///
    /// Visibility changes dirty their tiles through `mark_tile_index`, done by the caller: the fog
    /// record itself is not part of the world the client reads, the visibility buffer is.
    pub fn fog_mut(&mut self, nation: NationId) -> Option<&mut NationFog> {
        self.world.fog.of_mut(nation)
    }

    /// Advance to the next turn. Every nation is re-sent, since their per-turn figures moved.
    pub fn advance_turn(&mut self) -> GameTurnScalar {
        self.world.turn += 1;
        for id in self.world.nations.ids() {
            self.changes.mark_nation(id);
        }
        self.world.turn
    }

    pub fn set_active_nation(&mut self, nation: NationId) {
        self.world.active_nation = nation;
        self.changes.mark_nation(nation);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        game::journal::TurnChanges,
        hex_map::{
            HexMap, HexMapStorage, coordinates::OffsetCoords, shape::RectangularHexMapShape,
            tile::HexMapTile,
        },
        simulation::{
            ids::{EntityIndex, NationId, SimulationID},
            military::Unit,
            nations::Nation,
        },
    };

    use super::{World, WorldMut};

    pub(crate) fn build_test_world(columns: i16, rows: i16) -> World {
        let shape = RectangularHexMapShape::new(columns, rows);
        let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
        for coords in shape.coords() {
            tiles.insert(coords, HexMapTile::from_properties(100));
        }
        World::new(shape, HexMap::from_tiles(tiles))
    }

    #[test]
    fn test_tile_index_round_trip_and_wrapping() {
        let world = build_test_world(6, 4);
        for (index, coords) in world.shape().coords().enumerate() {
            assert_eq!(world.tile_index(coords), Some(index as u32));
            assert_eq!(world.tile_coords(index as u32), Some(coords));
        }
        assert_eq!(world.tile_coords(24), None);

        // east-west wrapping: one past the last column is the first one
        let east_of_last = OffsetCoords::new(6, 2).into();
        assert_eq!(
            world.tile_index(east_of_last),
            world.tile_index(OffsetCoords::new(0, 2).into())
        );
        // but there is nothing beyond the poles
        assert_eq!(world.tile_index(OffsetCoords::new(0, 4).into()), None);
    }

    #[test]
    fn test_every_mutation_marks_what_it_touched() {
        let mut world = build_test_world(6, 4);
        let nation = world.register_nation(Nation::new(
            SimulationID::new_abstract_id("test_nation"),
            "Test".to_string(),
        ));
        let mut changes = TurnChanges::new();
        let mut world_mut = WorldMut::new(&mut world, &mut changes);

        let position = OffsetCoords::new(2, 1).into();
        let unit = world_mut.insert_unit(Unit::recruit(
            SimulationID::new_abstract_id("infantry"),
            nation,
            position,
        ));
        world_mut
            .tile_mut(OffsetCoords::new(3, 1).into())
            .unwrap()
            .artificial_mut()
            .set_owner(Some(NationId::from_index(0)));

        assert_eq!(changes.units().collect::<Vec<_>>(), vec![unit]);
        assert_eq!(changes.tiles().collect::<Vec<_>>(), vec![8, 9]);
    }
}
