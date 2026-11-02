//! What a script sees.
//!
//! Rust types never leak to the script side - the rule `properties.rs` already states. Coordinates
//! are object maps `#{q, r}` (axial), IDs are plain integers, terrain and visibility are strings.
//!
//! The custom types a script is handed - `View`, `Orders`, `Rng` - are shared handles: Rhai clones
//! every value it passes to a function, and a copy of a handle still points at the same orders and
//! the same random stream.
//!
//! **NB** Every number a script hands back is checked before it reaches `core`, whose constructors
//! assert on what they are given: a panic in the worker is the end of the game.

mod common;
mod coords;
mod orders;
mod rng;
mod scenario;
mod view;

use rhai::{Engine, EvalAltResult, INT};

use project_x_core::simulation::ids::EntityIndex;

pub use common::Fields;
pub use orders::Orders;
pub use rng::ScriptRng;
pub use scenario::{Definitions, ScenarioHandle, UnitDefinition};
pub use view::ScriptView;

/// Register everything a script may call.
pub fn register(engine: &mut Engine) {
    coords::register(engine);
    orders::register(engine);
    rng::register(engine);
    scenario::register(engine);
    view::register(engine);
}

/// The result of a function a script calls.
pub type ScriptResult<T> = Result<T, Box<EvalAltResult>>;

/// An entity ID from a script, or why it cannot be one.
fn entity_id<I: EntityIndex>(value: INT, what: &str) -> ScriptResult<I> {
    usize::try_from(value)
        .ok()
        .filter(|&index| index <= I::MAX_INDEX)
        .map(I::from_index)
        .ok_or_else(|| format!("{value} is not a {what} ID").into())
}
