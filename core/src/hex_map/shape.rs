//! Shape of an `HexMap`.

use super::coordinates::{
    CUBE_COORDS_CACHED_DIRECTIONS, CubeCoords, CubeCoordsScalar, OffsetCoords,
};

/// Rectangular map of pointy-top hexes, laid out with `OffsetCoords`, like in Civilization games:
/// - row 0 is the northernmost row (north pole), the last row the southernmost one (south pole);
/// - it wraps around east-west (the last column is adjacent to the first one), but not north-south.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RectangularHexMapShape {
    columns: CubeCoordsScalar,
    rows: CubeCoordsScalar,
}

impl RectangularHexMapShape {
    pub fn new(columns: CubeCoordsScalar, rows: CubeCoordsScalar) -> Self {
        assert!(columns > 0 && rows > 0, "columns > 0 && rows > 0");
        Self { columns, rows }
    }

    pub fn columns(&self) -> CubeCoordsScalar {
        self.columns
    }

    pub fn rows(&self) -> CubeCoordsScalar {
        self.rows
    }

    pub fn tiles_count(&self) -> usize {
        self.columns as usize * self.rows as usize
    }

    /// Iterate over all the tiles, row by row from north to south, each row from west to east.
    pub fn coords(&self) -> impl Iterator<Item = CubeCoords> {
        let (columns, rows) = (self.columns, self.rows);
        (0..rows).flat_map(move |row| {
            (0..columns).map(move |col| CubeCoords::from(OffsetCoords::new(col, row)))
        })
    }

    /// Bring the given coordinates back within the map by wrapping them around east-west.
    ///
    /// Returns `None` beyond the poles, since the map does not wrap around north-south.
    pub fn wrap(&self, coords: CubeCoords) -> Option<CubeCoords> {
        let offset = OffsetCoords::from(coords);
        if !(0..self.rows).contains(&offset.row()) {
            return None;
        }
        let col = offset.col().rem_euclid(self.columns);
        Some(OffsetCoords::new(col, offset.row()).into())
    }

    /// Iterate over the immediate neighbors of the given tile, wrapping around east-west.
    ///
    /// Tiles on the polar rows only have 4 neighbors. On maps narrower than 3 columns,
    /// wrapping around makes some neighbors coincide.
    pub fn neighbors(&self, coords: CubeCoords) -> impl Iterator<Item = CubeCoords> {
        let shape = *self;
        CUBE_COORDS_CACHED_DIRECTIONS
            .into_iter()
            .filter_map(move |direction| shape.wrap(coords + direction))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::hex_map::coordinates::{CubeCoords, OffsetCoords};

    use super::RectangularHexMapShape;

    fn offset_neighbors(
        shape: &RectangularHexMapShape,
        col: i16,
        row: i16,
    ) -> HashSet<OffsetCoords> {
        shape
            .neighbors(OffsetCoords::new(col, row).into())
            .map(OffsetCoords::from)
            .collect()
    }

    #[test]
    fn test_rectangular_shape_coords() {
        let shape = RectangularHexMapShape::new(5, 4);
        let coords: Vec<OffsetCoords> = shape.coords().map(OffsetCoords::from).collect();
        assert_eq!(coords.len(), shape.tiles_count());
        assert_eq!(coords.iter().collect::<HashSet<_>>().len(), coords.len());
        assert_eq!(coords[0], OffsetCoords::new(0, 0));
        assert_eq!(coords[1], OffsetCoords::new(1, 0));
        assert_eq!(coords[5], OffsetCoords::new(0, 1));
        assert_eq!(coords[19], OffsetCoords::new(4, 3));
    }

    #[test]
    fn test_rectangular_shape_wrap() {
        let shape = RectangularHexMapShape::new(5, 4);
        let wrap = |col, row| {
            shape
                .wrap(OffsetCoords::new(col, row).into())
                .map(OffsetCoords::from)
        };
        assert_eq!(wrap(2, 1), Some(OffsetCoords::new(2, 1)));
        assert_eq!(wrap(5, 1), Some(OffsetCoords::new(0, 1)));
        assert_eq!(wrap(-1, 2), Some(OffsetCoords::new(4, 2)));
        assert_eq!(wrap(12, 3), Some(OffsetCoords::new(2, 3)));
        assert_eq!(wrap(2, -1), None);
        assert_eq!(wrap(2, 4), None);
    }

    #[test]
    fn test_rectangular_shape_neighbors_wrap_around_east_west() {
        // odd column count on purpose: pointy-top maps can wrap around with any
        let shape = RectangularHexMapShape::new(5, 4);
        let expected = |coords: &[(i16, i16)]| -> HashSet<OffsetCoords> {
            coords
                .iter()
                .map(|&(col, row)| OffsetCoords::new(col, row))
                .collect()
        };

        // interior, even row
        assert_eq!(
            offset_neighbors(&shape, 2, 2),
            expected(&[(3, 2), (2, 1), (1, 1), (1, 2), (1, 3), (2, 3)])
        );
        // eastern edge, odd row
        assert_eq!(
            offset_neighbors(&shape, 4, 1),
            expected(&[(0, 1), (0, 0), (4, 0), (3, 1), (4, 2), (0, 2)])
        );
        // western edge, even row
        assert_eq!(
            offset_neighbors(&shape, 0, 2),
            expected(&[(1, 2), (0, 1), (4, 1), (4, 2), (4, 3), (0, 3)])
        );
        // north pole
        assert_eq!(
            offset_neighbors(&shape, 0, 0),
            expected(&[(1, 0), (4, 0), (4, 1), (0, 1)])
        );
        // south pole
        assert_eq!(
            offset_neighbors(&shape, 4, 3),
            expected(&[(0, 3), (3, 3), (4, 2), (0, 2)])
        );
    }

    #[test]
    fn test_rectangular_shape_neighbors_are_symmetric() {
        let shape = RectangularHexMapShape::new(5, 4);
        for coords in shape.coords() {
            let row = OffsetCoords::from(coords).row();
            let neighbors: Vec<CubeCoords> = shape.neighbors(coords).collect();
            let polar = row == 0 || row == shape.rows() - 1;
            assert_eq!(neighbors.len(), if polar { 4 } else { 6 }, "{coords:?}");
            for neighbor in neighbors {
                assert!(shape.wrap(neighbor) == Some(neighbor));
                assert!(shape.neighbors(neighbor).any(|n| n == coords));
            }
        }
    }
}
