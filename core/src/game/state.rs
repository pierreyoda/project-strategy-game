//! The world, and the only door through which it is allowed to change.
//!
//! `World` owns every runtime entity in its stores; tiles, nations and commands only ever refer to
//! them by their dense ID. Nothing holds a reference into another entity, which is what lets the
//! whole thing be mutated turn after turn.
//!
//! Mutation goes through `WorldMut`, never through a bare `&mut World`: every write marks what it
//! touched in the journal's pending batch. A write that skips the gate is a change the client will
//! never be told about, and that class of bug is invisible until someone notices a stale hex.

use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet, BinaryHeap},
};

use crate::{
    game::{GameTurnScalar, genesis::WorldGenesis, journal::TurnChanges},
    hex_map::{
        HexMap,
        coordinates::{CubeCoords, OffsetCoords},
        shape::RectangularHexMapShape,
        terrain::TerrainKind,
        tile::HexMapTile,
        vegetation::Vegetation,
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

    /// The tiles a watcher on `from` sees with this sight, in ascending order: every tile at most
    /// `sight` steps away, each step out of a forest costing one more - the edge of a forest is
    /// in sight, what lies past it less so, and a watcher standing in one sees a step less. The
    /// tiles next to the watcher are always in sight.
    pub fn tiles_in_sight(&self, from: CubeCoords, sight: u32) -> Vec<TileIndex> {
        let Some(origin) = self.tile_index(from) else {
            return Vec::new();
        };
        let mut seen = BTreeSet::from([origin]);
        let mut spent: BTreeMap<TileIndex, u32> = BTreeMap::from([(origin, 0)]);
        let mut queue = BinaryHeap::from([Reverse((0u32, origin))]);
        while let Some(Reverse((cost, index))) = queue.pop() {
            if spent.get(&index).is_some_and(|&known| known < cost) {
                continue;
            }
            let Some(coords) = self.tile_coords(index) else {
                continue;
            };
            let leaving = self.tile(coords).map_or(1, |tile| {
                1 + u32::from(tile.natural().vegetation().is_forest())
            });
            let reached = cost + leaving;
            for neighbor in self.shape.neighbors(coords) {
                let Some(next) = self.tile_index(neighbor) else {
                    continue;
                };
                if index == origin {
                    seen.insert(next);
                }
                if reached <= sight && spent.get(&next).is_none_or(|&known| reached < known) {
                    seen.insert(next);
                    spent.insert(next, reached);
                    queue.push(Reverse((reached, next)));
                }
            }
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

    /// Every registered unit template, in a fixed order: the projection numbers them by it.
    pub fn unit_templates(&self) -> impl Iterator<Item = (&SimulationID, &UnitTemplate)> {
        self.unit_templates.iter()
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

    /// Write the terrain of every tile, row-major like the projection's buffers, as `procgen`
    /// classified it. Done when the world is set up, before anything has a version to catch up from,
    /// so this does not go through the mutation gate. Panics on a slice of the wrong length: a map
    /// and its classification come from the same generator, and a mismatch is a bug, not an input.
    pub fn paint_terrain(&mut self, kinds: &[TerrainKind]) {
        assert_eq!(
            kinds.len(),
            self.tiles_count(),
            "one terrain kind per tile, in row-major order"
        );
        for (coords, &kind) in self.shape.coords().zip(kinds) {
            if let Some(tile) = self.map.tile_mut(&coords) {
                tile.natural_mut().set_terrain(kind);
            }
        }
    }

    /// Write the vegetation of every tile, row-major like `paint_terrain` and for the same reasons
    /// outside the mutation gate. Panics on a slice of the wrong length.
    pub fn paint_vegetation(&mut self, vegetation: &[Vegetation]) {
        assert_eq!(
            vegetation.len(),
            self.tiles_count(),
            "one vegetation per tile, in row-major order"
        );
        for (coords, &grown) in self.shape.coords().zip(vegetation) {
            if let Some(tile) = self.map.tile_mut(&coords) {
                tile.natural_mut().set_vegetation(grown);
            }
        }
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
            tile::HexMapTile, vegetation::Vegetation,
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

    /// Tiles at most `radius` hexes from the center, by the plain disc.
    fn disc(world: &World, center: OffsetCoords, radius: u32) -> Vec<u32> {
        world.tiles_within(center.into(), radius)
    }

    fn plant(world: &mut World, wooded: &[(i16, i16)]) {
        let mut vegetation = vec![Vegetation::None; world.tiles_count()];
        for &(col, row) in wooded {
            let index = world
                .tile_index(OffsetCoords::new(col, row).into())
                .unwrap();
            vegetation[index as usize] = Vegetation::Forest;
        }
        world.paint_vegetation(&vegetation);
    }

    #[test]
    fn test_sight_over_bare_ground_is_the_plain_disc() {
        let world = build_test_world(12, 9);
        let center = OffsetCoords::new(5, 4);
        for sight in 0..=3 {
            assert_eq!(
                world.tiles_in_sight(center.into(), sight),
                if sight == 0 {
                    disc(&world, center, 1)
                } else {
                    disc(&world, center, sight)
                },
                "sight {sight}: the neighbors are always in sight"
            );
        }
    }

    #[test]
    fn test_a_forest_hides_what_lies_past_it() {
        let mut world = build_test_world(12, 9);
        // a wooded tile two hexes east of the watcher, on its row
        plant(&mut world, &[(7, 4)]);
        let seen = world.tiles_in_sight(OffsetCoords::new(5, 4).into(), 3);
        let index = |col, row| {
            world
                .tile_index(OffsetCoords::new(col, row).into())
                .unwrap()
        };
        assert!(seen.contains(&index(7, 4)), "the edge of the forest");
        assert!(!seen.contains(&index(8, 4)), "right behind it");
        assert!(seen.contains(&index(2, 4)), "the open side, as far as ever");
    }

    #[test]
    fn test_a_watcher_in_a_forest_sees_a_step_less() {
        let mut world = build_test_world(12, 9);
        plant(&mut world, &[(5, 4)]);
        let center = OffsetCoords::new(5, 4);
        assert_eq!(
            world.tiles_in_sight(center.into(), 2),
            disc(&world, center, 1)
        );
        assert_eq!(
            world.tiles_in_sight(center.into(), 1),
            disc(&world, center, 1),
            "but always its neighbors"
        );
    }

    #[test]
    fn test_paint_vegetation_reads_back_per_tile() {
        let mut world = build_test_world(4, 3);
        plant(&mut world, &[(1, 1)]);
        let wooded: Vec<bool> = world
            .shape()
            .coords()
            .map(|coords| {
                world
                    .tile(coords)
                    .unwrap()
                    .natural()
                    .vegetation()
                    .is_forest()
            })
            .collect();
        assert_eq!(wooded.iter().filter(|&&w| w).count(), 1);
        assert!(wooded[5]);
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
