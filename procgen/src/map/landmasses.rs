//! The continents and islands of a planet.

use project_x_core::game::state::TileIndex;

use super::surface::PlanetSurface;

/// Share of the land a landmass needs to count as a continent.
pub(crate) const CONTINENT_SHARE: f64 = 0.05;

/// A landmass, numbered from the north-west in row-major order.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LandmassId(u32);

impl LandmassId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The land tiles of a planet, grouped by connectivity: two land tiles are on the same landmass
/// when a path over land joins them, wrapping around east-west but not over the poles.
#[derive(Clone, Debug)]
pub struct Landmasses {
    /// Row-major; `None` over water.
    of_tile: Vec<Option<LandmassId>>,
    /// The tiles of each landmass, in row-major order.
    tiles: Vec<Vec<TileIndex>>,
}

impl Landmasses {
    pub fn find(surface: &PlanetSurface) -> Self {
        let mut of_tile = vec![None; surface.tiles_count()];
        let mut tiles = Vec::new();
        for start in 0..surface.tiles_count() as TileIndex {
            if of_tile[start as usize].is_some() || !surface.is_land(start) {
                continue;
            }
            let id = LandmassId(tiles.len() as u32);
            of_tile[start as usize] = Some(id);
            let mut landmass = Vec::new();
            let mut stack = vec![start];
            while let Some(index) = stack.pop() {
                landmass.push(index);
                for neighbor in surface.neighbors(index) {
                    if of_tile[neighbor as usize].is_none() && surface.is_land(neighbor) {
                        of_tile[neighbor as usize] = Some(id);
                        stack.push(neighbor);
                    }
                }
            }
            landmass.sort_unstable();
            tiles.push(landmass);
        }
        Self { of_tile, tiles }
    }

    pub fn len(&self) -> usize {
        self.tiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
    }

    pub fn ids(&self) -> impl Iterator<Item = LandmassId> {
        (0..self.tiles.len() as u32).map(LandmassId)
    }

    /// The landmass of a tile, or `None` over water.
    pub fn of(&self, index: TileIndex) -> Option<LandmassId> {
        self.of_tile[index as usize]
    }

    /// The tiles of a landmass, in row-major order.
    pub fn tiles(&self, id: LandmassId) -> &[TileIndex] {
        &self.tiles[id.index()]
    }

    pub fn area(&self, id: LandmassId) -> usize {
        self.tiles[id.index()].len()
    }

    /// Number of land tiles over the whole planet.
    pub fn land_tiles(&self) -> usize {
        self.tiles.iter().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::Landmasses;
    use crate::map::surface::tests::surface_from_art;

    #[test]
    fn test_landmasses_are_found_across_the_east_west_edge() {
        // one landmass broken by the edge, one island to the south
        let surface = surface_from_art("..~~.|.~~~.|~~~~~|~~.~~");
        let landmasses = Landmasses::find(&surface);
        assert_eq!(landmasses.len(), 2);
        assert_eq!(landmasses.of(0), landmasses.of(4));
        assert_eq!(landmasses.of(0), landmasses.of(9));
        assert_ne!(landmasses.of(0), landmasses.of(17));
        assert_eq!(landmasses.of(2), None);
        assert_eq!(landmasses.tiles(landmasses.of(0).unwrap()), [0, 1, 4, 5, 9]);
        assert_eq!(landmasses.land_tiles(), 6);
    }

    #[test]
    fn test_landmasses_do_not_join_over_the_poles() {
        let surface = surface_from_art(".~~|~~~|.~~");
        assert_eq!(Landmasses::find(&surface).len(), 2);
    }

    #[test]
    fn test_landmasses_are_not_bridged_by_sea_ice() {
        // ice over the sea between two shores, ice over land joining two others
        let surface = surface_from_art("~.__.|~~~~~|~.**.");
        let landmasses = Landmasses::find(&surface);
        assert_eq!(landmasses.len(), 3);
        assert_eq!(landmasses.of(11), landmasses.of(14));
        assert_ne!(landmasses.of(1), landmasses.of(4));
    }
}
