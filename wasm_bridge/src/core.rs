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

use wasm_bindgen::prelude::{JsError, wasm_bindgen};

use project_x_core::{
    game::{
        ai::{ControllerSlot, stub_ai_for},
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
        resources::{ResourceDataStorage, ResourceDataStore, ResourceDeposit},
        settlements::Settlement,
    },
};
use project_x_procgen::{
    deposits::{DepositKind, DepositsGenerator, DepositsGeneratorSettings, GeneratedDeposits},
    forests::Forests,
    map::{MAX_SEED, MapGenerator, MapGeneratorSettings},
    nations::{GeneratedNations, NationsGenerator, NationsGeneratorSettings, PlanetSurface},
};
use project_x_scripting::{NationSetup, ScriptEngine, ScriptSource};

use crate::wire::{CommandWire, EventWire, MetaWire, NewGameSettings, RejectionWire};

/// Terrain of a tile the viewer has never explored.
///
/// Terrain is classified here rather than in `core`, so it is also filtered here: handing the
/// client the terrain of the whole map and covering it with fog on the other side would leak the
/// shape of the planet to anyone who opens the devtools.
const UNKNOWN_TERRAIN: u8 = 255;

/// Vegetation of a tile the viewer has never explored, for the same reason as `UNKNOWN_TERRAIN`.
const UNKNOWN_VEGETATION: u8 = 255;

/// What a planet's ground is, classified once when the game starts: neither the terrain nor the
/// vegetation changes during a game, so neither belongs in a delta - only what the viewer may know
/// of them does.
struct Ground {
    /// `TerrainKind` discriminants, row-major.
    terrain: Vec<u8>,
    /// `Vegetation` discriminants, row-major.
    vegetation: Vec<u8>,
}

impl Ground {
    /// What the viewer may know of the tile's terrain and vegetation, given its visibility.
    fn seen(&self, index: usize, visibility: u8) -> (u8, u8) {
        if visibility == TileVisibility::Unexplored as u8 {
            return (UNKNOWN_TERRAIN, UNKNOWN_VEGETATION);
        }
        (
            self.terrain.get(index).copied().unwrap_or(UNKNOWN_TERRAIN),
            self.vegetation
                .get(index)
                .copied()
                .unwrap_or(UNKNOWN_VEGETATION),
        )
    }
}

/// One playthrough, driven from the client's worker.
#[wasm_bindgen]
pub struct GameSession {
    inner: CoreSession,
    ground: Ground,
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
        // scripts that do not load are a bug in the scripts, and no reason to generate a planet
        let sources: Vec<ScriptSource> = settings
            .scripts
            .iter()
            .cloned()
            .map(ScriptSource::from)
            .collect();
        let scripts =
            ScriptEngine::load(&sources).map_err(|error| JsError::new(&error.to_string()))?;
        let content = StartingContent::from_scripts(&scripts, settings.nations)
            .map_err(|error| JsError::new(&error))?;

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
        let vegetation = Forests::grow(&surface).vegetation();
        let nations = NationsGenerator::from_settings(NationsGeneratorSettings::new(
            u64::from(settings.seed),
            settings.nations,
        ))
        .generate(&surface);

        let deposit_settings = DepositsGeneratorSettings::new(u64::from(settings.seed));
        let deposits = DepositsGenerator::from_settings(deposit_settings.clone())
            .generate(&surface, Some(&nations));

        let mut world = World::new(shape, map);
        // the simulation reads the ground for its rules; the client gets the same kinds from `terrain`
        world.paint_terrain(surface.terrain());
        world.paint_vegetation(&vegetation);
        lay_deposits(&mut world, &deposits, &deposit_settings);
        set_up_world(&mut world, &nations, content);

        let local_player = NationId::from_index(0);
        let mut inner = CoreSession::new(world, settings.seed as u64, local_player);
        for index in 1..settings.nations as usize {
            let nation = NationId::from_index(index);
            let controller = match scripts.controller_for(nation, inner.seed()) {
                Some(scripted) => ControllerSlot::Ai(Box::new(scripted)),
                None => stub_ai_for(inner.seed(), nation),
            };
            inner.set_controller(nation, controller);
        }

        Ok(Self {
            inner,
            ground: Ground {
                terrain,
                vegetation: vegetation.iter().map(|&grown| grown as u8).collect(),
            },
        })
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
            &self.ground,
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
                delta: Some(WorldDeltaView::build(delta, &self.ground)),
            },
            Projection::Full(snapshot) => WorldProjectionView {
                full: Some(WorldSnapshotView::build(snapshot, &self.ground)),
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
    vegetation: Vec<u8>,
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

    /// `Vegetation` discriminants, for the whole map, `UNKNOWN_VEGETATION` where never explored.
    #[wasm_bindgen(getter)]
    pub fn vegetation(&self) -> Vec<u8> {
        self.vegetation.clone()
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

    /// Indices into the meta's `buildingTemplates`.
    #[wasm_bindgen(getter, js_name = buildingKinds)]
    pub fn building_kinds(&self) -> Vec<u8> {
        self.inner.building_kinds.clone()
    }

    /// 0 while a building is first being built.
    #[wasm_bindgen(getter, js_name = buildingLevels)]
    pub fn building_levels(&self) -> Vec<u8> {
        self.inner.building_levels.clone()
    }

    /// `projection::building_states` bits; a rival's halted bit is never set.
    #[wasm_bindgen(getter, js_name = buildingStates)]
    pub fn building_states(&self) -> Vec<u8> {
        self.inner.building_states.clone()
    }

    /// Zero without works under way, and for a rival's building.
    #[wasm_bindgen(getter, js_name = buildingTurnsLeft)]
    pub fn building_turns_left(&self) -> Vec<u16> {
        self.inner.building_turns_left.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingTurnsTotal)]
    pub fn building_turns_total(&self) -> Vec<u16> {
        self.inner.building_turns_total.clone()
    }

    #[wasm_bindgen(getter, js_name = depositIds)]
    pub fn deposit_ids(&self) -> Vec<u32> {
        self.inner.deposits.ids.clone()
    }

    #[wasm_bindgen(getter, js_name = depositTiles)]
    pub fn deposit_tiles(&self) -> Vec<u32> {
        self.inner.deposits.tiles.clone()
    }

    /// `Resource` discriminants, in the order of the client's `RESOURCES`.
    #[wasm_bindgen(getter, js_name = depositResources)]
    pub fn deposit_resources(&self) -> Vec<u8> {
        self.inner.deposits.resources.clone()
    }

    /// 1 to 3, 0 once exhausted.
    #[wasm_bindgen(getter, js_name = depositLevels)]
    pub fn deposit_levels(&self) -> Vec<u8> {
        self.inner.deposits.levels.clone()
    }

    #[wasm_bindgen(getter, js_name = depositQuantities)]
    pub fn deposit_quantities(&self) -> Vec<u16> {
        self.inner.deposits.quantities.clone()
    }

    #[wasm_bindgen(getter, js_name = depositRecharges)]
    pub fn deposit_recharges(&self) -> Vec<u16> {
        self.inner.deposits.recharges.clone()
    }

    /// `0xFFFF` while nothing draws on the deposit.
    #[wasm_bindgen(getter, js_name = depositTurnsLeft)]
    pub fn deposit_turns_left(&self) -> Vec<u16> {
        self.inner.deposits.turns_left.clone()
    }
}

impl WorldSnapshotView {
    fn build(snapshot: WorldSnapshot, ground: &Ground) -> Self {
        let (terrain, vegetation) = snapshot
            .visibility
            .iter()
            .enumerate()
            .map(|(index, &visibility)| ground.seen(index, visibility))
            .unzip();
        Self {
            meta: meta_json(&MetaWire::from(&snapshot.meta)),
            columns: snapshot.columns,
            rows: snapshot.rows,
            terrain,
            vegetation,
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
    vegetation: Vec<u8>,
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

    /// `Vegetation` discriminants of those tiles, like `terrain`.
    #[wasm_bindgen(getter)]
    pub fn vegetation(&self) -> Vec<u8> {
        self.vegetation.clone()
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

    /// Indices into the meta's `buildingTemplates`.
    #[wasm_bindgen(getter, js_name = buildingKinds)]
    pub fn building_kinds(&self) -> Vec<u8> {
        self.inner.building_kinds.clone()
    }

    /// 0 while a building is first being built.
    #[wasm_bindgen(getter, js_name = buildingLevels)]
    pub fn building_levels(&self) -> Vec<u8> {
        self.inner.building_levels.clone()
    }

    /// `projection::building_states` bits; a rival's halted bit is never set.
    #[wasm_bindgen(getter, js_name = buildingStates)]
    pub fn building_states(&self) -> Vec<u8> {
        self.inner.building_states.clone()
    }

    /// Zero without works under way, and for a rival's building.
    #[wasm_bindgen(getter, js_name = buildingTurnsLeft)]
    pub fn building_turns_left(&self) -> Vec<u16> {
        self.inner.building_turns_left.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingTurnsTotal)]
    pub fn building_turns_total(&self) -> Vec<u16> {
        self.inner.building_turns_total.clone()
    }

    #[wasm_bindgen(getter, js_name = depositIds)]
    pub fn deposit_ids(&self) -> Vec<u32> {
        self.inner.deposits.ids.clone()
    }

    #[wasm_bindgen(getter, js_name = depositTiles)]
    pub fn deposit_tiles(&self) -> Vec<u32> {
        self.inner.deposits.tiles.clone()
    }

    /// `Resource` discriminants, in the order of the client's `RESOURCES`.
    #[wasm_bindgen(getter, js_name = depositResources)]
    pub fn deposit_resources(&self) -> Vec<u8> {
        self.inner.deposits.resources.clone()
    }

    /// 1 to 3, 0 once exhausted.
    #[wasm_bindgen(getter, js_name = depositLevels)]
    pub fn deposit_levels(&self) -> Vec<u8> {
        self.inner.deposits.levels.clone()
    }

    #[wasm_bindgen(getter, js_name = depositQuantities)]
    pub fn deposit_quantities(&self) -> Vec<u16> {
        self.inner.deposits.quantities.clone()
    }

    #[wasm_bindgen(getter, js_name = depositRecharges)]
    pub fn deposit_recharges(&self) -> Vec<u16> {
        self.inner.deposits.recharges.clone()
    }

    /// `0xFFFF` while nothing draws on the deposit.
    #[wasm_bindgen(getter, js_name = depositTurnsLeft)]
    pub fn deposit_turns_left(&self) -> Vec<u16> {
        self.inner.deposits.turns_left.clone()
    }

    #[wasm_bindgen(getter, js_name = buildingsRemoved)]
    pub fn buildings_removed(&self) -> Vec<u32> {
        self.inner.buildings_removed.clone()
    }
}

impl WorldDeltaView {
    fn build(delta: WorldDelta, ground: &Ground) -> Self {
        let (terrain, vegetation) = delta
            .tile_indices
            .iter()
            .zip(delta.visibility.iter())
            .map(|(&index, &visibility)| ground.seen(index as usize, visibility))
            .unzip();
        Self {
            meta: meta_json(&MetaWire::from(&delta.meta)),
            terrain,
            vegetation,
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

/// What a new game starts with, as its scenario script says it.
struct StartingContent {
    unit_templates: Vec<UnitTemplate>,
    building_templates: Vec<BuildingTemplate>,
    /// One per nation, in `NationId` order.
    nations: Vec<NationSetup>,
}

impl StartingContent {
    /// Everything the game's scenario says, every slot asked for up front: a slot the script gets
    /// wrong is refused before a planet is generated for nothing.
    fn from_scripts(scripts: &ScriptEngine, nations: u16) -> Result<Self, String> {
        let scenario = scripts
            .scenario()
            .ok_or("a game needs a scenario script, to say what its nations start with")?;
        Ok(Self {
            unit_templates: scenario.unit_templates().collect(),
            building_templates: scenario.building_templates().collect(),
            nations: (0..nations as usize)
                .map(|slot| scenario.nation(slot).map_err(|error| error.to_string()))
                .collect::<Result<_, _>>()?,
        })
    }
}

/// Write the starting content into the world, where the nations generator placed each nation: its
/// capital, its turn-1 territory, and its starting units around the capital.
fn set_up_world(world: &mut World, generated: &GeneratedNations, content: StartingContent) {
    for template in content.unit_templates {
        world.register_unit_template(template);
    }
    for template in content.building_templates {
        world.register_building_template(template);
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
    for (index, setup) in content.nations.into_iter().enumerate() {
        let nation = world.register_nation(setup.nation);

        // the generator places the nations in this very order; one it found no room for starts
        // with nothing, like a nation on a planet with no land at all
        let Some(start) = generated.nations().get(index) else {
            continue;
        };
        debug_assert_eq!(start.id(), nation);
        let mut gate = WorldMut::new(world, &mut changes);
        gate.insert_settlement(Settlement::found(setup.capital, nation, start.capital()));
        for &tile in start.territory() {
            if let Some(coords) = gate.world().tile_coords(tile)
                && let Some(tile) = gate.tile_mut(coords)
            {
                tile.artificial_mut().set_owner(Some(nation));
            }
        }
        // one unit per tile: the capital, then its land neighbours, nearest first - a roster longer
        // than there is land around the capital is cut short
        let mut sites = vec![start.capital()];
        sites.extend(world.shape().neighbors(start.capital()).filter(|&coords| {
            world
                .tile(coords)
                .is_some_and(|tile| !tile.natural().terrain().is_water())
        }));
        for (template, site) in setup.starting_units.into_iter().zip(sites) {
            world.place_unit(Unit::recruit(template, nation, site));
        }
    }
}

/// Write the generated deposits into the world, each on its tile.
///
/// A deposit's richness is its stock, except water's: water springs back, so its richness is what
/// it recharges per turn, an income on an empty stock rather than a quantity to run down. Its scale,
/// what its level is read against, is the richest its kind is generated at.
fn lay_deposits(
    world: &mut World,
    deposits: &GeneratedDeposits,
    settings: &DepositsGeneratorSettings,
) {
    for generated in deposits.deposits() {
        let Some(position) = world.tile_coords(generated.tile()) else {
            continue;
        };
        let kind = generated.kind();
        let resource = kind.resource();
        let mut held = ResourceDataStorage::new();
        if kind == DepositKind::Water {
            held.replenish(resource, 0);
            held.set_income(resource, generated.richness());
        } else {
            held.replenish(resource, generated.richness());
        }
        world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id(deposit_definition(kind)),
            position,
            held,
            settings.kind(kind).richness_max,
        ));
    }
}

/// The definition a deposit of this kind is recorded under. TODO: deposit definitions from scripts.
fn deposit_definition(kind: DepositKind) -> &'static str {
    match kind {
        DepositKind::Water => "deposit_water",
        DepositKind::Metals => "deposit_metals",
        DepositKind::Oil => "deposit_oil",
        DepositKind::Uranium => "deposit_uranium",
    }
}

/// The terrain of a tile, or `UNKNOWN_TERRAIN` while the viewer has never explored it.
fn meta_json(meta: &MetaWire) -> String {
    // a `MetaWire`'s only maps are keyed by resource names: it cannot fail to serialize
    serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string())
}

fn to_js_error(error: serde_json::Error) -> JsError {
    JsError::new(&error.to_string())
}

#[cfg(test)]
mod tests {
    use project_x_core::{
        game::session::LoggedCommand, hex_map::vegetation::Vegetation,
        simulation::fog::TileVisibility,
    };

    use super::{GameSession, UNKNOWN_TERRAIN, UNKNOWN_VEGETATION};

    /// The default scenario, as the client sends it.
    fn default_scenario() -> serde_json::Value {
        serde_json::json!({
            "name": "default",
            "kind": "scenario",
            "source": include_str!("../../scripts/scenarios/default.rhai"),
        })
    }

    /// A new game's settings: a map small enough to assert over, big enough to have land and
    /// unexplored corners, the default scenario - and whatever `overrides` replaces of those.
    fn settings(overrides: serde_json::Value) -> String {
        let mut settings = serde_json::json!({
            "seed": 7,
            "columns": 24,
            "rows": 16,
            "nations": 2,
            "scripts": [default_scenario()],
        });
        let (Some(settings_map), Some(overrides)) =
            (settings.as_object_mut(), overrides.as_object())
        else {
            panic!("settings are objects");
        };
        settings_map.extend(overrides.clone());
        settings.to_string()
    }

    fn start_game() -> GameSession {
        GameSession::new_game(&settings(serde_json::json!({}))).expect("the settings are valid")
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

    /// The scout AI, as the client sends it.
    fn scout_ai() -> serde_json::Value {
        serde_json::json!({
            "name": "scout",
            "kind": "ai",
            "source": include_str!("../../scripts/ai/scout.rhai"),
        })
    }

    /// A game on `seed` with the client's default scripts, played for `turns` turns by its AI
    /// nations alone: every order they gave, and where every unit ended up.
    fn scripted_game(seed: u32, turns: usize) -> (Vec<LoggedCommand>, String) {
        let mut session = GameSession::new_game(&settings(serde_json::json!({
            "seed": seed,
            "nations": 3,
            "scripts": [default_scenario(), scout_ai()],
        })))
        .expect("the settings are valid");
        for _ in 0..turns {
            resolve_turn(&mut session);
        }
        let world = session.inner.world();
        (
            session.inner.command_log().to_vec(),
            format!("{:?}", world.units()),
        )
    }

    /// `seed + scripts + command log` must replay a game exactly: the scripts draw their numbers
    /// from their nation's own stream, read no clock, and iterate nothing in an unstable order.
    ///
    /// Also run in WASM, by `wasm-pack test --node wasm_bridge`: the client only ever runs there.
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn test_a_scripted_game_replays_from_its_seed_and_its_scripts() {
        let (orders, units) = scripted_game(7, 5);
        // two scripted nations of three units each, five turns: enough for a divergence to show
        assert!(orders.len() >= 20, "only {} orders", orders.len());

        let (replayed_orders, replayed_units) = scripted_game(7, 5);
        assert_eq!(
            replayed_orders, orders,
            "same seed, same scripts: same orders"
        );
        assert_eq!(replayed_units, units, "and the same world after them");

        let (other_orders, _) = scripted_game(8, 5);
        assert_ne!(other_orders, orders, "another seed is another game");
    }

    #[test]
    fn test_a_game_sent_no_scenario_is_refused() {
        use project_x_scripting::ScriptEngine;

        let error = super::StartingContent::from_scripts(&ScriptEngine::load(&[]).unwrap(), 2)
            .err()
            .expect("no scenario, no game");
        assert!(error.contains("scenario"), "{error}");
    }

    #[test]
    fn test_a_game_sent_an_ai_script_has_its_nations_played_by_it() {
        use project_x_core::{
            game::commands::GameCommand,
            simulation::ids::{EntityIndex, NationId},
        };

        let settings = settings(serde_json::json!({
            "scripts": [default_scenario(), scout_ai()],
        }));
        let mut session = GameSession::new_game(&settings).expect("the settings are valid");
        let ai = NationId::from_index(1);
        assert_eq!(
            session.inner.controller(ai).map(|slot| slot.name()),
            Some("scout")
        );

        resolve_turn(&mut session);

        let moves = session
            .inner
            .command_log()
            .iter()
            .filter(|logged| logged.nation == ai)
            .filter(|logged| matches!(logged.command, GameCommand::MoveUnit { .. }))
            .count();
        assert!(moves > 0, "the scripted nation never moved");
        let events = session.drain_events().expect("events");
        assert!(!events.contains(r#""type":"commandRejected""#), "{events}");
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
        let session = GameSession::new_game(&settings(serde_json::json!({"oceanCoverage": 0.5})))
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
    fn test_a_new_games_meta_carries_the_scenarios_building_templates() {
        let snapshot = start_game().snapshot(0);
        let meta: serde_json::Value = serde_json::from_str(&snapshot.meta()).unwrap();
        let templates = meta["buildingTemplates"].as_array().expect("in the meta");
        let mine = templates
            .iter()
            .find(|template| template["type"] == "mine")
            .expect("the default scenario's mine");

        assert_eq!(mine["id"], "mine", "what a construct order names it by");
        assert_eq!(mine["deposit"], "metals");
        let levels = mine["levels"].as_array().unwrap();
        assert_eq!(levels.len(), 3);
        for (index, level) in levels.iter().enumerate() {
            assert_eq!(level["sprite"], format!("mine_{}", index + 1));
            assert!(level["turns"].as_u64().is_some_and(|turns| turns >= 1));
            // resources by the names the client knows them by, as objects
            for figures in ["cost", "maintenance", "production"] {
                let figures = level[figures].as_object().expect("an object");
                assert!(figures.values().all(serde_json::Value::is_u64));
            }
            assert!(
                level["production"]["metals"]
                    .as_u64()
                    .is_some_and(|metals| metals > 0)
            );
        }
        let solar = templates
            .iter()
            .find(|template| template["type"] == "solar_plant")
            .unwrap();
        assert!(solar["deposit"].is_null(), "it extracts nothing");
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
    fn test_a_new_game_lays_the_generated_deposits_on_their_tiles() {
        let session = start_game();
        let world = session.inner.world();
        assert!(
            !world.deposits().is_empty(),
            "a planet with land has deposits"
        );
        let mut kinds = std::collections::BTreeSet::new();
        for (id, deposit) in world.deposits().iter() {
            let tile = world.tile(deposit.position()).expect("on the map");
            assert!(
                tile.natural().deposits().contains(&id),
                "{id} is listed on its tile"
            );
            assert!(!tile.natural().terrain().is_water(), "{id} is on land");
            // one resource each, and something of it: a stock, or water's recharge
            let held: Vec<_> = deposit.resources().iter().collect();
            assert_eq!(held.len(), 1, "{id} holds one resource");
            let (resource, data) = held[0];
            assert!(data.quantity() > 0 || data.income().is_some_and(|recharge| recharge > 0));
            kinds.insert(*resource);
        }
        // every nation is guaranteed water and metals, so both are always there
        assert!(kinds.contains(&project_x_core::simulation::resources::Resource::Water));
        assert!(kinds.contains(&project_x_core::simulation::resources::Resource::Metals));

        // and the viewer's tile flags say so wherever it has looked
        let snapshot = session.snapshot(0);
        let (flags, visibility) = (snapshot.tile_flags(), snapshot.visibility());
        for (index, coords) in world.shape().coords().enumerate() {
            let has = !world.tile(coords).unwrap().natural().deposits().is_empty();
            let seen = visibility[index] != TileVisibility::Unexplored as u8;
            let flagged = flags[index] & project_x_core::game::projection::tile_flags::DEPOSIT != 0;
            assert_eq!(flagged, has && seen, "tile {index}");
        }
    }

    #[test]
    fn test_a_new_games_snapshot_carries_the_deposits_the_viewer_has_explored() {
        use project_x_core::simulation::resources::Resource;

        let session = start_game();
        let world = session.inner.world();
        let snapshot = session.snapshot(0);
        let (ids, tiles, resources, levels) = (
            snapshot.deposit_ids(),
            snapshot.deposit_tiles(),
            snapshot.deposit_resources(),
            snapshot.deposit_levels(),
        );
        let (quantities, recharges) = (snapshot.deposit_quantities(), snapshot.deposit_recharges());
        let visibility = snapshot.visibility();
        let explored = world
            .deposits()
            .iter()
            .filter(|(_, deposit)| {
                let index = world.tile_index(deposit.position()).unwrap() as usize;
                visibility[index] != TileVisibility::Unexplored as u8
            })
            .count();
        assert_eq!(
            ids.len(),
            explored,
            "every deposit the viewer has explored, and only those"
        );
        assert!(
            explored > 0,
            "a capital's surroundings are guaranteed water and metals"
        );
        for (k, &tile) in tiles.iter().enumerate() {
            assert!(
                (1..=3).contains(&levels[k]),
                "a fresh deposit is not exhausted"
            );
            assert_ne!(visibility[tile as usize], TileVisibility::Unexplored as u8);
            // water springs back: no stock, a recharge; every other kind the reverse
            if resources[k] == Resource::Water as u8 {
                assert_eq!(quantities[k], 0);
                assert!(recharges[k] > 0);
            } else {
                assert!(quantities[k] > 0);
                assert_eq!(recharges[k], 0);
            }
        }
        assert!(
            snapshot
                .deposit_turns_left()
                .iter()
                .all(|&left| left == u16::MAX)
        );
    }

    #[test]
    fn test_the_world_knows_the_terrain_of_its_tiles() {
        let session = start_game();
        let world = session.inner.world();
        // every tile carries the kind the client is shown, so the rules and the map agree
        for (index, coords) in world.shape().coords().enumerate() {
            let kind = world.tile(coords).expect("on the map").natural().terrain();
            assert_eq!(kind as u8, session.ground.terrain[index], "tile {index}");
            let grown = world.tile(coords).unwrap().natural().vegetation();
            assert_eq!(
                grown as u8, session.ground.vegetation[index],
                "tile {index}"
            );
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
        // nor its forests
        let vegetation = snapshot.vegetation();
        assert_eq!(vegetation.len(), visibility.len());
        for (index, (&grown, &seen)) in vegetation.iter().zip(visibility.iter()).enumerate() {
            assert_eq!(grown == UNKNOWN_VEGETATION, seen == 0, "tile {index}");
        }
        assert!(
            session
                .ground
                .vegetation
                .contains(&(Vegetation::Forest as u8)),
            "the planet grows forests"
        );
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
        assert_eq!(delta.vegetation().len(), indices.len());
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
