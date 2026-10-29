//! Turn resolution, one bounded step at a time.
//!
//! The loop never makes a single blocking call into the simulation. The planet generator can afford
//! to - cancelling it means killing its worker, and a half-generated planet is worth nothing - but a
//! session's world *is* the game: it cannot be thrown away to interrupt it.
//!
//! So resolution is a state machine. `GameSession::step` does roughly `StepBudget` worth of work and
//! returns where it got to; the worker driving it posts progress, reads its inbox, and calls again.
//! Every phase keeps a cursor, so a phase too large for one step simply resumes.

use std::collections::VecDeque;

use crate::{game::GameTurnScalar, game::journal::StateVersion, simulation::ids::NationId};

/// The phases of a turn, in the order they run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnPhase {
    /// Seal the orders given during the player's turn.
    PlayerCommit,
    /// One AI nation plans and its commands are applied.
    AiPlanning {
        nation: NationId,
    },
    Economy,
    Logistics,
    Military,
    Population,
    /// Recompute every nation's field of view.
    Fog,
    /// Bump the turn counter and refill what a new turn refills.
    Finalize,
}

impl TurnPhase {
    pub fn name(&self) -> &'static str {
        match self {
            Self::PlayerCommit => "player_commit",
            Self::AiPlanning { .. } => "ai_planning",
            Self::Economy => "economy",
            Self::Logistics => "logistics",
            Self::Military => "military",
            Self::Population => "population",
            Self::Fog => "fog",
            Self::Finalize => "finalize",
        }
    }
}

/// How much work one `step` may do.
///
/// The unit is deliberately vague - a tile visited, a nation planned for, a settlement updated -
/// because its only job is to keep a step short enough that the worker stays responsive.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StepBudget {
    pub work_units: u32,
}

impl StepBudget {
    /// A step small enough to keep the worker answering messages, large enough not to spend the
    /// turn crossing the WASM boundary.
    pub const DEFAULT_WORK_UNITS: u32 = 4096;

    pub fn new(work_units: u32) -> Self {
        Self {
            work_units: work_units.max(1),
        }
    }
}

impl Default for StepBudget {
    fn default() -> Self {
        Self::new(Self::DEFAULT_WORK_UNITS)
    }
}

/// Where a `step` got to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TurnProgress {
    /// No turn is being resolved.
    Idle,
    Working {
        phase: TurnPhase,
        percent: u8,
    },
    Done {
        turn: GameTurnScalar,
        version: StateVersion,
    },
}

/// The ordered skeleton of a turn. `AiPlanning` is expanded into one entry per AI nation when the
/// turn starts, which is why it is not in here.
const PHASE_SKELETON: usize = 8;

/// The resolution in progress.
#[derive(Debug)]
pub struct TurnState {
    /// Phases left to run, in order.
    remaining: VecDeque<TurnPhase>,
    /// How many phases this turn has in total, for the progress percentage.
    total_phases: usize,
    /// Where the current phase got to: its meaning is the phase's own.
    cursor: usize,
}

impl TurnState {
    /// Start a turn, queueing one planning phase per AI nation.
    pub fn begin(ai_nations: impl IntoIterator<Item = NationId>) -> Self {
        let mut remaining = VecDeque::with_capacity(PHASE_SKELETON);
        remaining.push_back(TurnPhase::PlayerCommit);
        for nation in ai_nations {
            remaining.push_back(TurnPhase::AiPlanning { nation });
        }
        remaining.push_back(TurnPhase::Economy);
        remaining.push_back(TurnPhase::Logistics);
        remaining.push_back(TurnPhase::Military);
        remaining.push_back(TurnPhase::Population);
        remaining.push_back(TurnPhase::Fog);
        remaining.push_back(TurnPhase::Finalize);
        let total_phases = remaining.len();
        Self {
            remaining,
            total_phases,
            cursor: 0,
        }
    }

    pub fn current_phase(&self) -> Option<&TurnPhase> {
        self.remaining.front()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn advance_cursor(&mut self, by: usize) {
        self.cursor += by;
    }

    /// Move on to the next phase, resetting its cursor.
    pub fn finish_phase(&mut self) {
        self.remaining.pop_front();
        self.cursor = 0;
    }

    pub fn is_done(&self) -> bool {
        self.remaining.is_empty()
    }

    /// How far through the turn this is, counted in phases.
    pub fn percent(&self) -> u8 {
        if self.total_phases == 0 {
            return 100;
        }
        let done = self.total_phases - self.remaining.len();
        ((done * 100) / self.total_phases) as u8
    }
}

#[cfg(test)]
mod tests {
    use crate::simulation::ids::{EntityIndex, NationId};

    use super::{StepBudget, TurnPhase, TurnState};

    #[test]
    fn test_turn_state_queues_one_planning_phase_per_ai_nation() {
        let ai = [NationId::from_index(1), NationId::from_index(2)];
        let mut state = TurnState::begin(ai);
        assert_eq!(state.current_phase(), Some(&TurnPhase::PlayerCommit));
        assert_eq!(state.percent(), 0);

        state.finish_phase();
        assert_eq!(
            state.current_phase(),
            Some(&TurnPhase::AiPlanning { nation: ai[0] })
        );
        state.finish_phase();
        assert_eq!(
            state.current_phase(),
            Some(&TurnPhase::AiPlanning { nation: ai[1] })
        );

        while !state.is_done() {
            state.finish_phase();
        }
        assert_eq!(state.percent(), 100);
        assert_eq!(state.current_phase(), None);
    }

    #[test]
    fn test_step_budget_is_never_zero() {
        assert_eq!(StepBudget::new(0).work_units, 1);
        assert_eq!(
            StepBudget::default().work_units,
            StepBudget::DEFAULT_WORK_UNITS
        );
    }
}
