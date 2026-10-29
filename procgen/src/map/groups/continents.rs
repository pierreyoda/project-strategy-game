use noise::{
    Clamp, Curve, Fbm, Min, MultiFractal, Perlin, ScaleBias, Seedable, Select, Turbulence,
};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: continent definition
pub struct ContinentGroup {
    pub base_continent_def: NoiseNode,
    pub continent_def: NoiseNode,
}

impl ContinentGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        let base_continent_def = build_base_continent_def(settings);
        let continent_def = build_continent_def(settings, &base_continent_def);
        Self {
            base_continent_def,
            continent_def,
        }
    }
}

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
fn build_base_continent_def(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_frequency = settings.get_continent_frequency();
    let continent_lacunarity = settings.get_continent_lacunarity();
    let sea_level = settings.get_sea_level();

    // 1: [Continent module]: This FBM module generates the continents. This
    // noise function has a high number of octaves so that detail is visible at
    // high zoom levels.
    let fb0 = Fbm::<Perlin>::new(seed)
        .set_frequency(continent_frequency)
        .set_persistence(0.5)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(14);

    // 2: [Continent-with-ranges module]: Next, a curve module modifies the
    // output value from the continent module so that very high values appear
    // near sea level. This defines the positions of the mountain ranges.
    let cu = Curve::new(fb0)
        .add_control_point(-2.0000 + sea_level, -1.625 + sea_level)
        .add_control_point(-1.0000 + sea_level, -1.375 + sea_level)
        .add_control_point(0.0000 + sea_level, -0.375 + sea_level)
        .add_control_point(0.0625 + sea_level, 0.125 + sea_level)
        .add_control_point(0.1250 + sea_level, 0.250 + sea_level)
        .add_control_point(0.2500 + sea_level, 1.000 + sea_level)
        .add_control_point(0.5000 + sea_level, 0.250 + sea_level)
        .add_control_point(0.7500 + sea_level, 0.250 + sea_level)
        .add_control_point(1.0000 + sea_level, 0.500 + sea_level)
        .add_control_point(2.0000 + sea_level, 0.500 + sea_level);

    // 3: [Carver module]: This higher-frequency BasicMulti module will be
    // used by subsequent noise functions to carve out chunks from the
    // mountain ranges within the continent-with-ranges module so that the
    // mountain ranges will not be completely impassible.
    let fb1 = Fbm::<Perlin>::new(seed + 1)
        .set_frequency(continent_frequency * 4.34375)
        .set_persistence(0.5)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(11);

    // 4: [Scaled-carver module]: This scale/bias module scales the output
    // value from the carver module such that it is usually near 1.0. This
    // is required for step 5.
    let sb = ScaleBias::new(fb1).set_scale(0.375).set_bias(0.625);

    // 5: [Carved-continent module]: This minimum-value module carves out
    // chunks from the continent-with-ranges module. it does this by ensuring
    // that only the minimum of the output values from the scaled-carver
    // module and the continent-with-ranges module contributes to the output
    // value of this subgroup. Most of the time, the minimum value module will
    // select the output value from the continent-with-ranges module since the
    // output value from the scaled-carver is usually near 1.0. Occasionally,
    // the output from the scaled-carver module will be less than the output
    // value from the continent-with-ranges module, so in this case, the output
    // value from the scaled-carver module is selected.
    let mi = Min::new(sb, cu);

    // 6: [Clamped-continent module]: Finally, a clamp module modifies the
    // carved continent module to ensure that the output value of this subgroup
    // is between -1.0 and 1.0.
    let cl = Clamp::new(mi).set_bounds(-1.0, 1.0);

    // 7: [Base-continent-definition subgroup]: Caches the output value from
    // the clamped-continent module.
    NoiseNode::cached(cl)
}

/// Function subgroup: continent definition (5 noise functions)
///
/// This subgroup warps the output value from the base-continent-definition
/// subgroup, producing more realistic terrain.
///
/// Warping the base continent definition produces lumpier terrain with
/// cliffs and rifts.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_continent_def(
    settings: &MapGeneratorSettings,
    base_continent_def: &NoiseNode,
) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_frequency = settings.get_continent_frequency();
    let sea_level = settings.get_sea_level();

    // 1: [Coarse-turbulence module]: This turbulence module warps the output
    // value from the base-continent-definition subgroup, adding some coarse
    // detail to it.
    let tu0 = Turbulence::<_, Perlin>::new(base_continent_def.clone())
        .set_seed(seed + 10)
        .set_frequency(continent_frequency * 15.25)
        .set_power(continent_frequency / 113.75)
        .set_roughness(13);

    // 2: [Intermediate-turbulence module]: This turbulence module warps the
    // output value from the coarse-turbulence module. This turbulence has a
    // higher frequency, but lower power, than the coarse-turbulence module,
    // adding some intermediate detail to it.
    let tu1 = Turbulence::<_, Perlin>::new(tu0)
        .set_seed(seed + 11)
        .set_frequency(continent_frequency * 47.25)
        .set_power(continent_frequency / 433.75)
        .set_roughness(12);

    // 3: [Warped-base-continent-definition module]: This turbulence module
    // warps the output value from the intermediate-turbulence module. This
    // turbulence has a higher frequency, but lower power, than the
    // intermediate-turbulence module, adding some fine detail to it.
    let tu2 = Turbulence::<_, Perlin>::new(tu1)
        .set_seed(seed + 12)
        .set_frequency(continent_frequency * 95.25)
        .set_power(continent_frequency / 1019.75)
        .set_roughness(11);

    // 4: [Select-turbulence module]: At this stage, the turbulence is applied
    // to the entire base-continent-definition subgroup, producing some very
    // rugged, unrealistic coastlines.  This selector module selects the
    // output values from the (unwarped) base-continent-definition subgroup
    // and the warped-base-continent-definition module, based on the output
    // value from the (unwarped) base-continent-definition subgroup.  The
    // selection boundary is near sea level and has a relatively smooth
    // transition.  In effect, only the higher areas of the base-continent-
    // definition subgroup become warped; the underwater and coastal areas
    // remain unaffected.
    let se = Select::new(base_continent_def.clone(), tu2, base_continent_def.clone())
        .set_bounds(sea_level - 0.0375, sea_level + 1000.0375)
        .set_falloff(0.0625);

    // 5: [Continent-definition group]: Caches the output value from the
    // clamped-continent module. This is the output value for the entire
    // continent-definition group.
    NoiseNode::cached(se)
}
