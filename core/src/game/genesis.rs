//! How the world was generated: provenance the simulation never changes.
//!
//! Kept apart from what is built on the tiles - a sphere of influence is where a nation was *meant*
//! to grow when the planet was settled, not what it owns - and read by the projection, for the
//! client's political map, and by whatever plans a nation's expansion. A world nobody generated,
//! such as a test's, has an empty genesis and no sphere anywhere.

use crate::{
    game::state::TileIndex,
    hex_map::coordinates::CubeCoords,
    simulation::ids::{EntityIndex, NationId},
};

/// Where a nation started.
#[derive(Clone, Debug, PartialEq)]
pub struct Homeland {
    nation: NationId,
    capital: CubeCoords,
    /// Habitability of the capital's site, as the generator scored it.
    site_score: f64,
}

impl Homeland {
    pub fn new(nation: NationId, capital: CubeCoords, site_score: f64) -> Self {
        Self {
            nation,
            capital,
            site_score,
        }
    }

    pub fn nation(&self) -> NationId {
        self.nation
    }

    pub fn capital(&self) -> CubeCoords {
        self.capital
    }

    pub fn site_score(&self) -> f64 {
        self.site_score
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldGenesis {
    /// Nation whose sphere of influence holds each tile, row-major; empty when nothing was
    /// generated.
    tile_sphere: Vec<Option<NationId>>,
    /// In `NationId` order.
    homelands: Vec<Homeland>,
}

impl WorldGenesis {
    pub fn new(tile_sphere: Vec<Option<NationId>>, homelands: Vec<Homeland>) -> Self {
        Self {
            tile_sphere,
            homelands,
        }
    }

    /// Whether nothing was generated.
    pub fn is_empty(&self) -> bool {
        self.tile_sphere.is_empty() && self.homelands.is_empty()
    }

    pub fn tiles_count(&self) -> usize {
        self.tile_sphere.len()
    }

    /// The nation whose sphere of influence holds the tile, if any.
    pub fn sphere_of(&self, index: TileIndex) -> Option<NationId> {
        self.tile_sphere.get(index as usize).copied().flatten()
    }

    pub fn homelands(&self) -> &[Homeland] {
        &self.homelands
    }

    pub fn homeland(&self, nation: NationId) -> Option<&Homeland> {
        self.homelands
            .get(nation.index())
            .filter(|homeland| homeland.nation == nation)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        hex_map::coordinates::OffsetCoords,
        simulation::ids::{EntityIndex, NationId},
    };

    use super::{Homeland, WorldGenesis};

    #[test]
    fn test_an_empty_genesis_has_no_sphere_anywhere() {
        let genesis = WorldGenesis::default();
        assert!(genesis.is_empty());
        assert_eq!(genesis.sphere_of(0), None);
        assert_eq!(genesis.homeland(NationId::from_index(0)), None);
    }

    #[test]
    fn test_a_genesis_answers_by_tile_and_by_nation() {
        let (first, second) = (NationId::from_index(0), NationId::from_index(1));
        let genesis = WorldGenesis::new(
            vec![Some(first), Some(first), None, Some(second)],
            vec![
                Homeland::new(first, OffsetCoords::new(0, 0).into(), 1.0),
                Homeland::new(second, OffsetCoords::new(1, 1).into(), 0.5),
            ],
        );
        assert!(!genesis.is_empty());
        assert_eq!(genesis.sphere_of(1), Some(first));
        assert_eq!(genesis.sphere_of(2), None);
        assert_eq!(genesis.sphere_of(3), Some(second));
        assert_eq!(genesis.sphere_of(99), None, "beyond the map");
        assert_eq!(
            genesis.homeland(second).map(Homeland::site_score),
            Some(0.5)
        );
        assert_eq!(genesis.homeland(NationId::from_index(2)), None);
    }
}
