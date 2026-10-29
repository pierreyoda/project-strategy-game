use noise::{
    Billow, Blend, Constant, Exponent, MultiFractal, Perlin, RidgedMulti, ScaleBias, Seedable,
    Turbulence,
};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: hilly terrain
pub struct HillyGroup {
    pub hilly_terrain: NoiseNode,
}

impl HillyGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        Self {
            hilly_terrain: build_hilly_terrain(settings),
        }
    }
}

/// Function subgroup: hilly terrain (11 noise functions)
///
/// This subgroup generates the hilly terrain.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_hilly_terrain(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let hills_lacunarity = settings.get_hills_lacunarity();
    let hills_twist = settings.get_hills_twist();

    // 1: [Hills module]: This billow-noise function generates the hills.
    let bi = Billow::<Perlin>::new(seed + 60)
        .set_frequency(1663.0)
        .set_persistence(0.5)
        .set_lacunarity(hills_lacunarity)
        .set_octaves(6);

    // 2: [Scaled-hills module]: Next, a scale/bias module scales the output
    // value from the hills module so that its hilltops are not too high. The
    // reason for this is that these hills are eventually added to the river
    // valleys (see below).
    let sb0 = ScaleBias::new(bi).set_scale(0.5).set_bias(0.5);

    // 3: [River-valley module]: This ridged-multifractal-noise function generates
    // the river valleys. It has a much lower frequency so that more hills will
    // appear in between the valleys. Note that this noise function generates
    // ridged-multifractal noise using only one octave; this information will be
    // important in the next step.
    let rm = RidgedMulti::<Perlin>::new(seed + 61)
        .set_frequency(367.5)
        .set_lacunarity(hills_lacunarity)
        .set_octaves(1);

    // 4: [Scaled-river-valley module]: Next, a scale/bias module applies a
    // scaling factor of -2.0 to the output value from the river-valley module.
    // This stretches the possible elevation values because one-octave ridged-
    // multifractal noise has a lower range of output values than multiple-
    // octave ridged-multifractal noise. The negative scaling factor inverts
    // the range of the output value, turning the ridges from the river-valley
    // module into valleys.
    let sb1 = ScaleBias::new(rm).set_scale(-2.0).set_bias(-1.0);

    // 5: [Low-flat module]: This low constant value is used by step 6.
    let co = Constant::new(-1.0);

    // 6: [Mountains-and-valleys module]: This blender module merges the scaled-
    // hills module and the scaled-river-valley module together. It causes the
    // low-lying areas of the terrain to become smooth, and causes the high-
    // lying areas of the terrain to contain hills. To do this, it uses uses the
    // scaled-hills module as the control module, causing the low-flat module to
    // appear in the lower areas and causing the scaled-river-valley module to
    // appear in the higher areas.
    let bl = Blend::new(co, sb1, sb0);

    // 7: [Scaled-hills-and-valleys module]: This scale/bias module slightly
    // reduces the range of the output value from the hills-and-valleys
    // module, decreasing the heights of the hilltops.
    let sb2 = ScaleBias::new(bl).set_scale(0.75).set_bias(-0.25);

    // 8: [Increased-slope-hilly-terrain module]: To increase the hill slopes
    // at higher elevations, this exponential-curve module applies an
    // exponential curve to the output value the scaled-hills-and-valleys
    // module. This exponential-curve module expects the input value to range
    // from -1.0 to 1.0.
    let ex = Exponent::new(sb2).set_exponent(1.375);

    // 9: [Coarse-turbulence module]: This turbulence module warps the output
    // value from the increased-slope-hilly-terrain module, adding some
    // coarse detail to it.
    let tu0 = Turbulence::<_, Perlin>::new(ex)
        .set_seed(seed + 62)
        .set_frequency(1531.0)
        .set_power(1.0 / 16921.0 * hills_twist)
        .set_roughness(4);

    // 10: [Warped-hilly-terrain module]: This turbulence module warps the
    // output value from the coarse-turbulence module. This turbulence has a
    // higher frequency, but lower power, than the coarse-turbulence module,
    // adding some fine detail to it.
    let tu1 = Turbulence::<_, Perlin>::new(tu0)
        .set_seed(seed + 63)
        .set_frequency(21617.0)
        .set_power(1.0 / 117529.0 * hills_twist)
        .set_roughness(6);

    // 11: [Hilly-terrain group]: Caches the output value from the warped-hilly-
    // terrain module. This is the output value for the entire hilly-terrain
    // group.
    NoiseNode::cached(tu1)
}
