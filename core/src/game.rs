//! The game loop: the world, what mutates it, and what the client is shown of it.
//!
//! The pieces, in the order they appear in a turn:
//!
//! - `state`: the `World` and the `WorldMut` gate every mutation goes through;
//! - `genesis`: how the world was generated, which the simulation reads and never changes;
//! - `commands`: what a player - or an AI - asks of the world, and why it can be refused;
//! - `turn`: the resolution pipeline, driven one bounded step at a time;
//! - `ai`: the controller a non-human nation plans with, emitting the very same commands;
//! - `journal`: what changed since which version;
//! - `projection`: what one nation is allowed to see of it, as buffers the client can render;
//! - `session`: the façade that ties all of the above together, and the only thing the WASM bridge
//!   ever holds.

pub mod ai;
pub mod commands;
pub mod events;
pub mod genesis;
pub mod journal;
pub mod projection;
pub mod rng;
pub mod session;
pub mod state;
pub mod turn;

/// Turn number unit (starts at 1).
pub type GameTurnScalar = u32;
