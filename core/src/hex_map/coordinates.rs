//! TODO:
//!
//! See: https://www.redblobgames.com/grids/hexagons/#coordinates

use std::{
    hash::{Hash, Hasher},
    ops::{Add, Sub},
};

/// TODO:
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HexMapDirection {
    NW = 0,
}

/// TODO: for computation results (distance etc)
pub type HexMapCoordinatesCommonComputeScalar = f64;

/// TODO:
///
/// TODO: `From<HexMapDirection` **relative** direction
pub trait HexMapCoordinatesSystem:
    Sized + Copy + Add<Output = Self> + Sub<Output = Self> + From<HexMapDirection>
{
    fn length(&self) -> HexMapCoordinatesCommonComputeScalar;

    /// Compute the distance to the given coordinates.
    fn distance_to(self, to: Self) -> HexMapCoordinatesCommonComputeScalar {
        let delta: Self = self - to;
        Self::length(&delta)
    }

    /// Compute the coordinates of the immediate neighbor in the given direction.
    fn neighbor<D: Into<Self>>(&self, direction: HexMapDirection) -> Self {
        *self + direction.into()
    }

    // TODO: neighbors returning an iterator?
}

/// Scalar used for storage and computations for `CubeCoords`.
///
/// This is used instead of generics to avoid extra complexity, which would affect
/// the usage of `HexMapCubeCoordinates` everywhere in the system due to abstraction leakage.
///
/// It is a `u16`, which should be more than enough, even in computations.
pub type CubeCoordsScalar = i16;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CubeCoords {
    q: CubeCoordsScalar,
    r: CubeCoordsScalar,
    s: CubeCoordsScalar,
}

/// TODO:
///
/// TODO: no `Result::Err` script etc
/// TODO: not checking map bounds here at any point
impl CubeCoords {
    pub fn from_cube_coords(q: CubeCoordsScalar, r: CubeCoordsScalar, s: CubeCoordsScalar) -> Self {
        // this is a fundamental invariant, so no "debug_assert"
        assert!(q + r + s == 0, "q + r + s == 0");
        Self { q, r, s }
    }

    pub fn from_axial_coords(q: CubeCoordsScalar, r: CubeCoordsScalar) -> Self {
        // since q+r+s = 0
        let s = 0 - q - r;
        // this is a fundamental invariant, so no "debug_assert" nor "Result:Err"
        assert!(q + r + s == 0, "q + r + s == 0");
        Self { q, r, s }
    }
}

/// TODO: same order as `HexMapDirection`
pub const CUBE_COORDS_CACHED_DIRECTIONS: [CubeCoords; 6] = [
    CubeCoords { q: 1, r: 0, s: -1 },
    CubeCoords { q: 1, r: -1, s: 0 },
    CubeCoords { q: 0, r: -1, s: 1 },
    CubeCoords { q: -1, r: 0, s: 1 },
    CubeCoords { q: -1, r: 1, s: 0 },
    CubeCoords { q: 0, r: 1, s: -1 },
];

impl From<HexMapDirection> for CubeCoords {
    fn from(value: HexMapDirection) -> Self {
        CUBE_COORDS_CACHED_DIRECTIONS[value as usize]
    }
}

impl Add for CubeCoords {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            q: self.q + rhs.q,
            r: self.q + rhs.q,
            s: self.q + rhs.q,
        }
    }
}

impl Sub for CubeCoords {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            q: self.q - rhs.q,
            r: self.q - rhs.q,
            s: self.q - rhs.q,
        }
    }
}

/// Needed for storage in `HexMap`.
impl Hash for CubeCoords {
    fn hash<H: Hasher>(&self, state: &mut H) {
        todo!()
    }
}

impl HexMapCoordinatesSystem for CubeCoords {
    fn length(&self) -> HexMapCoordinatesCommonComputeScalar {
        (self.q.abs_diff(0) + self.r.abs_diff(0) + self.s.abs_diff(0)) as f64 / 2f64
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum HexMapCoordinates {
    Cube(CubeCoords),
    Offset(),
    Doubled(),
}

impl HexMapCoordinates {
    pub fn distance_from(&self, other: &Self) -> HexMapCoordinatesCommonComputeScalar {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::hex_map::coordinates::HexMapCoordinatesSystem;

    use super::CUBE_COORDS_CACHED_DIRECTIONS;

    #[test]
    fn test_cube_coords_directions_cache() {
        for relative_coords in CUBE_COORDS_CACHED_DIRECTIONS {
            assert_eq!(relative_coords.q + relative_coords.r + relative_coords.s, 0);
            assert_eq!(relative_coords.length(), 1f64);
        }
    }

    // TODO: more tests
}
