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
        ai::stub_ai_for,
        projection::{Projection, WorldDelta, WorldSnapshot},
        session::GameSession as CoreSession,
        state::World,
        turn::{StepBudget, TurnProgress},
    },
    hex_map::shape::RectangularHexMapShape,
    simulation::{
        ids::{EntityIndex, NationId, SimulationID},
        military::{Unit, UnitTemplate},
        nations::Nation,
    },
};
use project_x_procgen::map::{
    MAX_SEED, MapGenerator, MapGeneratorSettings, TerrainClassifier, TerrainKind,
};

use crate::wire::{CommandWire, EventWire, MetaWire, NewGameSettings, RejectionWire};

/// The scaffold's starting unit. TODO: templates come from scripts.
const SCOUT_TEMPLATE: &str = "scout";

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

        let generator_settings =
            MapGeneratorSettings::new(settings.seed, settings.columns, settings.rows);
        let map = MapGenerator::from_settings(MapGeneratorSettings::new(
            settings.seed,
            settings.columns,
            settings.rows,
        ))
        .generate(|_| {});
        let shape = RectangularHexMapShape::new(settings.columns, settings.rows);

        let classifier = TerrainClassifier::new(&generator_settings);
        let terrain: Vec<u8> = shape
            .coords()
            .map(|coords| {
                let elevation = map.tile(&coords).expect("missing tile").elevation();
                classifier.classify(coords.into(), elevation) as u8
            })
            .collect();

        let mut world = World::new(shape, map);
        set_up_scaffold_world(&mut world, settings.nations, &terrain);

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

    /// Everything the viewer can see, from scratch.
    pub fn snapshot(&self, viewer: u16) -> WorldSnapshotView {
        WorldSnapshotView::build(
            self.inner.snapshot(NationId::from_index(viewer as usize)),
            self.terrain.clone(),
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
                delta: Some(WorldDeltaView::build(delta)),
            },
            Projection::Full(snapshot) => WorldProjectionView {
                full: Some(WorldSnapshotView::build(snapshot, self.terrain.clone())),
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
}

impl WorldSnapshotView {
    fn build(snapshot: WorldSnapshot, terrain: Vec<u8>) -> Self {
        Self {
            meta: meta_json(&MetaWire::from(&snapshot.meta)),
            columns: snapshot.columns,
            rows: snapshot.rows,
            terrain,
            inner: snapshot,
        }
    }
}

/// A catch-up. The tile arrays are parallel to `tileIndices`, not to the map.
#[wasm_bindgen]
pub struct WorldDeltaView {
    meta: String,
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

    #[wasm_bindgen(getter, js_name = settlementsRemoved)]
    pub fn settlements_removed(&self) -> Vec<u32> {
        self.inner.settlements_removed.clone()
    }
}

impl WorldDeltaView {
    fn build(delta: WorldDelta) -> Self {
        Self {
            meta: meta_json(&MetaWire::from(&delta.meta)),
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

/// Nations, and one scout each on a habitable tile.
///
/// TODO: this is the scaffold standing in for a scenario. Nations, their rosters and their starting
/// position all belong to scripts, and this function is what they will replace.
fn set_up_scaffold_world(world: &mut World, nations: u16, terrain: &[u8]) {
    world.register_unit_template(UnitTemplate::new(
        SimulationID::new_abstract_id(SCOUT_TEMPLATE),
        "scout".to_string(),
        2,
        2,
    ));

    let shape = world.shape();
    let habitable: Vec<usize> = terrain
        .iter()
        .enumerate()
        .filter(|(_, kind)| **kind == TerrainKind::Flat as u8 || **kind == TerrainKind::Hills as u8)
        .map(|(index, _)| index)
        .collect();

    for index in 0..nations as usize {
        let nation = world.register_nation(Nation::new(
            SimulationID::new_abstract_id(&format!("nation_{index}")),
            format!("Nation {}", index + 1),
        ));
        // spread the starts evenly through the habitable tiles, so nobody starts in anyone's lap
        if habitable.is_empty() {
            continue;
        }
        let tile = habitable[(index * habitable.len()) / nations.max(1) as usize];
        let Some(coords) = shape.coords().nth(tile) else {
            continue;
        };
        world.place_unit(Unit::recruit(
            SimulationID::new_abstract_id(SCOUT_TEMPLATE),
            nation,
            coords,
        ));
    }
}

fn meta_json(meta: &MetaWire) -> String {
    // a `MetaWire` has no map and no non-string keys: it cannot fail to serialize
    serde_json::to_string(meta).unwrap_or_else(|_| "{}".to_string())
}

fn to_js_error(error: serde_json::Error) -> JsError {
    JsError::new(&error.to_string())
}
