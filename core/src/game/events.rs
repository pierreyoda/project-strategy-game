//! What the client is *told* about a turn, as opposed to what it is *shown*.
//!
//! Events are narrative: a turn log, a combat report, a nation running out of fuel. They are small,
//! irregular and ordered, which is why they are pushed to the client as they happen while the state
//! itself is pulled.
//!
//! They are never a substitute for the projection. A client that rebuilt its world from events
//! would be a second implementation of the simulation, in TypeScript, drifting from this one.

use crate::{
    game::{GameTurnScalar, state::TileIndex},
    simulation::{
        ids::{BuildingId, NationId, SettlementId, UnitId},
        resources::Resource,
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameEvent {
    /// A new turn started.
    TurnStarted { turn: GameTurnScalar },
    /// A line for the UI's turn log. `nation` is `None` for anything the whole world sees.
    Log {
        nation: Option<NationId>,
        message: String,
    },
    UnitMoved {
        unit: UnitId,
        from: TileIndex,
        to: TileIndex,
    },
    SettlementFounded {
        settlement: SettlementId,
        nation: NationId,
        tile: TileIndex,
    },
    BuildingConstructed {
        building: BuildingId,
        settlement: SettlementId,
    },
    ResourceShortage {
        nation: NationId,
        resource: Resource,
    },
    /// A command was refused.
    ///
    /// For a player this is feedback; for an AI it is a bug report, which is exactly why the AI
    /// goes through the same validation rather than writing to the world directly.
    CommandRejected { nation: NationId, reason: String },
}

/// Events buffered until the client drains them.
#[derive(Debug, Default)]
pub struct EventBuffer {
    events: Vec<GameEvent>,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &GameEvent> {
        self.events.iter()
    }

    /// Hand over everything buffered, leaving the buffer empty.
    pub fn drain(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
}
