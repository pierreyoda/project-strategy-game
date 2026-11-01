//! What grows on a tile, over its terrain.
//!
//! A layer beside the `TerrainKind` rather than a kind of terrain: a wooded plain is still a plain
//! to every rule keyed on the ground, and the vegetation adds its own on top - a forest slows the
//! units entering it and hides what lies past it. Grown by `procgen` and painted into the tiles
//! when a world is set up, like the terrain.

/// Vegetation of a tile.
///
/// The discriminants are part of the WASM bridge's API, mirrored by the client's `Vegetation`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Vegetation {
    /// Bare ground, the default of a tile nobody painted.
    #[default]
    None = 0,
    Forest = 1,
}

impl Vegetation {
    pub fn is_forest(self) -> bool {
        self == Self::Forest
    }
}
