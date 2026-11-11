//! Scripts: the game's content, and what plays for its nations, written in Rhai and run inside the
//! simulation.
//!
//! `core` knows nothing of any of this. A script plays for a nation through `NationController`,
//! like any other controller: it is handed a fog-filtered view, and all it can do is *ask*, by
//! pushing `GameCommand`s that are judged by the same rules as the player's.
//!
//! *For* a nation, not only *as* one: an AI script plays a whole AI nation, but a human nation will
//! have parts of it played by scripts too - automation the player opts into, a disloyal general.
//!
//! The pieces:
//!
//! - `engine`: the Rhai engine, its limits, and loading a game's scripts;
//! - `api`: what a script sees - coordinates, the view, its orders, its random numbers;
//! - `controller`: the `NationController` that runs an AI script's `plan`;
//! - `scenario`: what a new game starts with, as a scenario script says it.
//!
//! Every script must replay: nothing here reads the clock, and the only random numbers a script
//! draws come from its nation's own `GameRng` stream.

mod api;
mod controller;
mod engine;
mod scenario;

pub use controller::ScriptedController;
pub use engine::{ScriptEngine, ScriptError, ScriptKind, ScriptLimits, ScriptSource};
pub use scenario::{NationSetup, Scenario, ScriptedDistrictRules};
