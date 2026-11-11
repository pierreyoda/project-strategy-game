//! The shores of a planet, walked edge by edge: how straight they run.
//!
//! A coast drawn with a ruler - along a row of tiles, down a diagonal, in even stairs between the
//! two - reads as the work of a tool, not of the sea. The walk follows each shore around its
//! land, one tile edge after the other, and looks for the stretches that fit in a narrow strip:
//! a line at any angle does, whatever the stairs the tiles cut it into; a coast that wanders
//! does not.

use project_x_core::hex_map::{
    coordinates::{CUBE_COORDS_CACHED_DIRECTIONS, OffsetCoords},
    shape::RectangularHexMapShape,
};

/// What makes a stretch of shore straight.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Stretch {
    /// How long it runs at least, in tiles from end to end.
    pub length: f64,
    /// How far its edges stray from the line through them at most, in tiles.
    pub within: f64,
}

impl Stretch {
    /// A coast drawn with a ruler, eight tiles or more: the stairs of a line drawn on tiles stay
    /// within half a tile of it at every angle, a lone cape or cove does not.
    pub(crate) const RULER: Self = Self {
        length: 8.0,
        within: 0.5,
    };
    /// A coast that keeps to a line over fourteen tiles or more, whatever its capes and coves of
    /// a tile: the ruler shows through.
    pub(crate) const TREND: Self = Self {
        length: 14.0,
        within: 1.0,
    };
}

/// Where a neighbor lies, in the order of `CUBE_COORDS_CACHED_DIRECTIONS`: east, then
/// counterclockwise. In tiles, `y` growing southwards.
const STEPS: [(f64, f64); 6] = {
    const HALF: f64 = 0.5;
    const ROW: f64 = 0.866_025_403_784_438_6;
    [
        (1.0, 0.0),
        (HALF, -ROW),
        (-HALF, -ROW),
        (-1.0, 0.0),
        (-HALF, ROW),
        (HALF, ROW),
    ]
};

/// How straight the shores of a planet run.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct Straightness {
    /// Tile edges between the land and the sea.
    pub edges: usize,
    /// Those in a straight stretch.
    pub straight: usize,
    /// The longest straight stretch, in tiles from end to end; 0.0 without any.
    pub longest: f64,
}

impl Straightness {
    /// The share of the shore in straight stretches, `NaN` without a shore.
    pub(crate) fn share(&self) -> f64 {
        if self.edges == 0 {
            f64::NAN
        } else {
            self.straight as f64 / self.edges as f64
        }
    }

    /// The shores of the land of a map, its tiles in the order of `shape.coords()`. The edge of
    /// the map beyond the poles is no shore: a stretch stops there.
    pub(crate) fn of(shape: RectangularHexMapShape, land: &[bool], stretch: Stretch) -> Self {
        let mut straightness = Self {
            edges: 0,
            straight: 0,
            longest: 0.0,
        };
        for shore in shores(shape, land) {
            let (straight, longest) = shore.straight_edges(stretch);
            straightness.edges += shore.sea.iter().filter(|&&sea| sea).count();
            straightness.straight += straight.iter().filter(|&&straight| straight).count();
            straightness.longest = straightness.longest.max(longest);
        }
        straightness
    }
}

/// The land tiles with an edge on a straight stretch of shore.
pub(crate) fn straight_tiles(
    shape: RectangularHexMapShape,
    land: &[bool],
    stretch: Stretch,
) -> Vec<bool> {
    let mut tiles = vec![false; land.len()];
    for shore in shores(shape, land) {
        let (straight, _) = shore.straight_edges(stretch);
        for (&tile, _) in shore
            .tiles
            .iter()
            .zip(straight)
            .filter(|&(_, straight)| straight)
        {
            tiles[tile] = true;
        }
    }
    tiles
}

/// One shore, all the way around its land or its lake: the middle of each tile edge in turn, on
/// the map unrolled along the walk.
struct Shore {
    edges: Vec<(f64, f64)>,
    /// The land tile of each edge.
    tiles: Vec<usize>,
    /// Whether each edge has the sea beyond it, rather than the edge of the map.
    sea: Vec<bool>,
    /// How far a lap moves on the unrolled map: nothing, but for a shore that goes around the
    /// world.
    lap: (f64, f64),
}

impl Shore {
    /// The edge `step` edges into the walk, lap after lap.
    fn edge(&self, step: usize) -> (f64, f64) {
        let (x, y) = self.edges[step % self.edges.len()];
        let laps = (step / self.edges.len()) as f64;
        (x + laps * self.lap.0, y + laps * self.lap.1)
    }

    /// Whether each edge is on a straight stretch, and the length of the longest one.
    fn straight_edges(&self, stretch: Stretch) -> (Vec<bool>, f64) {
        let count = self.edges.len();
        let mut straight = vec![false; count];
        let mut longest: f64 = 0.0;
        for start in 0..count {
            // the longest stretch from here, grown an edge at a time for as long as it is straight
            let mut strip = Strip::default();
            let mut end = start;
            let mut found = None;
            while end - start < count && self.sea[end % count] {
                strip.push(self.edge(end));
                end += 1;
                let points = (start..end).map(|step| self.edge(step));
                let Some(length) = strip.length(points, stretch.within) else {
                    break;
                };
                if length >= stretch.length {
                    found = Some((end, length));
                }
            }
            if let Some((end, length)) = found {
                for step in start..end {
                    straight[step % count] = true;
                }
                longest = longest.max(length);
            }
        }
        (straight, longest)
    }
}

/// The line through a run of points: the sums its least-squares fit takes.
#[derive(Default)]
struct Strip {
    count: f64,
    x: f64,
    y: f64,
    xx: f64,
    yy: f64,
    xy: f64,
}

impl Strip {
    fn push(&mut self, (x, y): (f64, f64)) {
        self.count += 1.0;
        self.x += x;
        self.y += y;
        self.xx += x * x;
        self.yy += y * y;
        self.xy += x * y;
    }

    /// How long the points run along the line through them, when none strays from it by more
    /// than `within`; `None` otherwise. `points` are the points pushed.
    fn length(&self, points: impl Iterator<Item = (f64, f64)>, within: f64) -> Option<f64> {
        let (mean_x, mean_y) = (self.x / self.count, self.y / self.count);
        let spread_x = self.xx / self.count - mean_x * mean_x;
        let spread_y = self.yy / self.count - mean_y * mean_y;
        let together = self.xy / self.count - mean_x * mean_y;
        // the direction the points spread along the most
        let angle = 0.5 * (2.0 * together).atan2(spread_x - spread_y);
        let (along_x, along_y) = (angle.cos(), angle.sin());
        let (mut from, mut to) = (f64::INFINITY, f64::NEG_INFINITY);
        for (x, y) in points {
            let (x, y) = (x - mean_x, y - mean_y);
            if (x * along_y - y * along_x).abs() > within {
                return None;
            }
            let along = x * along_x + y * along_y;
            (from, to) = (from.min(along), to.max(along));
        }
        Some(to - from)
    }
}

/// Every shore of the land: each edge between a land tile and anything else belongs to one.
fn shores(shape: RectangularHexMapShape, land: &[bool]) -> Vec<Shore> {
    let columns = shape.columns() as usize;
    let neighbors: Vec<[Option<usize>; 6]> = shape
        .coords()
        .map(|coords| {
            CUBE_COORDS_CACHED_DIRECTIONS.map(|direction| {
                let neighbor: OffsetCoords = shape.wrap(coords + direction)?.into();
                Some(neighbor.row() as usize * columns + neighbor.col() as usize)
            })
        })
        .collect();
    let is_land = |tile: Option<usize>| tile.is_some_and(|tile| land[tile]);
    let mut walked = vec![false; land.len() * 6];
    let mut shores = Vec::new();
    for start in 0..land.len() * 6 {
        let (tile, direction) = (start / 6, start % 6);
        if walked[start] || !land[tile] || is_land(neighbors[tile][direction]) {
            continue;
        }
        // counterclockwise around the land, the sea on the right: at the end of an edge, on to the
        // next edge of the tile, or to the tile around the corner
        let mut shore = Shore {
            edges: Vec::new(),
            tiles: Vec::new(),
            sea: Vec::new(),
            lap: (0.0, 0.0),
        };
        let (mut tile, mut direction) = (tile, direction);
        let mut center = (0.0, 0.0);
        while !walked[tile * 6 + direction] {
            walked[tile * 6 + direction] = true;
            let (step_x, step_y) = STEPS[direction];
            shore
                .edges
                .push((center.0 + 0.5 * step_x, center.1 + 0.5 * step_y));
            shore.tiles.push(tile);
            shore.sea.push(neighbors[tile][direction].is_some());
            let next = (direction + 1) % 6;
            match neighbors[tile][next] {
                Some(around) if land[around] => {
                    center = (center.0 + STEPS[next].0, center.1 + STEPS[next].1);
                    (tile, direction) = (around, (direction + 5) % 6);
                }
                _ => direction = next,
            }
        }
        shore.lap = center;
        shores.push(shore);
    }
    shores
}

#[cfg(test)]
mod tests {
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use super::{Straightness, Stretch, shores};

    /// A land mask drawn one character per tile, rows separated by `|`: `#` land, `.` sea.
    fn mask(art: &str) -> (RectangularHexMapShape, Vec<bool>) {
        let rows: Vec<&str> = art.split('|').collect();
        let shape = RectangularHexMapShape::new(rows[0].len() as i16, rows.len() as i16);
        let land = rows
            .iter()
            .flat_map(|row| row.chars().map(|tile| tile == '#'))
            .collect();
        (shape, land)
    }

    #[test]
    fn test_every_edge_of_the_land_is_on_one_shore() {
        // an island of one tile, another of two, a ring of land around a lake
        let (shape, land) = mask(
            &[
                "............",
                ".#..........",
                "......###...",
                "..##..#.#...",
                "......###...",
                "............",
            ]
            .join("|"),
        );
        let mut lengths: Vec<usize> = shores(shape, &land)
            .iter()
            .map(|shore| shore.edges.len())
            .collect();
        lengths.sort_unstable();
        // the lake's six edges, the tile's six, the pair's ten, the ring's outer shore
        assert_eq!(lengths[..3], [6, 6, 10]);
        assert_eq!(lengths.len(), 4);
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert_eq!(straightness.edges, lengths.iter().sum::<usize>());
        assert_eq!(
            straightness.straight, 0,
            "nothing long enough to be straight"
        );
        assert_eq!(straightness.longest, 0.0);
    }

    #[test]
    fn test_a_coast_along_a_row_is_straight_all_the_way_around() {
        let (shape, land) = mask(
            &[
                ".".repeat(20),
                "#".repeat(20),
                "#".repeat(20),
                ".".repeat(20),
            ]
            .join("|"),
        );
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert_eq!(straightness.edges, 80);
        assert_eq!(straightness.share(), 1.0);
        assert!(straightness.longest > 19.0, "{}", straightness.longest);
    }

    #[test]
    fn test_the_edge_of_the_map_is_no_shore() {
        // land up to the north pole: only its south coast counts
        let (shape, land) = mask(&["#".repeat(20), "#".repeat(20), ".".repeat(20)].join("|"));
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert_eq!(straightness.edges, 40);
        assert_eq!(straightness.share(), 1.0);
    }

    #[test]
    fn test_a_coast_down_a_diagonal_or_in_stairs_is_straight() {
        // a parallelogram: two coasts along the rows, two down the diagonal
        let diagonal: Vec<String> = (0..18)
            .map(|row| match row {
                0 | 17 => ".".repeat(24),
                _ => format!(
                    "{}{}{}",
                    ".".repeat(2 + row / 2),
                    "#".repeat(10),
                    ".".repeat(12 - row / 2)
                ),
            })
            .collect();
        let (shape, land) = mask(&diagonal.join("|"));
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert!(straightness.share() > 0.9, "{straightness:?}");
        // stairs: a step east every two rows
        let stairs: Vec<String> = (0..18)
            .map(|row| match row {
                0 | 17 => ".".repeat(24),
                _ => format!(
                    "{}{}{}",
                    ".".repeat(2 + row / 4),
                    "#".repeat(10),
                    ".".repeat(12 - row / 4)
                ),
            })
            .collect();
        let (shape, land) = mask(&stairs.join("|"));
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert!(straightness.share() > 0.9, "{straightness:?}");
    }

    #[test]
    fn test_a_coast_that_wanders_is_not_straight() {
        // capes and coves of a tile or two, all around
        let (shape, land) = mask(
            &[
                "............................",
                "......##........#.....##....",
                "..#..###..#..#.###...###.#..",
                ".##########################.",
                "..#########################.",
                ".#########################..",
                ".##########################.",
                "..#.###...###.#..#..###..#..",
                "....##.....#........##......",
                "............................",
            ]
            .join("|"),
        );
        let straightness = Straightness::of(shape, &land, Stretch::RULER);
        assert!(straightness.edges > 100);
        assert!(straightness.share() < 0.1, "{straightness:?}");
    }
}
