//! The planet as the nations generator reads it: elevations and terrain, indexed like the
//! client's buffers.

use std::collections::VecDeque;

use project_x_core::{
    game::state::TileIndex,
    hex_map::{HexMap, coordinates::CubeCoords, shape::RectangularHexMapShape},
};

use crate::map::{MapGeneratorSettings, TerrainClassifier, TerrainKind};

/// A generated planet, classified, with its tiles in the row-major order of
/// `RectangularHexMapShape::coords`.
#[derive(Clone, Debug)]
pub struct PlanetSurface {
    shape: RectangularHexMapShape,
    /// In meters.
    elevations: Vec<i16>,
    terrain: Vec<TerrainKind>,
    /// Sea level in meters: what tells the ice over land from the ice over the sea.
    sea_level: f64,
}

impl PlanetSurface {
    /// Classify a map generated with the given settings.
    pub fn classify(map: &HexMap, settings: &MapGeneratorSettings) -> Self {
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        let classifier = TerrainClassifier::new(settings);
        let (scale, bias) = settings.get_elevation_scale_bias();
        let mut elevations = Vec::with_capacity(shape.tiles_count());
        let mut terrain = Vec::with_capacity(shape.tiles_count());
        for coords in shape.coords() {
            let elevation = map.tile(&coords).expect("missing tile").elevation();
            elevations.push(elevation);
            terrain.push(classifier.classify(coords.into(), elevation));
        }
        Self::new(
            shape,
            elevations,
            terrain,
            settings.get_sea_level().mul_add(scale, bias),
        )
    }

    /// From an already classified map, like the WASM bridge holds.
    ///
    /// `sea_level` is in meters: the elevation at which the shore lies.
    pub fn new(
        shape: RectangularHexMapShape,
        elevations: Vec<i16>,
        terrain: Vec<TerrainKind>,
        sea_level: f64,
    ) -> Self {
        assert_eq!(
            elevations.len(),
            shape.tiles_count(),
            "one elevation per tile"
        );
        assert_eq!(terrain.len(), shape.tiles_count(), "one terrain per tile");
        Self {
            shape,
            elevations,
            terrain,
            sea_level,
        }
    }

    pub fn shape(&self) -> RectangularHexMapShape {
        self.shape
    }

    pub fn tiles_count(&self) -> usize {
        self.shape.tiles_count()
    }

    pub fn terrain(&self) -> &[TerrainKind] {
        &self.terrain
    }

    pub fn terrain_at(&self, index: TileIndex) -> TerrainKind {
        self.terrain[index as usize]
    }

    /// In meters.
    pub fn elevation_at(&self, index: TileIndex) -> i16 {
        self.elevations[index as usize]
    }

    /// Whether the tile is above the sea: the polar ice counts when it caps land, not when it
    /// caps the sea, so that the ice does not bridge the continents.
    pub fn is_land(&self, index: TileIndex) -> bool {
        match self.terrain_at(index) {
            TerrainKind::DeepWater | TerrainKind::ShallowWater => false,
            TerrainKind::Flat | TerrainKind::Hills | TerrainKind::Mountains => true,
            TerrainKind::Ice | TerrainKind::RoughIce => {
                f64::from(self.elevation_at(index)) >= self.sea_level
            }
        }
    }

    /// Whether the tile is land adjacent to the sea.
    pub fn is_coastal(&self, index: TileIndex) -> bool {
        self.is_land(index)
            && self
                .neighbors(index)
                .any(|neighbor| !self.is_land(neighbor))
    }

    /// Latitude from -1.0 (the north pole) to +1.0 (the south pole).
    pub fn latitude(&self, index: TileIndex) -> f64 {
        let rows = self.shape.rows() as u32;
        let row = index / self.shape.columns() as u32;
        if rows == 1 {
            0.0
        } else {
            f64::from(row) / f64::from(rows - 1) * 2.0 - 1.0
        }
    }

    /// Row-major index of a tile, wrapped around east-west, or `None` beyond the poles.
    pub fn index(&self, coords: CubeCoords) -> Option<TileIndex> {
        let offset =
            project_x_core::hex_map::coordinates::OffsetCoords::from(self.shape.wrap(coords)?);
        Some(
            offset.row() as TileIndex * self.shape.columns() as TileIndex
                + offset.col() as TileIndex,
        )
    }

    pub fn coords(&self, index: TileIndex) -> CubeCoords {
        let columns = self.shape.columns() as TileIndex;
        project_x_core::hex_map::coordinates::OffsetCoords::new(
            (index % columns) as i16,
            (index / columns) as i16,
        )
        .into()
    }

    /// Indices of the neighbors of a tile, wrapping around east-west.
    pub fn neighbors(&self, index: TileIndex) -> impl Iterator<Item = TileIndex> + '_ {
        self.shape
            .neighbors(self.coords(index))
            .filter_map(|coords| self.index(coords))
    }

    /// Every tile within `radius` steps of `center`, the center first, each one paired with its
    /// distance. Wraps around east-west and goes over water: this is hex distance.
    pub fn within(&self, center: TileIndex, radius: u32) -> Vec<(TileIndex, u32)> {
        let mut seen = vec![false; self.tiles_count()];
        seen[center as usize] = true;
        let mut tiles = vec![(center, 0)];
        let mut queue = VecDeque::from([(center, 0)]);
        while let Some((index, distance)) = queue.pop_front() {
            if distance == radius {
                continue;
            }
            for neighbor in self.neighbors(index) {
                if !seen[neighbor as usize] {
                    seen[neighbor as usize] = true;
                    tiles.push((neighbor, distance + 1));
                    queue.push_back((neighbor, distance + 1));
                }
            }
        }
        tiles
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use project_x_core::hex_map::shape::RectangularHexMapShape;

    use crate::map::TerrainKind;

    use super::PlanetSurface;

    /// A surface drawn with one character per tile, rows separated by `|`:
    /// `~` deep water, `-` shallow water, `.` flat, `:` hills, `^` mountains,
    /// `*` ice over land, `_` ice over the sea.
    pub(crate) fn surface_from_art(art: &str) -> PlanetSurface {
        let rows: Vec<&str> = art.split('|').collect();
        let columns = rows[0].len();
        assert!(rows.iter().all(|row| row.len() == columns), "ragged rows");
        let mut elevations = Vec::new();
        let mut terrain = Vec::new();
        for row in &rows {
            for glyph in row.chars() {
                let (elevation, kind) = match glyph {
                    '~' => (-2000, TerrainKind::DeepWater),
                    '-' => (-50, TerrainKind::ShallowWater),
                    '.' => (200, TerrainKind::Flat),
                    ':' => (1200, TerrainKind::Hills),
                    '^' => (2500, TerrainKind::Mountains),
                    '*' => (100, TerrainKind::Ice),
                    '_' => (-1000, TerrainKind::Ice),
                    other => panic!("unknown glyph {other:?}"),
                };
                elevations.push(elevation);
                terrain.push(kind);
            }
        }
        PlanetSurface::new(
            RectangularHexMapShape::new(columns as i16, rows.len() as i16),
            elevations,
            terrain,
            0.0,
        )
    }

    #[test]
    fn test_surface_tells_ice_over_land_from_ice_over_the_sea() {
        let surface = surface_from_art("*_|.~");
        assert!(surface.is_land(0));
        assert!(!surface.is_land(1));
        assert!(surface.is_land(2));
        assert!(!surface.is_land(3));
    }

    #[test]
    fn test_surface_coastal_tiles_touch_the_sea() {
        let surface = surface_from_art("....|.~..|....");
        // the tiles around the lone water tile, odd row 1 shoved east
        assert!(surface.is_coastal(1));
        assert!(surface.is_coastal(4));
        assert!(surface.is_coastal(6));
        assert!(!surface.is_coastal(3));
        assert!(!surface.is_coastal(5), "the water itself");
    }

    #[test]
    fn test_surface_latitude_runs_from_pole_to_pole() {
        let surface = surface_from_art("..|..|..|..|..");
        assert_eq!(surface.latitude(0), -1.0);
        assert_eq!(surface.latitude(4), 0.0);
        assert_eq!(surface.latitude(9), 1.0);
    }

    #[test]
    fn test_surface_within_wraps_east_west_but_not_over_the_poles() {
        let surface = surface_from_art(".....|.....|.....");
        let within: Vec<_> = surface.within(5, 1).into_iter().collect();
        let mut tiles: Vec<u32> = within.iter().map(|&(index, _)| index).collect();
        tiles.sort_unstable();
        // row 1 is odd: its neighbors to the north and south are columns 0 and 1,
        // and column 4 wraps around to its west
        assert_eq!(tiles, [0, 1, 5, 6, 9, 10, 11]);
        assert_eq!(within[0], (5, 0));
        assert!(within[1..].iter().all(|&(_, distance)| distance == 1));

        let polar: Vec<u32> = surface.within(0, 1).into_iter().map(|(i, _)| i).collect();
        assert_eq!(polar.len(), 5, "4 neighbors on a polar row");
    }
}
