//! The dynamic layer of an `HexMap` will involve geographically static, artificially built objects like buildings.

use std::fmt::Debug;

#[derive(Debug)]
pub struct HexMapArtificialTileDate {
    settlement: Option<Box<dyn HexMapTileSettlement>>,
    buildings: Option<Vec<Box<dyn HexMapTileBuilding>>>,
}

/// A lived-in settlement on the tile, regardless of size or status (from tiny remote outpost or village, to State capital).
pub trait HexMapTileSettlement: Debug {}

/// A built building on the tile.
pub trait HexMapTileBuilding: Debug {}
