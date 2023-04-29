use std::{collections::HashMap, fmt};

use self::{coordinates::CubeCoords, tile::HexMapTile};

pub mod coordinates;
pub mod layers;
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

// TODO: dont forget preaollocation from size map
pub struct HexMap {
    tiles: HexMapStorage,
}

impl fmt::Debug for HexMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::hex_map::{
        coordinates::{CubeCoords, CubeCoordsScalar},
        HexMap, HexMapStorage,
    };

    fn generate_small_rectangular_hashmap(
        left: CubeCoordsScalar,
        right: CubeCoordsScalar,
        top: CubeCoordsScalar,
        bottom: CubeCoordsScalar,
    ) -> HexMap {
        assert!(left < right);
        assert!(top < bottom);
        let mut tiles = HexMapStorage::new();
        for q in left..=right {
            let q_offset = q >> 1;
            for r in (top - q_offset)..=(bottom - q_offset) {
                tiles.insert(CubeCoords::from_axial_coords(q, r), todo!());
            }
        }
        HexMap { tiles }
    }

    // TODO:
}
