use project_x_core::game::rng::GameSeed;

use crate::map::TerrainKind;

/// A value per kind of land, for scores and costs alike. Ice stands for both kinds of ice.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PerTerrain<T> {
    pub flat: T,
    pub hills: T,
    pub mountains: T,
    pub ice: T,
}

impl<T: Copy> PerTerrain<T> {
    /// The value of a kind of land, or `None` over water.
    pub fn of(&self, kind: TerrainKind) -> Option<T> {
        match kind {
            TerrainKind::DeepWater | TerrainKind::ShallowWater => None,
            TerrainKind::Flat => Some(self.flat),
            TerrainKind::Hills => Some(self.hills),
            TerrainKind::Mountains => Some(self.mountains),
            TerrainKind::Ice | TerrainKind::RoughIce => Some(self.ice),
        }
    }
}

/// Settings of the nations generator.
#[derive(Clone, Debug)]
pub struct NationsGeneratorSettings {
    seed: GameSeed,
    /// Number of nations to place.
    nations: u16,
    /// Minimum distance between two capitals, in hexes. `None` derives it from the land available
    /// per nation.
    capital_spacing: Option<u32>,
    /// The spacing never relaxes below this.
    spacing_floor: u32,
    /// Factor the spacing relaxes by when no site satisfies it.
    spacing_relaxation: f64,
    /// A capital is drawn among this many of the best remaining sites: 1 always takes the best.
    site_jitter: usize,
    /// Radius of the disc a nation owns around its capital on turn 1.
    border_radius: u32,
    /// How much people like to live on each kind of land, from 0.0 to 1.0.
    terrain_scores: PerTerrain<f64>,
    /// Bonus on the score of a tile adjacent to the sea, as a share of it.
    coastal_bonus: f64,
    /// How much the score drops at the equator and at the poles, from the mid-latitudes.
    latitude_falloff: f64,
    /// Elevation in meters above which the score starts dropping...
    elevation_penalty_from: f64,
    /// ...reaching zero this many meters higher.
    elevation_penalty_span: f64,
    /// Radius of the neighborhood whose mean score is blended into a tile's own.
    elbow_room_radius: u32,
    /// Weight of that neighborhood mean in the final score, from 0.0 (ignored) to 1.0.
    elbow_room_weight: f64,
    /// Cost of entering each kind of land, for the spheres of influence.
    movement_costs: PerTerrain<u32>,
    /// Cost beyond which a sphere of influence stops reaching, if any.
    max_sphere_cost: Option<u32>,
    /// Habitable weight below which a landmass gets no nation at all.
    min_landmass_weight: f64,
}

impl Default for NationsGeneratorSettings {
    fn default() -> Self {
        Self {
            seed: 0,
            nations: 8,
            capital_spacing: None,
            spacing_floor: 3,
            spacing_relaxation: 0.75,
            site_jitter: 3,
            border_radius: 3,
            terrain_scores: PerTerrain {
                flat: 1.0,
                hills: 0.6,
                mountains: 0.1,
                ice: 0.0,
            },
            coastal_bonus: 0.5,
            latitude_falloff: 0.5,
            elevation_penalty_from: 1500.0,
            elevation_penalty_span: 2000.0,
            elbow_room_radius: 2,
            elbow_room_weight: 0.5,
            movement_costs: PerTerrain {
                flat: 1,
                hills: 2,
                mountains: 5,
                ice: 4,
            },
            max_sphere_cost: None,
            min_landmass_weight: 12.0,
        }
    }
}

impl NationsGeneratorSettings {
    pub fn new(seed: GameSeed, nations: u16) -> Self {
        assert!(nations > 0, "nations > 0");
        Self {
            seed,
            nations,
            ..Self::default()
        }
    }

    pub fn with_capital_spacing(self, capital_spacing: Option<u32>) -> Self {
        Self {
            capital_spacing,
            ..self
        }
    }

    pub fn with_site_jitter(self, site_jitter: usize) -> Self {
        assert!(site_jitter > 0, "site_jitter > 0");
        Self {
            site_jitter,
            ..self
        }
    }

    pub fn with_border_radius(self, border_radius: u32) -> Self {
        Self {
            border_radius,
            ..self
        }
    }

    pub fn with_max_sphere_cost(self, max_sphere_cost: Option<u32>) -> Self {
        Self {
            max_sphere_cost,
            ..self
        }
    }

    pub fn with_min_landmass_weight(self, min_landmass_weight: f64) -> Self {
        Self {
            min_landmass_weight,
            ..self
        }
    }

    settings_scalar_accessors! {
        get_seed: seed, GameSeed,
        get_nations: nations, u16,
        get_capital_spacing: capital_spacing, Option<u32>,
        get_spacing_floor: spacing_floor, u32,
        get_spacing_relaxation: spacing_relaxation, f64,
        get_site_jitter: site_jitter, usize,
        get_border_radius: border_radius, u32,
        get_terrain_scores: terrain_scores, PerTerrain<f64>,
        get_coastal_bonus: coastal_bonus, f64,
        get_latitude_falloff: latitude_falloff, f64,
        get_elevation_penalty_from: elevation_penalty_from, f64,
        get_elevation_penalty_span: elevation_penalty_span, f64,
        get_elbow_room_radius: elbow_room_radius, u32,
        get_elbow_room_weight: elbow_room_weight, f64,
        get_movement_costs: movement_costs, PerTerrain<u32>,
        get_max_sphere_cost: max_sphere_cost, Option<u32>,
        get_min_landmass_weight: min_landmass_weight, f64,
    }

    /// Minimum distance between two capitals, in hexes: the setting, or when it is left to the
    /// generator, the distance between the centers of the discs the nations would get if the
    /// land were shared evenly, never below the floor.
    pub fn capital_spacing_for(&self, land_tiles: usize) -> u32 {
        self.capital_spacing.unwrap_or_else(|| {
            let per_nation = land_tiles as f64 / f64::from(self.nations);
            // a disc of radius r holds 3r² + 3r + 1 hexes: r ≈ √(tiles / 3), two of them touch
            // at 2r
            let spacing = (2.0 * (per_nation / 3.0).sqrt()).round() as u32;
            spacing.max(self.spacing_floor)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::NationsGeneratorSettings;

    #[test]
    fn test_capital_spacing_derives_from_the_land_per_nation() {
        let settings = NationsGeneratorSettings::new(0, 8);
        // 1200 land tiles for 8 nations: discs of 150 hexes, radius 7, touching at 14
        assert_eq!(settings.capital_spacing_for(1200), 14);
        // never below the floor, even on a sliver of land
        assert_eq!(settings.capital_spacing_for(8), 3);
        // the setting wins when given
        let settings = settings.with_capital_spacing(Some(12));
        assert_eq!(settings.capital_spacing_for(1200), 12);
    }

    #[test]
    #[should_panic(expected = "nations > 0")]
    fn test_settings_reject_no_nation_at_all() {
        NationsGeneratorSettings::new(0, 0);
    }
}
