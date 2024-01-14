//! The static layer of an `HexMap` involve naturally-evolved properties that do not fundamentally change during gameplay but is set at world generation.
//!
//! Some properties may still be affected during gameplay though, for instance the initial accessible quantity of a resource diminishing from mining activity.

use std::fmt::Debug;

#[derive(Debug)]
pub struct HexMapWorldTileData {
    deposits: Option<Vec<Box<dyn HexMapTileDeposit>>>,
}

/// A resource deposit on the tile.
pub trait HexMapTileDeposit: Debug {}
