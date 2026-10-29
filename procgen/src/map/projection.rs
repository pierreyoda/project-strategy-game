use std::f64::consts::PI;

use project_x_core::hex_map::{coordinates::OffsetCoords, shape::RectangularHexMapShape};

/// Projection of the hexes of a `RectangularHexMapShape` onto a cylinder of radius 1,
/// where the noise functions are sampled.
///
/// The angle wraps around east-west seamlessly. The height goes from the north pole
/// (lower bound) to the south pole, over an extent that keeps the hexes from being stretched.
/// With a circumference of 2π, like the equator of the unit sphere, the noise functions
/// (whose frequencies are in radians) keep the feature sizes they were designed for.
///
/// Same arithmetic as `noise::utils::CylinderMapBuilder`: even rows sample the very same points.
#[derive(Copy, Clone, Debug)]
pub struct CylinderProjection {
    angle_bounds: (f64, f64),
    height_bounds: (f64, f64),
    angle_step: f64,
    height_step: f64,
}

impl CylinderProjection {
    pub fn new(shape: &RectangularHexMapShape) -> Self {
        let columns = shape.columns() as f64;
        let rows = shape.rows() as f64;
        // pointy-top hexes are √3 apart horizontally and 1.5 vertically (for a size of 1)
        let height_extent = PI * 3f64.sqrt() * rows / columns;
        let angle_bounds = (-180.0, 180.0);
        let height_bounds = (-height_extent / 2.0, height_extent / 2.0);
        Self {
            angle_bounds,
            height_bounds,
            angle_step: (angle_bounds.1 - angle_bounds.0) / columns,
            height_step: (height_bounds.1 - height_bounds.0) / rows,
        }
    }

    /// In degrees.
    pub fn angle_bounds(&self) -> (f64, f64) {
        self.angle_bounds
    }

    pub fn height_bounds(&self) -> (f64, f64) {
        self.height_bounds
    }

    /// Point to sample for the center of the given hex.
    pub fn project(&self, coords: OffsetCoords) -> [f64; 3] {
        // odd rows are shoved half a hex to the east
        let x = coords.col() as f64 + 0.5 * (coords.row() & 1) as f64;
        let angle = self.angle_bounds.0 + self.angle_step * x;
        let height = self.height_bounds.0 + self.height_step * coords.row() as f64;
        [angle.to_radians().cos(), height, angle.to_radians().sin()]
    }
}

#[cfg(test)]
mod tests {
    use noise::{
        NoiseFn,
        utils::{CylinderMapBuilder, NoiseMapBuilder},
    };
    use project_x_core::hex_map::{coordinates::OffsetCoords, shape::RectangularHexMapShape};

    use super::CylinderProjection;

    /// Outputs one of the coordinates of the sampled point.
    struct Axis(usize);

    impl NoiseFn<f64, 3> for Axis {
        fn get(&self, point: [f64; 3]) -> f64 {
            point[self.0]
        }
    }

    fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
        a.iter()
            .zip(b)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt()
    }

    #[test]
    fn test_cylinder_projection_matches_cylinder_map_builder_on_even_rows() {
        let (columns, rows) = (80, 52);
        let projection = CylinderProjection::new(&RectangularHexMapShape::new(columns, rows));
        let (angle_bounds, height_bounds) = (projection.angle_bounds(), projection.height_bounds());
        let maps: Vec<_> = (0..3)
            .map(|axis| {
                CylinderMapBuilder::new(Axis(axis))
                    .set_size(columns as usize, rows as usize)
                    .set_angle_bounds(angle_bounds.0, angle_bounds.1)
                    .set_height_bounds(height_bounds.0, height_bounds.1)
                    .build()
            })
            .collect();

        for row in (0..rows).step_by(2) {
            for col in 0..columns {
                let point = projection.project(OffsetCoords::new(col, row));
                for (axis, map) in maps.iter().enumerate() {
                    let expected = map[(col as usize, row as usize)];
                    assert_eq!(point[axis].to_bits(), expected.to_bits(), "({col}, {row})");
                }
            }
        }
    }

    #[test]
    fn test_cylinder_projection_shoves_odd_rows_half_a_hex_east() {
        let projection = CylinderProjection::new(&RectangularHexMapShape::new(80, 52));
        let angle = |col, row| {
            let [x, _, z] = projection.project(OffsetCoords::new(col, row));
            z.atan2(x).to_degrees()
        };
        for col in 0..79 {
            let expected = (angle(col, 0) + angle(col + 1, 0)) / 2.0;
            assert!((angle(col, 1) - expected).abs() < 1e-9, "column {col}");
        }
    }

    #[test]
    fn test_cylinder_projection_does_not_stretch_hexes() {
        let projection = CylinderProjection::new(&RectangularHexMapShape::new(80, 52));
        let point = |col, row| projection.project(OffsetCoords::new(col, row));
        let east = distance(point(10, 20), point(11, 20));
        let south_east = distance(point(10, 20), point(10, 21));
        let south_west = distance(point(10, 20), point(9, 21));
        assert!(
            (south_east / east - 1.0).abs() < 1e-3,
            "{south_east} vs {east}"
        );
        assert!(
            (south_west / east - 1.0).abs() < 1e-3,
            "{south_west} vs {east}"
        );
    }
}
