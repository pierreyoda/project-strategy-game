//! A script's random numbers: its nation's own stream, and nothing else.
//!
//! No random package is registered with the engine. Every number a script draws comes from the
//! `RngPurpose::Ai` stream of the nation it plays, which is what lets a playthrough replay from its
//! seed, and what keeps one AI's draws from shifting another's.

use std::{cell::RefCell, rc::Rc};

use rand::RngExt;
use rhai::{Engine, INT};

use project_x_core::game::rng::GameRng;

use super::ScriptResult;

/// A shared handle on a nation's random stream.
#[derive(Clone, Debug)]
pub struct ScriptRng(Rc<RefCell<GameRng>>);

impl ScriptRng {
    pub fn new(rng: GameRng) -> Self {
        Self(Rc::new(RefCell::new(rng)))
    }
}

pub fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<ScriptRng>("Rng")
        .register_fn(
            "range",
            // `max` excluded, like `random_range` and like a script's own `range(0, 10)`
            |rng: &mut ScriptRng, min: INT, max: INT| -> ScriptResult<INT> {
                if min >= max {
                    return Err(format!("cannot draw from the empty range {min}..{max}").into());
                }
                Ok(rng.0.borrow_mut().random_range(min..max))
            },
        );
}
