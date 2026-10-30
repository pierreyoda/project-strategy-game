//! What kind of ground a tile is.
//!
//! Classified by `procgen` from a generated map's elevations and latitudes, and written into the
//! tiles when a world is set up: the simulation reads it for movement, the client for drawing. It
//! lives here rather than in `procgen` because rules depend on it, and rules live in `core`.

/// Terrain of a tile.
///
/// The discriminants are part of the WASM bridge's API, mirrored by the client's `TerrainKind`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TerrainKind {
    DeepWater = 0,
    ShallowWater = 1,
    /// The default of a tile nobody classified, which is also the one every rule treats as plain.
    #[default]
    Flat = 2,
    Hills = 3,
    Mountains = 4,
    Ice = 5,
    /// Ice at the elevations of hills and mountains.
    RoughIce = 6,
}

impl TerrainKind {
    /// Whether the ground is water, which no land unit crosses.
    pub fn is_water(self) -> bool {
        matches!(self, Self::DeepWater | Self::ShallowWater)
    }
}
