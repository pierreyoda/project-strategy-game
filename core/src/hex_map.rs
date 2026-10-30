use std::{collections::HashMap, fmt};

use self::{coordinates::CubeCoords, tile::HexMapTile};

pub mod coordinates;
pub mod layers;
pub mod shape;
pub mod terrain;
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

    pub fn tiles_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn tile(&self, coords: &CubeCoords) -> Option<&HexMapTile> {
        self.tiles.get(coords)
    }

    /// Mutable access to a tile.
    ///
    /// **NB** Nothing in the turn loop calls this directly: the simulation goes through
    /// `game::state::WorldMut`, which marks the tile dirty for the next projection. A write that
    /// skips that gate is a tile the client will not redraw.
    pub fn tile_mut(&mut self, coords: &CubeCoords) -> Option<&mut HexMapTile> {
        self.tiles.get_mut(coords)
    }
}

impl fmt::Debug for HexMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // the tiles themselves are far too many to print
        f.debug_struct("HexMap")
            .field("tiles_count", &self.tiles.len())
            .finish_non_exhaustive()
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

    #[test]
    fn test_hex_map_tile_access() {
        let mut map = generate_small_rectangular_hashmap(4, 3);
        assert_eq!(map.tiles_count(), 12);

        let coords = RectangularHexMapShape::new(4, 3).coords().nth(5).unwrap();
        assert_eq!(map.tile(&coords).unwrap().elevation(), 10);
        map.tile_mut(&coords)
            .unwrap()
            .artificial_mut()
            .set_owner(None);
        assert!(map.tile(&coords).unwrap().artificial().owner().is_none());
    }

    // TODO:
}
