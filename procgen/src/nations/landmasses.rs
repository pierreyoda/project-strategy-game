//! The continents and islands of a planet, and how many nations each one gets.

use project_x_core::game::state::TileIndex;

use super::surface::PlanetSurface;

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

/// Share `nations` between landmasses in proportion to their weights, by largest remainder.
///
/// A landmass under `min_weight` gets nothing. Should none qualify, the heaviest one takes them
/// all: there is always somewhere to start, however poor.
pub fn apportion(weights: &[f64], nations: u16, min_weight: f64) -> Vec<u16> {
    let mut quotas = vec![0; weights.len()];
    if weights.is_empty() || nations == 0 {
        return quotas;
    }
    let eligible: Vec<usize> = (0..weights.len())
        .filter(|&i| weights[i] >= min_weight && weights[i] > 0.0)
        .collect();
    if eligible.is_empty() {
        let heaviest = (0..weights.len())
            .max_by(|&a, &b| weights[a].total_cmp(&weights[b]).then(b.cmp(&a)))
            .expect("non-empty weights");
        quotas[heaviest] = nations;
        return quotas;
    }

    let total: f64 = eligible.iter().map(|&i| weights[i]).sum();
    let mut given = 0;
    let mut remainders = Vec::with_capacity(eligible.len());
    for &i in &eligible {
        let exact = f64::from(nations) * weights[i] / total;
        let floor = exact.floor();
        quotas[i] = floor as u16;
        given += quotas[i];
        remainders.push((exact - floor, i));
    }
    // the largest remainders first, the first landmass on ties
    remainders.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, i) in remainders {
        if given == nations {
            break;
        }
        quotas[i] += 1;
        given += 1;
    }
    quotas
}

#[cfg(test)]
mod tests {
    use super::{Landmasses, apportion};
    use crate::nations::surface::tests::surface_from_art;

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

    #[test]
    fn test_apportionment_follows_the_weights_by_largest_remainder() {
        // 7 nations over weights 50/30/20: exact shares 3.5/2.1/1.4
        assert_eq!(apportion(&[50.0, 30.0, 20.0], 7, 0.0), [4, 2, 1]);
        // a tie on the remainders goes to the first landmass
        assert_eq!(apportion(&[10.0, 10.0, 10.0], 4, 0.0), [2, 1, 1]);
        assert_eq!(
            apportion(&[50.0, 30.0, 20.0], 7, 0.0).iter().sum::<u16>(),
            7
        );
    }

    #[test]
    fn test_apportionment_skips_landmasses_too_small_to_live_on() {
        assert_eq!(apportion(&[50.0, 5.0, 30.0], 4, 12.0), [3, 0, 1]);
        // nowhere good enough: the best of a bad lot takes everyone
        assert_eq!(apportion(&[5.0, 8.0, 3.0], 3, 12.0), [0, 3, 0]);
        assert_eq!(apportion(&[], 3, 12.0), []);
    }
}
