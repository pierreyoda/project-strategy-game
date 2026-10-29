use std::{collections::HashMap, fmt};

use self::{coordinates::CubeCoords, tile::HexMapTile};

pub mod coordinates;
pub mod layers;
pub mod shape;
pub mod tile;

pub type HexMapStorage = HashMap<CubeCoords, HexMapTile>;

/// Compass-like directions for usage with an `HexMap`.
///
/// Friendlier in usage than lower-level directions in the `coordinates` sub-module.
#[derive(Debug)]
pub enum HexMapDirection {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

// TODO: dont forget preallocation from size map
pub struct HexMap {
    tiles: HexMapStorage,
}

impl HexMap {
    pub fn from_tiles(tiles: HexMapStorage) -> Self {
        Self { tiles }
    }

    pub fn tile(&self, coords: &CubeCoords) -> Option<&HexMapTile> {
        self.tiles.get(coords)
    }
}

impl fmt::Debug for HexMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::hex_map::{
        HexMap, HexMapStorage, coordinates::CubeCoordsScalar, shape::RectangularHexMapShape,
        tile::HexMapTile,
    };

    fn generate_small_rectangular_hashmap(
        columns: CubeCoordsScalar,
        rows: CubeCoordsScalar,
    ) -> HexMap {
        let shape = RectangularHexMapShape::new(columns, rows);
        let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
        for coords in shape.coords() {
            tiles.insert(coords, HexMapTile::from_properties(10));
        }
        HexMap { tiles }
    }

    // TODO:
}
