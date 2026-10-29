use noise::{
    Add, Blend, Constant, Exponent, Max, MultiFractal, Multiply, Perlin, RidgedMulti, ScaleBias,
    Seedable, Select, Turbulence,
};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: mountainous terrain
pub struct MountainousGroup {
    pub mountain_base_def: NoiseNode,
    pub mountainous_high: NoiseNode,
    pub mountainous_low: NoiseNode,
    pub mountainous_terrain: NoiseNode,
}

impl MountainousGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        let mountain_base_def = build_mountain_base_def(settings);
        let mountainous_high = build_mountainous_high(settings);
        let mountainous_low = build_mountainous_low(settings);
        let mountainous_terrain = build_mountainous_terrain(
            settings,
            &mountain_base_def,
            &mountainous_high,
            &mountainous_low,
        );
        Self {
            mountain_base_def,
            mountainous_high,
            mountainous_low,
            mountainous_terrain,
        }
    }
}

/// Function subgroup: mountain base definition (9 noise functions)
///
/// This subgroup generates the base-mountain elevations. Other subgroups
/// will add the ridges and low areas to the base elevations.
///
/// -1.0 represents low mountainous terrain and +1.0 represents high
/// mountainous terrain.
fn build_mountain_base_def(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let mountain_lacunarity = settings.get_mountain_lacunarity();
    let mountains_twist = settings.get_mountains_twist();

    // 1: [Mountain-ridge module]: This ridged-multifractal-noise function
    // generates the mountain ridges.
    let rm0 = RidgedMulti::<Perlin>::new(seed + 30)
        .set_frequency(1723.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(4);

    // 2: [Scaled-mountain-ridge module]: Next, a scale/bias module scales the
    // output value from the mountain-ridge module so that its ridges are not
    // too high. The reason for this is that another subgroup adds actual
    // mountainous terrain to these ridges.
    let sb0 = ScaleBias::new(rm0).set_scale(0.5).set_bias(0.375);

    // 3: [River-valley module]: This ridged-multifractal-noise function
    // generates the river valleys.  It has a much lower frequency than the
    // mountain-ridge module so that more mountain ridges will appear outside
    // of the valleys. Note that this noise function generates ridged-multifractal
    // noise using only one octave; this information will be important in the
    // next step.
    let rm1 = RidgedMulti::<Perlin>::new(seed + 31)
        .set_frequency(367.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(1);

    // 4: [Scaled-river-valley module]: Next, a scale/bias module applies a
    // scaling factor of -2.0 to the output value from the river-valley module.
    // This stretches the possible elevation values because one-octave ridged-
    // multifractal noise has a lower range of output values than multiple-
    // octave ridged-multifractal noise. The negative scaling factor inverts
    // the range of the output value, turning the ridges from the river-valley
    // module into valleys.
    let sb1 = ScaleBias::new(rm1).set_scale(-2.0).set_bias(-0.5);

    // 5: [Low-flat module]: This low constant value is used by step 6.
    let co = Constant::new(-1.0);

    // 6: [Mountains-and-valleys module]: This blender module merges the scaled-
    // mountain-ridge module and the scaled-river-valley module together. It
    // causes the low-lying areas of the terrain to become smooth, and causes
    // the high-lying areas of the terrain to contain ridges. To do this, it
    // uses the scaled-river-valley module as the control module, causing the
    // low-flat module to appear in the lower areas and causing the scaled-
    // mountain-ridge module to appear in the higher areas.
    let bl = Blend::new(co, sb0, sb1);

    // 7: [Coarse-turbulence module]: This turbulence module warps the output
    // value from the mountain-and-valleys module, adding some coarse detail to
    // it.
    let tu0 = Turbulence::<_, Perlin>::new(bl)
        .set_seed(seed + 32)
        .set_frequency(1337.0)
        .set_power(1.0 / 6730.0 * mountains_twist)
        .set_roughness(4);

    // 8: [Warped-mountains-and-valleys module]: This turbulence module warps
    // the output value from the coarse-turbulence module. This turbulence has
    // a higher frequency, but lower power, than the coarse-turbulence module,
    // adding some fine detail to it.
    let tu1 = Turbulence::<_, Perlin>::new(tu0)
        .set_seed(seed + 33)
        .set_frequency(21221.0)
        .set_power(1.0 / 120157.0 * mountains_twist)
        .set_roughness(6);

    // 9: [Mountain-base-definition subgroup]: Caches the output value from the
    // warped-mountains-and-valleys module.
    NoiseNode::cached(tu1)
}

/// Function subgroup: high mountainous terrain (5 noise functions)
///
/// This subgroup generates the mountainous terrain that appears at high
/// elevations within the mountain ridges.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_mountainous_high(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let mountain_lacunarity = settings.get_mountain_lacunarity();
    let mountains_twist = settings.get_mountains_twist();

    // 1: [Mountain-basis-0 module]: This ridged-multifractal-noise function,
    // along with the mountain-basis-1 module, generates the individual
    // mountains.
    let rm0 = RidgedMulti::<Perlin>::new(seed + 40)
        .set_frequency(2371.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(3);

    // 2: [Mountain-basis-1 module]: This ridged-multifractal-noise function,
    // along with the mountain-basis-0 module, generates the individual
    // mountains.
    let rm1 = RidgedMulti::<Perlin>::new(seed + 41)
        .set_frequency(2341.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(3);

    // 3: [High-mountains module]: Next, a maximum-value module causes more
    // mountains to appear at the expense of valleys. It does this by ensuring
    // that only the maximum of the output values from the two ridged-
    // multifractal-noise functions contribute to the output value of this
    // subgroup.
    let ma = Max::new(rm0, rm1);

    // 4: [Warped-high-mountains module]: This turbulence module warps the
    // output value from the high-mountains module, adding some detail to it.
    let tu = Turbulence::<_, Perlin>::new(ma)
        .set_seed(seed + 42)
        .set_frequency(31511.0)
        .set_power(1.0 / 180371.0 * mountains_twist)
        .set_roughness(4);

    // 5: [High-mountainous-terrain subgroup]: Caches the output value from the
    // warped-high-mountains module.
    NoiseNode::cached(tu)
}

/// Function subgroup: low mountainous terrain (4 noise functions)
///
/// This subgroup generates the mountainous terrain that appears at low
/// elevations within the river valleys.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_mountainous_low(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let mountain_lacunarity = settings.get_mountain_lacunarity();

    // 1: [Lowland-basis-0 module]: This ridged-multifractal-noise function,
    // along with the lowland-basis-1 module, produces the low mountainous
    // terrain.
    let rm0 = RidgedMulti::<Perlin>::new(seed + 50)
        .set_frequency(1381.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(8);

    // 1: [Lowland-basis-1 module]: This ridged-multifractal-noise function,
    // along with the lowland-basis-0 module, produces the low mountainous
    // terrain.
    let rm1 = RidgedMulti::<Perlin>::new(seed + 51)
        .set_frequency(1427.0)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(8);

    // 3: [Low-mountainous-terrain module]: This multiplication module combines
    // the output values from the two ridged-multifractal-noise functions. This
    // causes the following to appear in the resulting terrain:
    // - Cracks appear when two negative output values are multiplied together.
    // - Flat areas appear when a positive and a negative output value are
    //   multiplied together.
    // - Ridges appear when two positive output values are multiplied together.
    let mu = Multiply::new(rm0, rm1);

    // 4: [Low-mountainous-terrain subgroup]: Caches the output value from the
    // low-mountainous-terrain module.
    NoiseNode::cached(mu)
}

/// Function subgroup: mountainous terrain (7 noise functions)
///
/// This subgroup generates the final mountainous terrain by combining the
/// high-mountainous-terrain subgroup with the low-mountainous-terrain
/// subgroup.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_mountainous_terrain(
    settings: &MapGeneratorSettings,
    mountain_base_def: &NoiseNode,
    mountainous_high: &NoiseNode,
    mountainous_low: &NoiseNode,
) -> NoiseNode {
    let mountain_glaciation = settings.get_mountain_glaciation();

    // 1: [Scaled-low-mountainous-terrain module]: First, this scale/bias module
    // scales the output value from the low-mountainous-terrain subgroup to a very
    // low value and biases it towards -1.0. This results in the low mountainous
    // areas becoming more-or-less flat with little variation. This will also
    // result in the low mountainous areas appearing at the lowest elevations in
    // this subgroup.
    let sb0 = ScaleBias::new(mountainous_low.clone())
        .set_scale(0.03125)
        .set_bias(-0.96875);

    // 2: [Scaled-high-mountainous-terrain module]: Next, this scale/bias module
    // scales the output value from the high-mountainous-terrain subgroup to 1/4
    // of its initial value and biases it so that its output value is usually
    // positive.
    let sb1 = ScaleBias::new(mountainous_high.clone())
        .set_scale(0.25)
        .set_bias(0.25);

    // 3: [Added-high-mountainous-terrain module]: This addition module adds the
    // output value from the scaled-high-mountainous-terrain module to the
    // output value from the mountain-base-definition subgroup. Mountains now
    // appear all over the terrain.
    let ad = Add::new(sb1, mountain_base_def.clone());

    // 4: [Combined-mountainous-terrain module]: Note that at this point, the
    // entire terrain is covered in high mountainous terrain, even at the low
    // elevations. To make sure the mountains only appear at the higher
    // elevations, this selector module causes low mountainous terrain to appear
    // at the low elevations (within the valleys) and the high mountainous
    // terrain to appear at the high elevations (within the ridges). To do this,
    // this noise function selects the output value from the added-high-
    // mountainous-terrain module if the output value from the mountain-base-
    // definition subgroup is higher than a set amount. Otherwise, this noise
    // module selects the output value from the scaled-low-mountainous-terrain
    // module.
    let se = Select::new(sb0, ad, mountain_base_def.clone())
        .set_bounds(-0.5, 999.5)
        .set_falloff(0.5);

    // 5: [Scaled-mountainous-terrain-module]: This scale/bias module slightly
    // reduces the range of the output value from the combined-mountainous-
    // terrain module, decreasing the heights of the mountain peaks.
    let sb2 = ScaleBias::new(se).set_scale(0.8).set_bias(0.0);

    // 6: [Glaciated-mountainous-terrain-module]: This exponential-curve module
    // applies an exponential curve to the output value from the scaled-
    // mountainous-terrain module. This causes the slope of the mountains to
    // smoothly increase towards higher elevations, as if a glacier ground out
    // those mountains. This exponential-curve module expects the output value
    // to range from -1.0 to +1.0.
    let ex = Exponent::new(sb2).set_exponent(mountain_glaciation);

    NoiseNode::cached(ex)
}
