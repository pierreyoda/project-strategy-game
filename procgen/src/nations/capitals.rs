//! Where each nation starts: the best site left on its landmass, no two capitals too close.

use project_x_core::{
    game::{
        rng::{GameRng, RngPurpose},
        state::TileIndex,
    },
    simulation::ids::{EntityIndex, NationId},
};
use rand::RngExt;

use super::{
    habitability::Habitability,
    landmasses::{LandmassId, Landmasses, apportion},
    settings::NationsGeneratorSettings,
    surface::PlanetSurface,
};

/// A capital, placed.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Capital {
    pub nation: NationId,
    pub tile: TileIndex,
    pub landmass: LandmassId,
    pub site_score: f64,
}

/// How the siting went, for tests and for the balance of a start.
#[derive(Clone, Debug, PartialEq)]
pub struct SitingReport {
    /// The spacing the siting started from.
    pub spacing_requested: u32,
    /// The spacing it ended with: lower when it had to relax to fit everyone.
    pub spacing_final: u32,
    /// How many times the spacing relaxed.
    pub relaxations: u32,
    /// Nations the planet had no room for at all.
    pub unplaced: u16,
}

/// Place the capitals.
///
/// Landmasses get their share of the nations by habitable weight, then each nation takes the best
/// site left on its landmass that keeps clear of the capitals already placed - the less crowded
/// the better - with a little randomness among the best few. When no site is clear, the spacing
/// relaxes down to its floor; when a landmass is full, the nation looks anywhere else; when the
/// planet is full, it goes unplaced and the report says so.
pub fn place_capitals(
    surface: &PlanetSurface,
    landmasses: &Landmasses,
    habitability: &Habitability,
    settings: &NationsGeneratorSettings,
) -> (Vec<Capital>, SitingReport) {
    let nations = settings.get_nations();
    let weights = habitability.landmass_weights(landmasses);
    let quotas = apportion(&weights, nations, settings.get_min_landmass_weight());

    let spacing_requested = settings.capital_spacing_for(landmasses.land_tiles());
    let mut siting = Siting {
        surface,
        landmasses,
        habitability,
        settings,
        rng: GameRng::new(settings.get_seed(), RngPurpose::WorldGeneration),
        nearest: vec![u32::MAX; surface.tiles_count()],
        capitals: Vec::with_capacity(nations as usize),
        spacing: spacing_requested,
        relaxations: 0,
    };

    // the landmasses in order, their nations one after the other: the first nations land on the
    // first landmass, which keeps the numbering stable across seeds that only move the capitals
    let mut homeless = Vec::new();
    for (id, &quota) in landmasses.ids().zip(&quotas) {
        for _ in 0..quota {
            if !siting.place(Some(id)) {
                homeless.push(id);
            }
        }
    }
    // a landmass that could not hold its share: anywhere else will do
    let mut unplaced = 0;
    for _ in homeless {
        if !siting.place(None) {
            unplaced += 1;
        }
    }

    let report = SitingReport {
        spacing_requested,
        spacing_final: siting.spacing,
        relaxations: siting.relaxations,
        unplaced,
    };
    (siting.capitals, report)
}

struct Siting<'a> {
    surface: &'a PlanetSurface,
    landmasses: &'a Landmasses,
    habitability: &'a Habitability,
    settings: &'a NationsGeneratorSettings,
    rng: GameRng,
    /// Distance to the nearest capital already placed, `u32::MAX` beyond the crowding distance.
    nearest: Vec<u32>,
    capitals: Vec<Capital>,
    spacing: u32,
    relaxations: u32,
}

impl Siting<'_> {
    /// Place the next nation on the given landmass, or anywhere, relaxing the spacing as needed.
    /// `false` when there is no room even at the floor.
    fn place(&mut self, landmass: Option<LandmassId>) -> bool {
        loop {
            if let Some(tile) = self.pick_site(landmass) {
                let nation = NationId::from_index(self.capitals.len());
                self.capitals.push(Capital {
                    nation,
                    tile,
                    landmass: landmass.unwrap_or_else(|| {
                        // a homeless nation: the landmass of wherever it ended up
                        self.landmass_of(tile)
                    }),
                    site_score: self.habitability.score(tile),
                });
                self.crowd_around(tile);
                return true;
            }
            if !self.relax() {
                return false;
            }
        }
    }

    /// Capitals closer than this crowd a site: twice the spacing, so that a nation prefers the
    /// middle of what is left over a seat right against its neighbor's fence.
    fn crowding_distance(&self) -> u32 {
        self.spacing.saturating_mul(2)
    }

    /// A site's score, weighed down by how crowded it is: halved against a capital at exactly the
    /// spacing, untouched beyond the crowding distance.
    fn weighed_score(&self, index: TileIndex) -> f64 {
        let nearest = self.nearest[index as usize];
        let crowding = if nearest == u32::MAX {
            1.0
        } else {
            (f64::from(nearest) / f64::from(self.crowding_distance())).min(1.0)
        };
        self.habitability.score(index) * crowding
    }

    /// The best free site, drawn among the best few when the jitter asks for it.
    fn pick_site(&mut self, landmass: Option<LandmassId>) -> Option<TileIndex> {
        let surface = self.surface;
        let mut candidates: Vec<TileIndex> = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| {
                self.nearest[index as usize] >= self.spacing
                    && self.habitability.score(index) > 0.0
                    && landmass.is_none_or(|id| self.landmass_of(index) == id)
            })
            .collect();
        if candidates.is_empty() {
            return None;
        }
        // the best first, the north-westernmost on ties
        candidates.sort_by(|&a, &b| {
            self.weighed_score(b)
                .total_cmp(&self.weighed_score(a))
                .then(a.cmp(&b))
        });
        let jitter = self.settings.get_site_jitter().min(candidates.len());
        let pick = if jitter > 1 {
            self.rng.random_range(0..jitter)
        } else {
            0
        };
        Some(candidates[pick])
    }

    fn landmass_of(&self, tile: TileIndex) -> LandmassId {
        // only ever asked of a habitable tile, which is land by construction
        self.landmasses
            .of(tile)
            .expect("a habitable tile is on a landmass")
    }

    /// Record the new capital in the distances to the nearest one.
    fn crowd_around(&mut self, tile: TileIndex) {
        for (index, distance) in self.surface.within(tile, self.crowding_distance()) {
            let nearest = &mut self.nearest[index as usize];
            *nearest = (*nearest).min(distance);
        }
    }

    /// Relax the spacing one notch, and recompute the distances. `false` at the floor.
    fn relax(&mut self) -> bool {
        let floor = self.settings.get_spacing_floor();
        if self.spacing <= floor {
            return false;
        }
        let relaxed = (f64::from(self.spacing) * self.settings.get_spacing_relaxation()).floor();
        self.spacing = (relaxed as u32).clamp(floor, self.spacing - 1);
        self.relaxations += 1;
        self.nearest
            .iter_mut()
            .for_each(|nearest| *nearest = u32::MAX);
        let placed: Vec<TileIndex> = self.capitals.iter().map(|capital| capital.tile).collect();
        for tile in placed {
            self.crowd_around(tile);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::simulation::ids::EntityIndex;

    use super::place_capitals;
    use crate::nations::{
        habitability::Habitability, landmasses::Landmasses, settings::NationsGeneratorSettings,
        surface::tests::surface_from_art,
    };

    /// Hex distance between two tiles, by brute force over the surface's own BFS.
    fn distance(surface: &crate::nations::surface::PlanetSurface, a: u32, b: u32) -> u32 {
        surface
            .within(a, u32::MAX)
            .into_iter()
            .find(|&(index, _)| index == b)
            .map(|(_, distance)| distance)
            .expect("reachable")
    }

    const PLAIN: &str =
        "..........|..........|..........|..........|..........|..........|..........|..........";

    #[test]
    fn test_capitals_keep_their_distance_and_sit_on_the_best_sites() {
        let surface = surface_from_art(PLAIN);
        let landmasses = Landmasses::find(&surface);
        let settings = NationsGeneratorSettings::new(0, 4)
            .with_capital_spacing(Some(4))
            .with_site_jitter(1);
        let habitability = Habitability::compute(&surface, &settings);
        let (capitals, report) = place_capitals(&surface, &landmasses, &habitability, &settings);

        assert_eq!(capitals.len(), 4);
        assert_eq!(report.relaxations, 0);
        assert_eq!(report.unplaced, 0);
        for (i, capital) in capitals.iter().enumerate() {
            assert_eq!(capital.nation.index(), i);
            assert!(surface.is_land(capital.tile));
            for other in &capitals[..i] {
                assert!(distance(&surface, capital.tile, other.tile) >= 4);
            }
        }
        // without jitter, the first capital is the single best site
        let best = (0..surface.tiles_count() as u32)
            .max_by(|&a, &b| {
                habitability
                    .score(a)
                    .total_cmp(&habitability.score(b))
                    .then(b.cmp(&a))
            })
            .unwrap();
        assert_eq!(capitals[0].tile, best);
    }

    #[test]
    fn test_capitals_spread_out_rather_than_crowd_the_best_region() {
        // a plain of one uniform quality: the second capital goes as far as it can get
        let surface = surface_from_art(&["....................."; 7].join("|"));
        let landmasses = Landmasses::find(&surface);
        let settings = NationsGeneratorSettings::new(0, 2)
            .with_capital_spacing(Some(3))
            .with_site_jitter(1);
        let habitability = Habitability::compute(&surface, &settings);
        let (capitals, _) = place_capitals(&surface, &landmasses, &habitability, &settings);
        let apart = distance(&surface, capitals[0].tile, capitals[1].tile);
        assert!(apart >= 6, "{apart} hexes apart: {capitals:?}");
    }

    #[test]
    fn test_capitals_relax_the_spacing_when_the_land_is_crowded() {
        let surface = surface_from_art(&[".........."; 6].join("|"));
        let landmasses = Landmasses::find(&surface);
        let settings = NationsGeneratorSettings::new(0, 6).with_capital_spacing(Some(10));
        let habitability = Habitability::compute(&surface, &settings);
        let (capitals, report) = place_capitals(&surface, &landmasses, &habitability, &settings);
        assert!(report.relaxations > 0);
        assert!(report.spacing_final < report.spacing_requested);
        assert!(report.spacing_final >= settings.get_spacing_floor());
        assert_eq!(capitals.len() + report.unplaced as usize, 6);
        assert!(capitals.len() >= 4, "{capitals:?}");
    }

    #[test]
    fn test_capitals_go_unplaced_when_the_planet_is_full() {
        let surface = surface_from_art("~~~|~.~|~~~");
        let landmasses = Landmasses::find(&surface);
        let settings = NationsGeneratorSettings::new(0, 3);
        let habitability = Habitability::compute(&surface, &settings);
        let (capitals, report) = place_capitals(&surface, &landmasses, &habitability, &settings);
        assert_eq!(capitals.len(), 1);
        assert_eq!(report.unplaced, 2);
    }

    #[test]
    fn test_capitals_are_shared_between_landmasses_by_their_weight() {
        // a wide plain and a narrower one, seas on both sides so that neither wraps around
        let surface = surface_from_art(&[".....~~....~"; 8].join("|"));
        let landmasses = Landmasses::find(&surface);
        let settings = NationsGeneratorSettings::new(0, 3)
            .with_capital_spacing(Some(2))
            .with_min_landmass_weight(0.0);
        let habitability = Habitability::compute(&surface, &settings);
        let (capitals, _) = place_capitals(&surface, &landmasses, &habitability, &settings);
        let on_first = capitals
            .iter()
            .filter(|capital| capital.landmass == landmasses.of(0).unwrap())
            .count();
        assert_eq!(on_first, 2, "{capitals:?}");
    }

    #[test]
    fn test_capitals_are_the_same_for_the_same_seed() {
        let surface = surface_from_art(PLAIN);
        let landmasses = Landmasses::find(&surface);
        let place = |seed| {
            let settings = NationsGeneratorSettings::new(seed, 3).with_capital_spacing(Some(3));
            let habitability = Habitability::compute(&surface, &settings);
            place_capitals(&surface, &landmasses, &habitability, &settings).0
        };
        assert_eq!(place(7), place(7));
        assert!(
            (1..=6).any(|seed| place(seed) != place(7)),
            "the jitter draws from the seed"
        );
    }
}
