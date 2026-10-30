//! Where people would want to live: a score per land tile.

use project_x_core::game::state::TileIndex;

use super::{landmasses::Landmasses, settings::NationsGeneratorSettings, surface::PlanetSurface};

/// A score per tile, roughly from 0.0 (nobody lives here, every water tile included) to 1.0.
///
/// A tile's own quality comes from its terrain, its latitude and its elevation; the mean quality
/// of its neighborhood is then blended in, water counting for nothing, so that a good tile on a
/// thin peninsula scores below the same tile in the middle of a plain. Touching the sea finally
/// earns a bonus, enough for a straight shore to beat the plain behind it.
#[derive(Clone, Debug)]
pub struct Habitability {
    scores: Vec<f64>,
}

impl Habitability {
    pub fn compute(surface: &PlanetSurface, settings: &NationsGeneratorSettings) -> Self {
        let own: Vec<f64> = (0..surface.tiles_count() as TileIndex)
            .map(|index| own_score(surface, settings, index))
            .collect();

        let radius = settings.get_elbow_room_radius();
        let weight = settings.get_elbow_room_weight();
        let coastal_bonus = settings.get_coastal_bonus();
        let scores = (0..surface.tiles_count() as TileIndex)
            .map(|index| {
                if own[index as usize] <= 0.0 {
                    return 0.0;
                }
                let neighborhood = surface.within(index, radius);
                let mean = neighborhood
                    .iter()
                    .map(|&(tile, _)| own[tile as usize])
                    .sum::<f64>()
                    / neighborhood.len() as f64;
                let room = own[index as usize] * (1.0 - weight) + mean * weight;
                if surface.is_coastal(index) {
                    room * (1.0 + coastal_bonus)
                } else {
                    room
                }
            })
            .collect();
        Self { scores }
    }

    pub fn score(&self, index: TileIndex) -> f64 {
        self.scores[index as usize]
    }

    pub fn scores(&self) -> &[f64] {
        &self.scores
    }

    /// The sum of the scores of each landmass: how much of it is worth living on.
    pub fn landmass_weights(&self, landmasses: &Landmasses) -> Vec<f64> {
        landmasses
            .ids()
            .map(|id| {
                landmasses
                    .tiles(id)
                    .iter()
                    .map(|&index| self.score(index))
                    .sum()
            })
            .collect()
    }
}

/// The quality of a tile on its own merits, before its surroundings weigh in.
fn own_score(
    surface: &PlanetSurface,
    settings: &NationsGeneratorSettings,
    index: TileIndex,
) -> f64 {
    if !surface.is_land(index) {
        return 0.0;
    }
    let Some(terrain) = settings.get_terrain_scores().of(surface.terrain_at(index)) else {
        return 0.0;
    };
    if terrain <= 0.0 {
        return 0.0;
    }

    // the mid-latitudes are the sweet spot, the equator and the poles both fall off
    let off_temperate = (surface.latitude(index).abs() - 0.5).abs() * 2.0;
    let latitude = 1.0 - settings.get_latitude_falloff() * off_temperate;

    let above = f64::from(surface.elevation_at(index)) - settings.get_elevation_penalty_from();
    let elevation = (1.0 - above / settings.get_elevation_penalty_span()).clamp(0.0, 1.0);

    terrain * latitude * elevation
}

#[cfg(test)]
mod tests {
    use super::Habitability;
    use crate::nations::{
        landmasses::Landmasses, settings::NationsGeneratorSettings,
        surface::tests::surface_from_art,
    };

    #[test]
    fn test_habitability_is_zero_over_water_and_ice() {
        let surface = surface_from_art("*~-|.:^");
        let habitability = Habitability::compute(&surface, &NationsGeneratorSettings::default());
        assert_eq!(habitability.score(0), 0.0, "ice");
        assert_eq!(habitability.score(1), 0.0, "deep water");
        assert_eq!(habitability.score(2), 0.0, "shallow water");
        assert!(
            habitability.score(3) > habitability.score(4),
            "flat over hills"
        );
        assert!(
            habitability.score(4) > habitability.score(5),
            "hills over mountains"
        );
        assert!(habitability.score(5) > 0.0, "mountains are still land");
    }

    #[test]
    fn test_habitability_rewards_the_shore_but_not_the_peninsula() {
        // all three tiles on the same row, hence at the same latitude
        const PLAIN: &str = ".......~~~~~~~";
        let shore_map = [PLAIN; 7].join("|");
        let mut peninsula_map = [PLAIN; 7];
        peninsula_map[3] = "...........~~~";
        let peninsula_map = peninsula_map.join("|");
        let settings = NationsGeneratorSettings::default();

        let on_shore = Habitability::compute(&surface_from_art(&shore_map), &settings);
        let inland = on_shore.score(3 * 14 + 3);
        let shore = on_shore.score(3 * 14 + 6);
        assert!(shore > inland, "shore {shore} vs inland {inland}");

        let on_peninsula = Habitability::compute(&surface_from_art(&peninsula_map), &settings);
        let tip = on_peninsula.score(3 * 14 + 10);
        assert!(tip > 0.0);
        assert!(inland > tip, "inland {inland} vs tip {tip}");
    }

    #[test]
    fn test_habitability_falls_off_towards_the_poles_and_the_equator() {
        let surface = surface_from_art(".|.|.|.|.|.|.|.|.");
        let habitability = Habitability::compute(&surface, &NationsGeneratorSettings::default());
        let pole = habitability.score(0);
        let temperate = habitability.score(2);
        let equator = habitability.score(4);
        assert!(temperate > pole, "{temperate} vs {pole}");
        assert!(temperate > equator, "{temperate} vs {equator}");
    }

    #[test]
    fn test_habitability_weighs_landmasses_by_their_good_tiles() {
        // a large icy landmass against a small plain
        let surface = surface_from_art("****~..~|****~..~|****~..~");
        let settings = NationsGeneratorSettings::default();
        let habitability = Habitability::compute(&surface, &settings);
        let landmasses = Landmasses::find(&surface);
        let weights = habitability.landmass_weights(&landmasses);
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0], 0.0, "ice weighs nothing");
        assert!(weights[1] > 0.0);
    }
}
