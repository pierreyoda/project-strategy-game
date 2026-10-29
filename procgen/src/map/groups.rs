//! The groups of noise functions composing the map procgen, one module per group
//! (except for the 4 scaled terrain groups, sharing one).
//!
//! Each group is built from the settings and the upstream groups it depends on,
//! and exposes the output of each of its subgroups.

pub use self::{
    badlands::BadlandsGroup, continents::ContinentGroup, final_planet::FinalPlanetGroup,
    hilly::HillyGroup, mountainous::MountainousGroup, plains::PlainsGroup,
    rivers::RiverPositionsGroup, scaled::ScaledTerrainGroup, terrain_type::TerrainTypeGroup,
};

mod badlands;
mod continents;
mod final_planet;
mod hilly;
mod mountainous;
mod plains;
mod rivers;
mod scaled;
mod terrain_type;
