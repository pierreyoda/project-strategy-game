use noise::{Perlin, Seedable, Terrace, Turbulence};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

use super::continents::ContinentGroup;

/// Function group: terrain type definition
pub struct TerrainTypeGroup {
    pub terrain_type_def: NoiseNode,
}

impl TerrainTypeGroup {
    pub fn build(settings: &MapGeneratorSettings, continents: &ContinentGroup) -> Self {
        Self {
            terrain_type_def: build_terrain_type_def(settings, &continents.continent_def),
        }
    }
}

/// Function subgroup: terrain type definition (3 noise functions)
///
/// This subgroup defines the positions of the terrain types on the planet.
///
/// Terrain types include, in order of increasing roughness, plains, hills,
/// and mountains.
///
/// This subgroup's output value is based on the output value from the
/// continent-definition group. Rougher terrain mainly appears at higher
/// elevations.
///
/// -1.0 represents the smoothest terrain types (plains and underwater) and
/// +1.0 represents the roughest terrain types (mountains).
fn build_terrain_type_def(settings: &MapGeneratorSettings, continent_def: &NoiseNode) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_frequency = settings.get_continent_frequency();
    let terrain_offset = settings.get_terrain_offset();
    let shelf_level = settings.get_shelf_level();
    let sea_level = settings.get_sea_level();

    // 1: [Warped-continent module]: This turbulence module slightly warps the
    // output value from the continent-definition group. This prevents the
    // rougher terrain from appearing exclusively at higher elevations. Rough
    // areas may now appear in the the ocean, creating rocky islands and
    // fjords.
    let tu = Turbulence::<_, Perlin>::new(continent_def.clone())
        .set_seed(seed + 20)
        .set_frequency(continent_frequency * 18.125)
        .set_power(continent_frequency / 20.59375 * terrain_offset)
        .set_roughness(3);

    // 2: [Roughness-probability-shift module]: This terracing module sharpens
    // the edges of the warped-continent module near sea level and lowers the
    // slope towards the higher-elevation areas. This shrinks the areas in
    // which the rough terrain appears, increasing the "rarity" of rough
    // terrain.
    let te = Terrace::new(tu)
        .add_control_point(-1.00)
        .add_control_point(shelf_level + sea_level / 2.0)
        .add_control_point(1.00);

    // 3: [Terrain-type-definition group]: Caches the output value from the
    // roughness-probability-shift module. This is the output value for the
    // entire terrain-type-definition group.
    NoiseNode::cached(te)
}
