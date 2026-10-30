//! The game session, as the client holds it.
//!
//! The bridge owns a `GameSession` and nothing else: no reference into the world, no entity handle.
//! What crosses is commands in, events out, and buffers pulled on demand - never an object graph,
//! because a graph costs one JavaScript object per node and the map has up to a million tiles.
//!
//! The session is driven one `step` at a time by the worker on the other side. There is no progress
//! callback here, unlike `procgen_generate_planet`: that one call cannot be interrupted, so it has
//! to shout as it goes, while this one returns after every slice of work and lets its caller decide
//! what to do next.

use std::collections::BTreeMap;

use wasm_bindgen::prelude::{JsError, wasm_bindgen};

use project_x_core::{
    game::{
        ai::stub_ai_for,
        genesis::{Homeland, WorldGenesis},
        journal::TurnChanges,
        projection::{Projection, WorldDelta, WorldSnapshot},
        session::GameSession as CoreSession,
        state::{World, WorldMut},
        turn::{StepBudget, TurnProgress},
    },
    hex_map::shape::RectangularHexMapShape,
    simulation::{
        buildings::BuildingTemplate,
        fog::TileVisibility,
        ids::{EntityIndex, NationId, SimulationID, UnitId},
        military::{Unit, UnitTemplate},
        movement::{self, Reachable},
        nations::Nation,
        resources::{Resource, ResourceDataStore},
        settlements::Settlement,
    },
};
use project_x_procgen::{
    map::{MAX_SEED, MapGenerator, MapGeneratorSettings},
    nations::{GeneratedNations, NationsGenerator, NationsGeneratorSettings, PlanetSurface},
};

use crate::wire::{CommandWire, EventWire, MetaWire, NewGameSettings, RejectionWire};

/// The scaffold's starting roster, named after the client's counters: type, movement, sight.
/// TODO: templates come from scripts.
const UNIT_TEMPLATES: [(&str, u8, u8); 3] = [
    ("infantry_division", 2, 2),
    ("motorized_division", 4, 3),
    ("light_tank_division", 3, 2),
];
/// The scaffold's buildings, named after the client's sprites. TODO: templates come from scripts.
const BUILDING_TEMPLATES: [(&str, Resource, u16); 2] = [
    ("mine", Resource::Metals, 2),
    ("solar_plant", Resource::Electricity, 3),
];

/// Terrain of a tile the viewer has never explored.
///
/// Terrain is classified here rather than in `core`, so it is also filtered here: handing the
/// client the terrain of the whole map and covering it with fog on the other side would leak the
/// shape of the planet to anyone who opens the devtools.
const UNKNOWN_TERRAIN: u8 = 255;

/// One playthrough, driven from the client's worker.
#[wasm_bindgen]
pub struct GameSession {
    inner: CoreSession,
    /// `TerrainKind` discriminants, classified once: terrain is derived from the elevation and the
    /// latitude, neither of which a turn changes, so it never belongs in a delta.
    terrain: Vec<u8>,
}

#[wasm_bindgen]
impl GameSession {
    /// Start a new game from `NewGameSettings` as JSON.
    ///
    /// Generating the planet happens here rather than in `core`: `procgen` depends on `core`, not
    /// the other way around.
    #[wasm_bindgen(js_name = newGame)]
    pub fn new_game(settings_json: &str) -> Result<GameSession, JsError> {
        let settings: NewGameSettings = serde_json::from_str(settings_json)
            .map_err(|error| JsError::new(&format!("invalid settings: {error}")))?;
        // `MapGeneratorSettings` panics on invalid settings, which is unrecoverable in WASM
        if settings.seed > MAX_SEED {
            return Err(JsError::new(&format!(
                "the seed must be at most {MAX_SEED}"
            )));
        }
        if settings.columns <= 0 || settings.rows <= 0 {
            return Err(JsError::new(
                "the map must have at least one column and one row",
            ));
        }
        if settings.nations == 0 {
            return Err(JsError::new("a game needs at least one nation"));
        }
        if !(0.0..=1.0).contains(&settings.ocean_coverage) {
            return Err(JsError::new("the ocean coverage must be from 0 to 1"));
        }

        // the generator takes its settings, which the classification needs in turn
        let generator_settings = || {
            MapGeneratorSettings::new(settings.seed, settings.columns, settings.rows)
                .with_ocean_coverage(settings.ocean_coverage)
        };
        let map = MapGenerator::from_settings(generator_settings()).generate(|_| {});
        let generator_settings = generator_settings();
        let shape = RectangularHexMapShape::new(settings.columns, settings.rows);

        let surface = PlanetSurface::classify(&map, &generator_settings);
        let terrain: Vec<u8> = surface.terrain().iter().map(|&kind| kind as u8).collect();
        let nations = NationsGenerator::from_settings(NationsGeneratorSettings::new(
            u64::from(settings.seed),
            settings.nations,
        ))
        .generate(&surface);

        let mut world = World::new(shape, map);
        // the simulation reads the ground for its rules; the client gets the same kinds from `terrain`
        world.paint_terrain(surface.terrain());
        set_up_scaffold_world(&mut world, settings.nations, &nations);

        let local_player = NationId::from_index(0);
        let mut inner = CoreSession::new(world, settings.seed as u64, local_player);
        for index in 1..settings.nations as usize {
            let nation = NationId::from_index(index);
            inner.set_controller(nation, stub_ai_for(inner.seed(), nation));
        }

        Ok(Self { inner, terrain })
    }

    /// The nation this client plays.
    #[wasm_bindgen(getter, js_name = localPlayer)]
    pub fn local_player(&self) -> u16 {
        self.inner.local_player().raw()
    }

    #[wasm_bindgen(getter)]
    pub fn turn(&self) -> u32 {
        self.inner.turn()
    }

    /// The world's current version, which the client passes back to `project`.
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> f64 {
        self.inner.version() as f64
    }

    /// Send an order. `Ok(None)` means it was taken, `Ok(Some(json))` that it was refused
    /// (a `RejectionWire`), and `Err` that the JSON itself made no sense.
    ///
    /// The distinction matters: a refusal is gameplay and belongs in the UI, a parse error is a bug
    /// in the client.
    pub fn submit(&mut self, command_json: &str) -> Result<Option<String>, JsError> {
        let command: CommandWire = serde_json::from_str(command_json)
            .map_err(|error| JsError::new(&format!("invalid command: {error}")))?;
        match self.inner.submit(command.into()) {
            Ok(()) => Ok(None),
            Err(rejection) => {
                let wire = RejectionWire::from(&rejection);
                Ok(Some(serde_json::to_string(&wire).map_err(to_js_error)?))
            }
        }
    }

    /// Start resolving a turn. Does nothing if one is already being resolved.
    #[wasm_bindgen(js_name = beginTurn)]
    pub fn begin_turn(&mut self) {
        self.inner.begin_turn();
    }

    /// Do one bounded slice of turn resolution, and report where it got to.
    pub fn step(&mut self) -> TurnStepView {
        TurnStepView::from(self.inner.step(StepBudget::default()))
    }

    /// Everything buffered since the last drain, as a JSON array of `EventWire`.
    #[wasm_bindgen(js_name = drainEvents)]
    pub fn drain_events(&mut self) -> Result<String, JsError> {
        let events: Vec<EventWire> = self
            .inner
            .drain_events()
            .iter()
            .map(EventWire::from)
            .collect();
        serde_json::to_string(&events).map_err(to_js_error)
    }

    /// Where `viewer`'s `unit` can get to this turn, by the rules an order is judged by, or
    /// `undefined` for a unit the viewer does not own. See `simulation::movement::reachable`.
    pub fn reachable(&self, viewer: u16, unit: u32) -> Option<ReachableView> {
        movement::reachable(
            self.inner.world(),
            NationId::from_index(viewer as usize),
            UnitId::from_index(unit as usize),
        )
        .map(|inner| ReachableView { inner })
    }

    /// Everything the viewer can see, from scratch.
    pub fn snapshot(&self, viewer: u16) -> WorldSnapshotView {
        WorldSnapshotView::build(
            self.inner.snapshot(NationId::from_index(viewer as usize)),
            &self.terrain,
        )
    }

    /// What the viewer has to be sent to catch up with `since`.
    ///
    /// Whether that is a delta or a whole new snapshot is `core`'s call, not the client's.
    pub fn project(&self, viewer: u16, since: f64) -> WorldProjectionView {
        let viewer = NationId::from_index(viewer as usize);
        match self.inner.project(viewer, since.max(0.0) as u64) {
            Projection::Delta(delta) => WorldProjectionView {
                full: None,
                delta: Some(WorldDeltaView::build(delta, &self.terrain)),
            },
            Projection::Full(snapshot) => WorldProjectionView {
                full: Some(WorldSnapshotView::build(snapshot, &self.terrain)),
                delta: None,
            },
        }
    }
}

/// Where a `step` got to.
#[wasm_bindgen]
pub struct TurnStepView {
    phase: String,
    percent: u8,
    done: bool,
    idle: bool,
    turn: u32,
    version: f64,
}

#[wasm_bindgen]
impl TurnStepView {
    /// `TurnPhase::name`, or `"idle"` / `"done"`.
    #[wasm_bindgen(getter)]
    pub fn phase(&self) -> String {
        self.phase.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn percent(&self) -> u8 {
        self.percent
    }

    /// Whether the turn just finished: the client fetches its projection now.
    #[wasm_bindgen(getter)]
    pub fn done(&self) -> bool {
        self.done
    }

    /// Whether no turn was being resolved at all.
    #[wasm_bindgen(getter)]
    pub fn idle(&self) -> bool {
        self.idle
    }

    #[wasm_bindgen(getter)]
    pub fn turn(&self) -> u32 {
        self.turn
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> f64 {
        self.version
    }
}

impl From<TurnProgress> for TurnStepView {
    fn from(value: TurnProgress) -> Self {
        match value {
            TurnProgress::Idle => Self {
                phase: "idle".to_string(),
                percent: 0,
                done: false,
                idle: true,
                turn: 0,
                version: 0.0,
            },
            TurnProgress::Working { phase, percent } => Self {
                phase: phase.name().to_string(),
                percent,
                done: false,
                idle: false,
                turn: 0,
                version: 0.0,
            },
            TurnProgress::Done { turn, version } => Self {
                phase: "done".to_string(),
                percent: 100,
                done: true,
                idle: false,
                turn,
                version: version as f64,
            },
        }
    }
}

/// A full projection. Every getter copies into a typed array, so read each one once.
#[wasm_bindgen]
pub struct WorldSnapshotView {
    meta: String,
    columns: i16,
    rows: i16,
    terrain: Vec<u8>,
    inner: WorldSnapshot,
}

#[wasm_bindgen]
impl WorldSnapshotView {
    /// `MetaWire` as JSON: turn, version, nations, stockpiles.
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> String {
        self.meta.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn columns(&self) -> i16 {
        self.columns
    }

    #[wasm_bindgen(getter)]
    pub fn rows(&self) -> i16 {
        self.rows
    }

    /// `TerrainKind` discriminants, for the whole map.
    #[wasm_bindgen(getter)]
    pub fn terrain(&self) -> Vec<u8> {
        self.terrain.clone()
    }

    /// In meters, zeroed on tiles the viewer has never explored.
    #[wasm_bindgen(getter)]
    pub fn elevations(&self) -> Vec<i16> {
        self.inner.elevations.clone()
    }

    /// `TileVisibility` discriminants.
    #[wasm_bindgen(getter)]
    pub fn visibility(&self) -> Vec<u8> {
        self.inner.visibility.clone()
    }

    /// Owning nation per tile, `0xFFFF` where none or unknown.
    #[wasm_bindgen(getter, js_name = tileOwner)]
    pub fn tile_owner(&self) -> Vec<u16> {
        self.inner.tile_owner.clone()
    }

    /// Nation whose sphere of influence holds each tile, `0xFFFF` where none or unknown: the
    /// viewer's own everywhere, a rival's only once the tile is explored.
    #[wasm_bindgen(getter, js_name = tileSphere)]
    pub fn tile_sphere(&self) -> Vec<u16> {
        self.inner.tile_sphere.clone()
    }

    #[wasm_bindgen(getter, js_name = tileFlags)]
    pub fn tile_flags(&self) -> Vec<u8> {
        self.inner.tile_flags.clone()
    }

    #[wasm_bindgen(getter, js_name = unitIds)]
    pub fn unit_ids(&self) -> Vec<u32> {
        self.inner.unit_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = unitTiles)]
    pub fn unit_tiles(&self) -> Vec<u32> {
        self.inner.unit_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = unitOwners)]
    pub fn unit_owners(&self) -> Vec<u16> {
        self.inner.unit_owners.clone()
    }

    #[wasm_bindgen(getter, js_name = unitHealth)]
    pub fn unit_health(&self) -> Vec<u16> {
        self.inner.unit_health.clone()
    }

    /// Indices into the meta's `unitTemplates`.
    #[wasm_bindgen(getter, js_name = unitKinds)]
    pub fn unit_kinds(&self) -> Vec<u8> {
        self.inner.unit_kinds.clone()
    }

    #[wasm_bindgen(getter, js_name = unitMovementLeft)]
    pub fn unit_movement_left(&self) -> Vec<u8> {
        self.inner.unit_movement_left.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementIds)]
    pub fn settlement_ids(&self) -> Vec<u32> {
        self.inner.settlement_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementTiles)]
    pub fn settlement_tiles(&self) -> Vec<u32> {
        self.inner.settlement_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementOwners)]
    pub fn settlement_owners(&self) -> Vec<u16> {
        self.inner.settlement_owners.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementStages)]
    pub fn settlement_stages(&self) -> Vec<u8> {
        self.inner.settlement_stages.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingIds)]
    pub fn building_ids(&self) -> Vec<u32> {
        self.inner.building_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingTiles)]
    pub fn building_tiles(&self) -> Vec<u32> {
        self.inner.building_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingOwners)]
    pub fn building_owners(&self) -> Vec<u16> {
        self.inner.building_owners.clone()
    }

    /// Indices into the meta's `buildingTypes`.
    #[wasm_bindgen(getter, js_name = buildingKinds)]
    pub fn building_kinds(&self) -> Vec<u8> {
        self.inner.building_kinds.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingLevels)]
    pub fn building_levels(&self) -> Vec<u8> {
        self.inner.building_levels.clone()
    }
}

impl WorldSnapshotView {
    fn build(snapshot: WorldSnapshot, terrain: &[u8]) -> Self {
        let visible_terrain = snapshot
            .visibility
            .iter()
            .zip(terrain.iter())
            .map(|(&visibility, &kind)| hidden_unless_explored(visibility, kind))
            .collect();
        Self {
            meta: meta_json(&MetaWire::from(&snapshot.meta)),
            columns: snapshot.columns,
            rows: snapshot.rows,
            terrain: visible_terrain,
            inner: snapshot,
        }
    }
}

/// The tiles a unit can end its turn on: three parallel arrays, see `movement::Reachable`.
#[wasm_bindgen]
pub struct ReachableView {
    inner: Reachable,
}

#[wasm_bindgen]
impl ReachableView {
    /// Row-major indices, ascending, the unit's own tile included.
    #[wasm_bindgen(getter)]
    pub fn tiles(&self) -> Vec<u32> {
        self.inner.tiles.clone()
    }

    /// Points spent to reach each tile.
    #[wasm_bindgen(getter)]
    pub fn costs(&self) -> Vec<u8> {
        self.inner.costs.clone()
    }

    /// The tile before each one on its cheapest path; the origin names itself.
    #[wasm_bindgen(getter)]
    pub fn parents(&self) -> Vec<u32> {
        self.inner.parents.clone()
    }
}

/// A catch-up. The tile arrays are parallel to `tileIndices`, not to the map.
#[wasm_bindgen]
pub struct WorldDeltaView {
    meta: String,
    terrain: Vec<u8>,
    inner: WorldDelta,
}

#[wasm_bindgen]
impl WorldDeltaView {
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> String {
        self.meta.clone()
    }

    /// Row-major indices of the tiles that changed.
    #[wasm_bindgen(getter, js_name = tileIndices)]
    pub fn tile_indices(&self) -> Vec<u32> {
        self.inner.tile_indices.clone()
    }

    /// `TerrainKind` discriminants of those tiles.
    ///
    /// Terrain never changes, but what the viewer is allowed to know of it does: a tile explored
    /// this turn is where its terrain stops being `UNKNOWN_TERRAIN`.
    #[wasm_bindgen(getter)]
    pub fn terrain(&self) -> Vec<u8> {
        self.terrain.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn elevations(&self) -> Vec<i16> {
        self.inner.elevations.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn visibility(&self) -> Vec<u8> {
        self.inner.visibility.clone()
    }

    #[wasm_bindgen(getter, js_name = tileOwner)]
    pub fn tile_owner(&self) -> Vec<u16> {
        self.inner.tile_owner.clone()
    }

    #[wasm_bindgen(getter, js_name = tileSphere)]
    pub fn tile_sphere(&self) -> Vec<u16> {
        self.inner.tile_sphere.clone()
    }

    #[wasm_bindgen(getter, js_name = tileFlags)]
    pub fn tile_flags(&self) -> Vec<u8> {
        self.inner.tile_flags.clone()
    }

    #[wasm_bindgen(getter, js_name = unitIds)]
    pub fn unit_ids(&self) -> Vec<u32> {
        self.inner.unit_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = unitTiles)]
    pub fn unit_tiles(&self) -> Vec<u32> {
        self.inner.unit_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = unitOwners)]
    pub fn unit_owners(&self) -> Vec<u16> {
        self.inner.unit_owners.clone()
    }

    #[wasm_bindgen(getter, js_name = unitHealth)]
    pub fn unit_health(&self) -> Vec<u16> {
        self.inner.unit_health.clone()
    }

    /// Indices into the meta's `unitTemplates`.
    #[wasm_bindgen(getter, js_name = unitKinds)]
    pub fn unit_kinds(&self) -> Vec<u8> {
        self.inner.unit_kinds.clone()
    }

    #[wasm_bindgen(getter, js_name = unitMovementLeft)]
    pub fn unit_movement_left(&self) -> Vec<u8> {
        self.inner.unit_movement_left.clone()
    }

    /// Units the client must drop: destroyed, or simply no longer in sight.
    #[wasm_bindgen(getter, js_name = unitsRemoved)]
    pub fn units_removed(&self) -> Vec<u32> {
        self.inner.units_removed.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementIds)]
    pub fn settlement_ids(&self) -> Vec<u32> {
        self.inner.settlement_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementTiles)]
    pub fn settlement_tiles(&self) -> Vec<u32> {
        self.inner.settlement_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementOwners)]
    pub fn settlement_owners(&self) -> Vec<u16> {
        self.inner.settlement_owners.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementStages)]
    pub fn settlement_stages(&self) -> Vec<u8> {
        self.inner.settlement_stages.clone()
    }

    #[wasm_bindgen(getter, js_name = settlementsRemoved)]
    pub fn settlements_removed(&self) -> Vec<u32> {
        self.inner.settlements_removed.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingIds)]
    pub fn building_ids(&self) -> Vec<u32> {
        self.inner.building_ids.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingTiles)]
    pub fn building_tiles(&self) -> Vec<u32> {
        self.inner.building_tiles.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingOwners)]
    pub fn building_owners(&self) -> Vec<u16> {
        self.inner.building_owners.clone()
    }

    /// Indices into the meta's `buildingTypes`.
    #[wasm_bindgen(getter, js_name = buildingKinds)]
    pub fn building_kinds(&self) -> Vec<u8> {
        self.inner.building_kinds.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingLevels)]
    pub fn building_levels(&self) -> Vec<u8> {
        self.inner.building_levels.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingsRemoved)]
    pub fn buildings_removed(&self) -> Vec<u32> {
        self.inner.buildings_removed.clone()
    }
}

impl WorldDeltaView {
    fn build(delta: WorldDelta, terrain: &[u8]) -> Self {
        let changed_terrain = delta
            .tile_indices
            .iter()
            .zip(delta.visibility.iter())
            .map(|(&index, &visibility)| {
                let kind = terrain
                    .get(index as usize)
                    .copied()
                    .unwrap_or(UNKNOWN_TERRAIN);
                hidden_unless_explored(visibility, kind)
            })
            .collect();
        Self {
            meta: meta_json(&MetaWire::from(&delta.meta)),
            terrain: changed_terrain,
            inner: delta,
        }
    }
}

/// Either a catch-up or a whole new snapshot, whichever `core` decided on.
#[wasm_bindgen]
pub struct WorldProjectionView {
    full: Option<WorldSnapshotView>,
    delta: Option<WorldDeltaView>,
}

#[wasm_bindgen]
impl WorldProjectionView {
    /// Whether the client has to replace everything it holds.
    #[wasm_bindgen(getter, js_name = isFull)]
    pub fn is_full(&self) -> bool {
        self.full.is_some()
    }

    /// Take the snapshot out, if this is one. Returns `undefined` the second time.
    #[wasm_bindgen(js_name = takeFull)]
    pub fn take_full(&mut self) -> Option<WorldSnapshotView> {
        self.full.take()
    }

    /// Take the delta out, if this is one. Returns `undefined` the second time.
    #[wasm_bindgen(js_name = takeDelta)]
    pub fn take_delta(&mut self) -> Option<WorldDeltaView> {
        self.delta.take()
    }
}

/// Nations, a capital each, and one division of each kind around it.
///
/// TODO: this is the scaffold standing in for a scenario. Nations, their rosters and their starting
/// position all belong to scripts, and this function is what they will replace.
fn set_up_scaffold_world(world: &mut World, nations: u16, generated: &GeneratedNations) {
    for (name, movement, sight) in UNIT_TEMPLATES {
        world.register_unit_template(UnitTemplate::new(
            SimulationID::new_abstract_id(name),
            name.to_string(),
            movement,
            sight,
        ));
    }
    for (name, produces, per_turn) in BUILDING_TEMPLATES {
        // priced so that a starting stockpile affords a couple, and the economy phase has something to add
        world.register_building_template(BuildingTemplate::new(
            SimulationID::new_abstract_id(name),
            name.to_string(),
            BTreeMap::from([(Resource::Credits, 60), (Resource::Metals, 15)]),
            BTreeMap::from([(Resource::Credits, 2)]),
            BTreeMap::from([(produces, per_turn)]),
        ));
    }

    // where each nation was meant to grow, kept for the political map and the AI
    world.set_genesis(WorldGenesis::new(
        generated.spheres().owners().to_vec(),
        generated
            .nations()
            .iter()
            .map(|nation| Homeland::new(nation.id(), nation.capital(), nation.site_score()))
            .collect(),
    ));

    // the setup writes through the mutation gate like everything else, into changes nobody will
    // read: nothing has a version to catch up from yet
    let mut changes = TurnChanges::new();
    for index in 0..nations as usize {
        let mut nation = Nation::new(
            SimulationID::new_abstract_id(&format!("nation_{index}")),
            format!("Nation {}", index + 1),
        );
        // something for the economy phase to tick and the toolbar to show, until a scenario says
        // what a nation actually starts with
        let stockpile = nation.resources_mut();
        for (resource, quantity, income) in [
            (Resource::Credits, 250, 12),
            (Resource::Water, 120, 4),
            (Resource::Food, 180, 6),
            (Resource::Electricity, 60, 3),
            (Resource::Metals, 90, 2),
        ] {
            stockpile.replenish(resource, quantity);
            stockpile.set_income(resource, income);
        }
        let nation = world.register_nation(nation);

        // the generator places the nations in this very order; one it found no room for starts
        // with nothing, like a nation on a planet with no land at all
        let Some(start) = generated.nations().get(index) else {
            continue;
        };
        debug_assert_eq!(start.id(), nation);
        let mut gate = WorldMut::new(world, &mut changes);
        gate.insert_settlement(Settlement::found(
            format!("Nation {} capital", index + 1),
            nation,
            start.capital(),
        ));
        for &tile in start.territory() {
            if let Some(coords) = gate.world().tile_coords(tile)
                && let Some(tile) = gate.tile_mut(coords)
            {
                tile.artificial_mut().set_owner(Some(nation));
            }
        }
        // one unit per tile: the capital, then its land neighbours, nearest first
        let mut sites = vec![start.capital()];
        sites.extend(world.shape().neighbors(start.capital()).filter(|&coords| {
            world
                .tile(coords)
                .is_some_and(|tile| !tile.natural().terrain().is_water())
        }));
        for ((name, _, _), site) in UNIT_TEMPLATES.into_iter().zip(sites) {
            world.place_unit(Unit::recruit(
                SimulationID::new_abstract_id(name),
                nation,
                site,
            ));
        }
    }
}

/// The terrain of a tile, or `UNKNOWN_TERRAIN` while the viewer has never explored it.
fn hidden_unless_explored(visibility: u8, kind: u8) -> u8 {
    if visibility == TileVisibility::Unexplored as u8 {
        UNKNOWN_TERRAIN
    } else {
        kind
    }
}

fn meta_json(meta: &MetaWire) -> String {
    // a `MetaWire` has no map and no non-string keys: it cannot fail to serialize
    serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string())
}

fn to_js_error(error: serde_json::Error) -> JsError {
    JsError::new(&error.to_string())
}

#[cfg(test)]
mod tests {
    use project_x_core::simulation::fog::TileVisibility;

    use super::{GameSession, UNKNOWN_TERRAIN};

    /// A map small enough to assert over, big enough to have land and unexplored corners.
    fn start_game() -> GameSession {
        GameSession::new_game(r#"{"seed":7,"columns":24,"rows":16,"nations":2}"#)
            .expect("the settings are valid")
    }

    fn resolve_turn(session: &mut GameSession) {
        session.begin_turn();
        for _ in 0..256 {
            if session.step().done() {
                return;
            }
        }
        panic!("the turn never finished");
    }

    #[test]
    fn test_a_new_game_remembers_its_genesis_and_shows_the_viewer_their_sphere() {
        let session = start_game();
        let world = session.inner.world();
        let genesis = world.genesis();
        assert_eq!(genesis.tiles_count(), world.tiles_count());
        assert_eq!(genesis.homelands().len(), 2);
        for (id, nation) in world.nations().iter() {
            let homeland = genesis.homeland(id).expect("a homeland");
            let capital = world
                .settlements()
                .get(nation.capital().expect("a capital"))
                .expect("the capital exists");
            assert_eq!(homeland.capital(), capital.position());
        }

        let snapshot = session.snapshot(0);
        let sphere = snapshot.tile_sphere();
        assert_eq!(sphere.len(), world.tiles_count());
        let own = sphere.iter().filter(|&&nation| nation == 0).count();
        assert!(own > 0, "the viewer's own sphere is shown");
        // nothing of the rival's sphere on a tile never explored
        for (index, &nation) in sphere.iter().enumerate() {
            if nation == 1 {
                assert_ne!(
                    snapshot.visibility()[index],
                    TileVisibility::Unexplored as u8,
                    "tile {index}"
                );
            }
        }
    }

    #[test]
    fn test_a_new_game_takes_an_ocean_coverage() {
        let session = GameSession::new_game(
            r#"{"seed":7,"columns":24,"rows":16,"nations":2,"oceanCoverage":0.5}"#,
        )
        .expect("the settings are valid");
        let world = session.inner.world();
        let land = world
            .shape()
            .coords()
            .filter(|&coords| world.tile(coords).unwrap().elevation() >= 0)
            .count();
        assert!(
            (land as f64 / world.tiles_count() as f64 - 0.5).abs() < 0.02,
            "{land} land tiles"
        );
    }

    #[test]
    fn test_a_new_game_starts_each_nation_with_a_capital_on_its_own_land() {
        let session = start_game();
        let world = session.inner.world();
        assert_eq!(world.nations().len(), 2);
        assert_eq!(world.settlements().len(), 2, "one capital each");
        for (id, nation) in world.nations().iter() {
            let capital = nation.capital().expect("a capital");
            let settlement = world
                .settlements()
                .get(capital)
                .expect("the capital exists");
            assert_eq!(settlement.owner(), id);
            let tile = world.tile(settlement.position()).expect("on the map");
            assert_eq!(tile.artificial().settlement(), Some(capital));
            assert_eq!(tile.artificial().owner(), Some(id));
            // the roster: one of each division, one in the capital, none sharing a tile, all on land
            let positions: Vec<_> = world
                .units()
                .iter()
                .filter(|(_, unit)| unit.owner() == id)
                .map(|(_, unit)| unit.position())
                .collect();
            assert_eq!(positions.len(), 3, "three divisions");
            assert!(
                positions.contains(&settlement.position()),
                "one stands in the capital"
            );
            for (index, position) in positions.iter().enumerate() {
                assert!(!positions[..index].contains(position), "one unit per tile");
                let tile = world.tile(*position).expect("on the map");
                assert!(!tile.natural().terrain().is_water(), "a division on land");
            }
        }
        // the territory around the capitals is owned, the open sea is not
        let owned = world
            .shape()
            .coords()
            .filter(|&coords| world.tile(coords).unwrap().artificial().owner().is_some())
            .count();
        assert!(owned > 2, "{owned} owned tiles");
        assert!(owned < world.tiles_count() / 2, "{owned} owned tiles");
    }

    #[test]
    fn test_a_new_games_snapshot_names_the_units_and_their_points() {
        let session = start_game();
        let snapshot = session.snapshot(0);
        let meta: serde_json::Value = serde_json::from_str(&snapshot.meta()).unwrap();
        let templates = meta["unitTemplates"]
            .as_array()
            .expect("unit templates in the meta");
        let types: Vec<&str> = templates
            .iter()
            .map(|t| t["type"].as_str().unwrap())
            .collect();
        assert_eq!(
            types,
            [
                "infantry_division",
                "light_tank_division",
                "motorized_division"
            ]
        );
        // its own three, each a different kind, each with a fresh turn's points
        let kinds = snapshot.unit_kinds();
        let left = snapshot.unit_movement_left();
        assert_eq!(kinds.len(), 3);
        let mut sorted = kinds.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2]);
        for (kind, points) in kinds.iter().zip(left) {
            assert_eq!(
                u64::from(points),
                templates[*kind as usize]["movement"].as_u64().unwrap()
            );
        }
    }

    #[test]
    fn test_reachability_is_answered_for_the_viewers_own_units_only() {
        let session = start_game();
        let snapshot = session.snapshot(0);
        for (unit, tile) in snapshot.unit_ids().into_iter().zip(snapshot.unit_tiles()) {
            let reach = session.reachable(0, unit).expect("the viewer's own unit");
            assert!(reach.tiles().contains(&tile), "its own tile is in");
            assert_eq!(reach.tiles().len(), reach.costs().len());
            assert_eq!(reach.tiles().len(), reach.parents().len());
            // and the other nation gets nothing about it
            assert!(session.reachable(1, unit).is_none());
        }
    }

    #[test]
    fn test_the_world_knows_the_terrain_of_its_tiles() {
        let session = start_game();
        let world = session.inner.world();
        // every tile carries the kind the client is shown, so the rules and the map agree
        for (index, coords) in world.shape().coords().enumerate() {
            let kind = world.tile(coords).expect("on the map").natural().terrain();
            assert_eq!(kind as u8, session.terrain[index], "tile {index}");
        }
        assert!(
            world.shape().coords().any(|coords| world
                .tile(coords)
                .unwrap()
                .natural()
                .terrain()
                .is_water()),
            "a planet with 70% ocean has water tiles"
        );
    }

    #[test]
    fn test_a_snapshot_hides_the_terrain_of_what_was_never_explored() {
        let session = start_game();
        let snapshot = session.snapshot(0);
        let (terrain, visibility) = (snapshot.terrain(), snapshot.visibility());
        assert_eq!(terrain.len(), visibility.len());

        // the scout sees its surroundings, and nothing else of the planet has travelled
        assert!(terrain.iter().any(|&kind| kind != UNKNOWN_TERRAIN));
        assert!(terrain.iter().any(|&kind| kind == UNKNOWN_TERRAIN));
        for (index, (&kind, &seen)) in terrain.iter().zip(visibility.iter()).enumerate() {
            assert_eq!(kind == UNKNOWN_TERRAIN, seen == 0, "tile {index}");
        }
        // and neither has its elevation
        for (index, (&elevation, &seen)) in snapshot
            .elevations()
            .iter()
            .zip(visibility.iter())
            .enumerate()
        {
            if seen == 0 {
                assert_eq!(elevation, 0, "tile {index}");
            }
        }
    }

    #[test]
    fn test_a_resolved_turn_is_caught_up_with_a_delta() {
        let mut session = start_game();
        let since = session.version();
        resolve_turn(&mut session);
        assert!(session.version() > since);

        let mut projection = session.project(0, since);
        assert!(
            !projection.is_full(),
            "one turn must not cost a full snapshot"
        );
        let delta = projection.take_delta().expect("a delta");

        // the tile arrays are parallel to the indices, and sorted so the renderer can paint a row once
        let indices = delta.tile_indices();
        assert_eq!(delta.terrain().len(), indices.len());
        assert_eq!(delta.visibility().len(), indices.len());
        assert_eq!(delta.elevations().len(), indices.len());
        assert!(indices.windows(2).all(|pair| pair[0] < pair[1]));

        for (index, (&kind, &seen)) in delta
            .terrain()
            .iter()
            .zip(delta.visibility().iter())
            .enumerate()
        {
            assert_eq!(kind == UNKNOWN_TERRAIN, seen == 0, "entry {index}");
        }
    }

    #[test]
    fn test_a_viewer_is_only_shown_its_own_stockpiles() {
        let session = start_game();
        let meta = session.snapshot(0).meta();
        // the viewer's nation carries its resources, the other one's are not public knowledge
        assert!(meta.contains(r#""id":0"#) && meta.contains(r#""resource":"credits""#));
        assert!(
            meta.contains(r#""id":1,"name":"Nation 2","resources":[]"#),
            "{meta}"
        );
    }
}
