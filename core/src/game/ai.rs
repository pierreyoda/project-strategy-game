//! The slot a non-human nation is played by.
//!
//! Scaffolding, deliberately: `StubAiController` plans nothing. What matters at this stage is the
//! shape of the thing, because it is the shape that keeps the AI honest.
//!
//! A controller is handed a **read-only, fog-filtered** `NationView` and returns `GameCommand`s. It
//! cannot reach the world, so it cannot write to it, so it cannot see what its nation should not
//! see nor break a rule the player is held to. Its commands go through `commands::apply` like
//! anyone else's, and a command it gets wrong comes back as a `CommandRejected` event rather than
//! as a silent corruption of the world.
//!
//! The scripted controller that will eventually live in the `scripting` crate plugs in here, and
//! nothing else in the loop changes.

use std::fmt::Debug;

use crate::{
    game::{commands::GameCommand, rng::GameRng, state::World, turn::StepBudget},
    simulation::{
        fog::TileVisibility,
        ids::{EntityIndex, NationId, SettlementId, UnitId},
        military::Unit,
        settlements::Settlement,
    },
};

/// What one nation is allowed to know of the world.
///
/// Borrowed for the duration of a planning step, so a controller cannot squirrel it away and read
/// it later, after the world moved on.
#[derive(Debug)]
pub struct NationView<'a> {
    world: &'a World,
    nation: NationId,
}

impl<'a> NationView<'a> {
    pub fn new(world: &'a World, nation: NationId) -> Self {
        Self { world, nation }
    }

    pub fn nation(&self) -> NationId {
        self.nation
    }

    pub fn turn(&self) -> crate::game::GameTurnScalar {
        self.world.turn()
    }

    pub fn shape(&self) -> crate::hex_map::shape::RectangularHexMapShape {
        self.world.shape()
    }

    /// What this nation knows of a tile.
    pub fn visibility(&self, coords: crate::hex_map::coordinates::CubeCoords) -> TileVisibility {
        let Some(index) = self.world.tile_index(coords) else {
            return TileVisibility::Unexplored;
        };
        self.world
            .fog_of(self.nation)
            .map_or(TileVisibility::Unexplored, |fog| {
                fog.visibility(index as usize)
            })
    }

    /// This nation's own units.
    pub fn own_units(&self) -> impl Iterator<Item = (UnitId, &Unit)> {
        self.world
            .units()
            .iter()
            .filter(move |(_, unit)| unit.owner() == self.nation)
    }

    /// This nation's own settlements.
    pub fn own_settlements(&self) -> impl Iterator<Item = (SettlementId, &Settlement)> {
        self.world
            .settlements()
            .iter()
            .filter(move |(_, settlement)| settlement.owner() == self.nation)
    }

    /// Foreign units standing on a tile this nation is watching right now.
    ///
    /// Fogged tiles are left out on purpose: what was there last time it was looked at is not what
    /// is there now, and an AI that plans against stale positions should have to say so explicitly.
    pub fn visible_foreign_units(&self) -> impl Iterator<Item = (UnitId, &Unit)> {
        self.world.units().iter().filter(move |(_, unit)| {
            unit.owner() != self.nation
                && self.visibility(unit.position()) == TileVisibility::Visible
        })
    }

    /// Read-only access to the whole world.
    ///
    /// **NB** This is the escape hatch fog exists to prevent: it is here for debugging tooling and
    /// for an "omniscient" difficulty setting, and a controller reaching for it is cheating.
    pub fn world_unfiltered(&self) -> &World {
        self.world
    }
}

/// Plans one nation's turn.
pub trait NationController: Debug {
    /// What this controller is, for logs and for the turn's progress report.
    fn name(&self) -> &str;

    /// Plan, within the step's budget.
    ///
    /// Returning fewer commands than the nation could issue is fine: a controller that needs more
    /// than one step says so with `needs_more_steps`, and is called again with a fresh budget.
    fn plan(&mut self, view: &NationView<'_>, budget: StepBudget) -> Vec<GameCommand>;

    /// Whether this controller wants another step before the phase moves on.
    fn needs_more_steps(&self) -> bool {
        false
    }
}

/// The scaffold: a nation that does nothing at all.
///
/// It carries its own RNG stream so that the day it starts drawing numbers, it cannot shift any
/// other subsystem's stream by doing so.
#[derive(Debug)]
pub struct StubAiController {
    rng: GameRng,
}

impl StubAiController {
    pub fn new(rng: GameRng) -> Self {
        Self { rng }
    }

    /// Its own random stream, for whatever eventually plans with it.
    pub fn rng_mut(&mut self) -> &mut GameRng {
        &mut self.rng
    }
}

impl NationController for StubAiController {
    fn name(&self) -> &str {
        "stub"
    }

    fn plan(&mut self, _view: &NationView<'_>, _budget: StepBudget) -> Vec<GameCommand> {
        // TODO: this is where the AI goes. Everything it will need is already in `view`, and
        // everything it may do is already a `GameCommand`.
        Vec::new()
    }
}

/// Who plays a nation.
#[derive(Debug)]
pub enum ControllerSlot {
    /// A person: their commands arrive from the client, so there is nothing to plan here.
    Human,
    Ai(Box<dyn NationController>),
}

impl ControllerSlot {
    pub fn is_ai(&self) -> bool {
        matches!(self, Self::Ai(_))
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Human => "human",
            Self::Ai(controller) => controller.name(),
        }
    }
}

/// The stub controller for a nation, seeded off the playthrough's seed.
pub fn stub_ai_for(seed: crate::game::rng::GameSeed, nation: NationId) -> ControllerSlot {
    use crate::game::rng::RngPurpose;
    debug_assert!(nation.index() <= NationId::MAX_INDEX);
    ControllerSlot::Ai(Box::new(StubAiController::new(GameRng::new(
        seed,
        RngPurpose::Ai(nation),
    ))))
}
