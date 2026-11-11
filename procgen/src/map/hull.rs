//! The convex hull of a group of tiles: how filled in a continent is.

/// The tiles whose centers lie in the convex hull of the given tiles' centers, the given ones
/// among them, in row-major order; `None` for tiles that go all the way around the world.
///
/// `tiles` are row-major indices on a map of `columns` columns that wraps around east-west, in
/// row-major order. The hull is drawn on the map unrolled from the widest band of columns the
/// tiles leave empty, each tile at its true position: odd rows half a tile east, rows `√3 / 2`
/// apart.
pub(crate) fn hull_tiles(columns: usize, tiles: &[usize]) -> Option<Vec<usize>> {
    let (Some(&first), Some(&last)) = (tiles.first(), tiles.last()) else {
        return Some(Vec::new());
    };
    let mut used = vec![false; columns];
    for &index in tiles {
        used[index % columns] = true;
    }
    // the unrolled map starts right after the widest gap
    let (mut widest, mut gap_end, mut run) = (0, None, 0);
    for step in 0..2 * columns {
        if used[step % columns] {
            run = 0;
        } else {
            run += 1;
            if run > widest {
                (widest, gap_end) = (run, Some(step % columns));
            }
        }
    }
    let origin = gap_end? + 1;
    let position = |row: usize, unrolled: usize| {
        (
            unrolled as f64 + 0.5 * (row & 1) as f64,
            row as f64 * 0.75f64.sqrt(),
        )
    };
    let unroll = |col: usize| (col + columns - origin % columns) % columns;
    let mut points: Vec<(f64, f64)> = tiles
        .iter()
        .map(|&index| position(index / columns, unroll(index % columns)))
        .collect();
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let hull = convex_hull(&points);
    Some(
        (first / columns..=last / columns)
            .flat_map(|row| (0..columns).map(move |col| (row, col)))
            .filter(|&(row, col)| inside(&hull, position(row, unroll(col))))
            .map(|(row, col)| row * columns + col)
            .collect(),
    )
}

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// The convex hull of points sorted by x then y, counterclockwise: Andrew's monotone chain.
fn convex_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let chain = |points: &mut dyn Iterator<Item = &(f64, f64)>| {
        let mut half: Vec<(f64, f64)> = Vec::new();
        for &point in points {
            while half.len() >= 2 && cross(half[half.len() - 2], half[half.len() - 1], point) <= 0.0
            {
                half.pop();
            }
            half.push(point);
        }
        half.pop();
        half
    };
    let mut hull = chain(&mut points.iter());
    hull.extend(chain(&mut points.iter().rev()));
    hull
}

/// Whether a point lies in a counterclockwise convex polygon, its edges included. A polygon of
/// fewer than three corners holds its corners and the segment between them.
fn inside(polygon: &[(f64, f64)], point: (f64, f64)) -> bool {
    const EPSILON: f64 = 1e-9;
    match polygon {
        [] => false,
        [corner] => (corner.0 - point.0).abs() < EPSILON && (corner.1 - point.1).abs() < EPSILON,
        [a, b] => {
            cross(*a, *b, point).abs() < EPSILON
                && point.0 >= a.0.min(b.0) - EPSILON
                && point.0 <= a.0.max(b.0) + EPSILON
                && point.1 >= a.1.min(b.1) - EPSILON
                && point.1 <= a.1.max(b.1) + EPSILON
        }
        _ => (0..polygon.len())
            .all(|k| cross(polygon[k], polygon[(k + 1) % polygon.len()], point) >= -EPSILON),
    }
}

#[cfg(test)]
mod tests {
    use super::hull_tiles;

    #[test]
    fn test_the_hull_of_a_crescent_holds_its_bay() {
        // on 8 columns: a C of land around a bay of two tiles, at (3, 1) and (3, 2)
        let columns = 8;
        let at = |col: usize, row: usize| row * columns + col;
        let tiles = vec![
            at(2, 0),
            at(3, 0),
            at(4, 0),
            at(2, 1),
            at(2, 2),
            at(2, 3),
            at(3, 3),
            at(4, 3),
        ];
        let hull = hull_tiles(columns, &tiles).expect("a hull");
        for tile in &tiles {
            assert!(hull.contains(tile), "{tile}");
        }
        assert!(
            hull.contains(&at(3, 1)) && hull.contains(&at(3, 2)),
            "{hull:?}"
        );
        assert!(!hull.contains(&at(6, 1)));
    }

    #[test]
    fn test_a_hull_unrolls_across_the_seam() {
        let columns = 10;
        // three tiles in a row across the seam: columns 9, 0 and 1
        let hull = hull_tiles(columns, &[0, 1, 9]).expect("a hull");
        assert_eq!(hull, [0, 1, 9]);
        // a full row goes all the way around: no hull
        assert_eq!(hull_tiles(columns, &(0..10).collect::<Vec<_>>()), None);
        assert_eq!(hull_tiles(columns, &[]), Some(Vec::new()));
    }
}
