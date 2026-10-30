//! How far each nation's influence reaches, and what it owns on turn 1.

use std::{cmp::Reverse, collections::BinaryHeap};

use project_x_core::{game::state::TileIndex, simulation::ids::NationId};

use super::{capitals::Capital, settings::NationsGeneratorSettings, surface::PlanetSurface};

/// The spheres of influence: every land tile a capital can reach over land belongs to the
/// capital that reaches it the cheapest.
#[derive(Clone, Debug)]
pub struct Spheres {
    /// Row-major; `None` over water, and on land no capital reaches.
    owner: Vec<Option<NationId>>,
    /// The cost at which the owner reached the tile, meaningless where there is none.
    cost: Vec<u32>,
}

impl Spheres {
    /// One Dijkstra from every capital at once, each step costing what the land entered costs,
    /// water never entered. Ties go to the lower nation.
    pub fn carve(
        surface: &PlanetSurface,
        capitals: &[Capital],
        settings: &NationsGeneratorSettings,
    ) -> Self {
        let costs = settings.get_movement_costs();
        let max_cost = settings.get_max_sphere_cost().unwrap_or(u32::MAX);
        let mut owner = vec![None; surface.tiles_count()];
        let mut cost = vec![u32::MAX; surface.tiles_count()];

        // (cost, nation, tile): the cheapest first, then the lower nation
        let mut heap = BinaryHeap::new();
        for capital in capitals {
            heap.push(Reverse((0, capital.nation, capital.tile)));
        }
        while let Some(Reverse((reached_at, nation, tile))) = heap.pop() {
            if owner[tile as usize].is_some() {
                continue;
            }
            owner[tile as usize] = Some(nation);
            cost[tile as usize] = reached_at;
            for neighbor in surface.neighbors(tile) {
                if owner[neighbor as usize].is_some() {
                    continue;
                }
                let Some(step) = costs.of(surface.terrain_at(neighbor)) else {
                    continue;
                };
                if !surface.is_land(neighbor) {
                    continue;
                }
                let total = reached_at.saturating_add(step);
                if total <= max_cost && total < cost[neighbor as usize] {
                    cost[neighbor as usize] = total;
                    heap.push(Reverse((total, nation, neighbor)));
                }
            }
        }
        Self { owner, cost }
    }

    pub fn owner(&self, index: TileIndex) -> Option<NationId> {
        self.owner[index as usize]
    }

    pub fn owners(&self) -> &[Option<NationId>] {
        &self.owner
    }

    /// The cost at which its owner reached the tile, if any.
    pub fn cost(&self, index: TileIndex) -> Option<u32> {
        self.owner(index).map(|_| self.cost[index as usize])
    }

    /// The tiles of a nation's sphere, in row-major order.
    pub fn tiles_of(&self, nation: NationId) -> Vec<TileIndex> {
        (0..self.owner.len() as TileIndex)
            .filter(|&index| self.owner(index) == Some(nation))
            .collect()
    }
}

/// The disc a nation owns around its capital on turn 1: within the border radius, on land, and
/// within its own sphere, in row-major order.
pub fn starting_territory(
    surface: &PlanetSurface,
    spheres: &Spheres,
    capital: &Capital,
    settings: &NationsGeneratorSettings,
) -> Vec<TileIndex> {
    let mut territory: Vec<TileIndex> = surface
        .within(capital.tile, settings.get_border_radius())
        .into_iter()
        .map(|(index, _)| index)
        .filter(|&index| surface.is_land(index) && spheres.owner(index) == Some(capital.nation))
        .collect();
    territory.sort_unstable();
    territory
}

#[cfg(test)]
mod tests {
    use project_x_core::simulation::ids::{EntityIndex, NationId};

    use super::{Spheres, starting_territory};
    use crate::nations::{
        capitals::Capital, landmasses::Landmasses, settings::NationsGeneratorSettings,
        surface::tests::surface_from_art,
    };

    fn capital(
        nation: usize,
        tile: u32,
        surface: &crate::nations::surface::PlanetSurface,
    ) -> Capital {
        Capital {
            nation: NationId::from_index(nation),
            tile,
            landmass: Landmasses::find(surface).of(tile).unwrap(),
            site_score: 1.0,
        }
    }

    #[test]
    fn test_spheres_split_a_plain_between_two_capitals() {
        let surface = surface_from_art(".........|.........|.........");
        let settings = NationsGeneratorSettings::default();
        let capitals = [capital(0, 9, &surface), capital(1, 17, &surface)];
        let spheres = Spheres::carve(&surface, &capitals, &settings);
        assert_eq!(spheres.owner(9), Some(NationId::from_index(0)));
        assert_eq!(spheres.owner(17), Some(NationId::from_index(1)));
        assert_eq!(spheres.owner(10), Some(NationId::from_index(0)));
        assert_eq!(spheres.owner(16), Some(NationId::from_index(1)));
        assert!(
            spheres.owners().iter().all(Option::is_some),
            "every tile is land"
        );
        // the exact middle of an odd row goes to the lower nation
        assert_eq!(spheres.owner(13), Some(NationId::from_index(0)));
        assert_eq!(spheres.cost(9), Some(0));
        assert_eq!(spheres.cost(10), Some(1));
    }

    #[test]
    fn test_spheres_never_cross_the_sea() {
        // an island to the east, water on both sides so that nothing wraps around
        let surface = surface_from_art("...~..~|...~..~|...~..~");
        let settings = NationsGeneratorSettings::default();
        let capitals = [capital(0, 8, &surface)];
        let spheres = Spheres::carve(&surface, &capitals, &settings);
        for row in 0..3 {
            for col in 0..7 {
                let index = row * 7 + col;
                let expected = if col < 3 {
                    Some(NationId::from_index(0))
                } else {
                    None
                };
                assert_eq!(spheres.owner(index), expected, "({col}, {row})");
            }
        }
    }

    #[test]
    fn test_spheres_follow_the_terrain_cost() {
        // a mountain range right next to the first capital, water keeping the plain from
        // wrapping around
        let surface = surface_from_art(".^.......~|.^.......~|.^.......~");
        let settings = NationsGeneratorSettings::default();
        let capitals = [capital(0, 10, &surface), capital(1, 17, &surface)];
        let spheres = Spheres::carve(&surface, &capitals, &settings);
        // over the range costs 5 then 1: the second nation reaches the tile past it in 5
        assert_eq!(spheres.owner(12), Some(NationId::from_index(1)));
        assert_eq!(
            spheres.owner(11),
            Some(NationId::from_index(0)),
            "the range itself"
        );
    }

    #[test]
    fn test_spheres_stop_at_the_maximum_cost() {
        let surface = surface_from_art(".........");
        let settings = NationsGeneratorSettings::default().with_max_sphere_cost(Some(2));
        let capitals = [capital(0, 0, &surface)];
        let spheres = Spheres::carve(&surface, &capitals, &settings);
        assert_eq!(spheres.tiles_of(NationId::from_index(0)), [0, 1, 2, 7, 8]);
    }

    #[test]
    fn test_starting_territory_is_the_disc_within_the_sphere_on_land() {
        let surface = surface_from_art(".....~|.....~|.....~|.....~|.....~");
        let settings = NationsGeneratorSettings::default().with_border_radius(1);
        let capitals = [capital(0, 13, &surface), capital(1, 14, &surface)];
        let spheres = Spheres::carve(&surface, &capitals, &settings);
        let territory = starting_territory(&surface, &spheres, &capitals[0], &settings);
        assert!(territory.contains(&13));
        assert!(!territory.contains(&14), "the other capital");
        assert!(territory.iter().all(|&index| surface.is_land(index)));
        assert!(
            territory
                .iter()
                .all(|&index| spheres.owner(index) == Some(NationId::from_index(0)))
        );
        assert!(
            territory.windows(2).all(|pair| pair[0] < pair[1]),
            "row-major"
        );
    }
}
