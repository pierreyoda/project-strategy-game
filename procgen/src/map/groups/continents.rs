use noise::{Fbm, MultiFractal, Perlin};

use crate::map::settings::MapGeneratorSettings;

/// Function subgroup: base continent definition (7 noise functions)
///
/// This subgroup roughly defines the positions and base elevations of the
/// planet's continents.
///
/// The "base elevation" is the elevation of the terrain before any terrain
/// features (mountains, hills, etc.) are placed on that terrain.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
pub fn built_continents_group(settings: &MapGeneratorSettings) {
    // 1: [Continent module]: This FBM module generates the continents. This
    // noise function has a high number of octaves so that detail is visible at
    // high zoom levels.
    let fb0 = Fbm::<Perlin>::new(settings.get_seed())
        .set_frequency(settings.get_continent_frequency())
        .set_persistence(0.5)
        .set_lacunarity(settings.get_continent_lacunarity())
        .set_octaves(14);
}
