use noise::{Exponent, Fbm, MultiFractal, Multiply, Perlin, ScaleBias};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

use super::{
    badlands::BadlandsGroup, hilly::HillyGroup, mountainous::MountainousGroup, plains::PlainsGroup,
};

/// Function groups: scaled mountainous terrain, scaled hilly terrain,
/// scaled plains terrain and scaled badlands terrain
pub struct ScaledTerrainGroup {
    pub scaled_mountainous_terrain: NoiseNode,
    pub scaled_hilly_terrain: NoiseNode,
    pub scaled_plains_terrain: NoiseNode,
    pub scaled_badlands_terrain: NoiseNode,
}

impl ScaledTerrainGroup {
    pub fn build(
        settings: &MapGeneratorSettings,
        mountainous: &MountainousGroup,
        hilly: &HillyGroup,
        plains: &PlainsGroup,
        badlands: &BadlandsGroup,
    ) -> Self {
        Self {
            scaled_mountainous_terrain: build_scaled_mountainous_terrain(
                settings,
                &mountainous.mountainous_terrain,
            ),
            scaled_hilly_terrain: build_scaled_hilly_terrain(settings, &hilly.hilly_terrain),
            scaled_plains_terrain: build_scaled_plains_terrain(&plains.plains_terrain),
            scaled_badlands_terrain: build_scaled_badlands_terrain(&badlands.badlands_terrain),
        }
    }
}

/// Function subgroup: scaled mountainous terrain (6 noise functions)
///
/// This subgroup scales the output value from the mountainous-terrain group
/// so that it can be added to the elevation defined by the continent-
/// definition group.
///
/// This subgroup scales the output value such that it is almost always
/// positive.  This is done so that a negative elevation does not get applied
/// to the continent-definition group, preventing parts of that group from
/// having negative terrain features "stamped" into it.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_scaled_mountainous_terrain(
    settings: &MapGeneratorSettings,
    mountainous_terrain: &NoiseNode,
) -> NoiseNode {
    let seed = settings.get_seed();
    let mountain_lacunarity = settings.get_mountain_lacunarity();

    // 1: [Base-scaled-mountainous-terrain module]: This scale/bias module
    // scales the output value from the mountainous-terrain group so that the
    // output value is measured in planetary elevation units.
    let sb0 = ScaleBias::new(mountainous_terrain.clone())
        .set_scale(0.125)
        .set_bias(0.125);

    // 2: [Base-peak-modulation module]: At this stage, most mountain peaks have
    // roughly the same elevation. This BasicMulti module generates some
    // random values that will be used by subsequent noise functions to randomly
    // change the elevations of the mountain peaks.
    let fb = Fbm::<Perlin>::new(seed + 110)
        .set_frequency(14.5)
        .set_persistence(0.5)
        .set_lacunarity(mountain_lacunarity)
        .set_octaves(6);

    // 3: [Peak-modulation module]: This exponential-curve module applies an
    // exponential curve to the output value from the base-peak-modulation
    // module. This produces a small number of high values and a much larger
    // number of low values. This means there will be a few peaks with much
    // higher elevations than the majority of the peaks, making the terrain
    // features more varied.
    let ex = Exponent::new(fb).set_exponent(1.25);

    // 4: [Scaled-peak-modulation module]: This scale/bias module modifies the
    // range of the output value from the peak-modulation module so that it can
    // be used as the modulator for the peak-height-multiplier module. It is
    // important that this output value is not much lower than 1.0.
    let sb1 = ScaleBias::new(ex).set_scale(0.25).set_bias(1.0);

    // 5: [Peak-height-multiplier module]: This multiplier module modulates the
    // heights of the mountain peaks from the base-scaled-mountainous-terrain
    // module using the output value from the scaled-peak-modulation module.
    let mu = Multiply::new(sb0, sb1);

    // 6: [Scaled-mountainous-terrain group]: Caches the output value from the
    // peak-height-multiplier module.  This is the output value for the
    // entire scaled-mountainous-terrain group.
    NoiseNode::cached(mu)
}

/// Function subgroup: scaled hilly terrain (6 noise functions)
///
/// This subgroup scales the output value from the hilly-terrain group so
/// that it can be added to the elevation defined by the continent-
/// definition group. The scaling amount applied to the hills is one half of
/// the scaling amount applied to the scaled-mountainous-terrain group.
///
/// This subgroup scales the output value such that it is almost always
/// positive. This is done so that negative elevations are not applied to
/// the continent-definition group, preventing parts of the continent-
/// definition group from having negative terrain features "stamped" into it.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_scaled_hilly_terrain(
    settings: &MapGeneratorSettings,
    hilly_terrain: &NoiseNode,
) -> NoiseNode {
    let seed = settings.get_seed();
    let hills_lacunarity = settings.get_hills_lacunarity();

    // 1: [Base-scaled-hilly-terrain module]: This scale/bias module scales the
    // output value from the hilly-terrain group so that this output value is
    // measured in planetary elevation units.
    let sb0 = ScaleBias::new(hilly_terrain.clone())
        .set_scale(0.0625)
        .set_bias(0.0625);

    // 2: [Base-hilltop-modulation module]: At this stage, most hilltops have
    // roughly the same elevation. This BasicMulti module generates some
    // random values that will be used by subsequent noise functions to
    // randomly change the elevations of the hilltops.
    let fb = Fbm::<Perlin>::new(seed + 120)
        .set_frequency(13.5)
        .set_persistence(0.5)
        .set_lacunarity(hills_lacunarity)
        .set_octaves(6);

    // 3: [Hilltop-modulation module]: This exponential-curve module applies an
    // exponential curve to the output value from the base-hilltop-modulation
    // module. This produces a small number of high values and a much larger
    // number of low values. This means there will be a few hilltops with
    // much higher elevations than the majority of the hilltops, making the
    // terrain features more varied.
    let ex = Exponent::new(fb).set_exponent(1.25);

    // 4: [Scaled-hilltop-modulation module]: This scale/bias module modifies
    // the range of the output value from the hilltop-modulation module so that
    // it can be used as the modulator for the hilltop-height-multiplier module.
    // It is important that this output value is not much lower than 1.0.
    let sb1 = ScaleBias::new(ex).set_scale(0.5).set_bias(1.5);

    // 5: [Hilltop-height-multiplier module]: This multiplier module modulates
    // the heights of the hilltops from the base-scaled-hilly-terrain module
    // using the output value from the scaled-hilltop-modulation module.
    let mu = Multiply::new(sb0, sb1);

    // 6: [Scaled-hilly-terrain group]: Caches the output value from the
    // hilltop-height-multiplier module. This is the output value for the entire
    // scaled-hilly-terrain group.
    NoiseNode::cached(mu)
}

/// Function subgroup: scaled plains terrain (2 noise functions)
///
/// This subgroup scales the output value from the plains-terrain group so
/// that it can be added to the elevations defined by the continent-
/// definition group.
///
/// This subgroup scales the output value such that it is almost always
/// positive. This is done so that negative elevations are not applied to
/// the continent-definition group, preventing parts of the continent-
/// definition group from having negative terrain features "stamped" into it.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_scaled_plains_terrain(plains_terrain: &NoiseNode) -> NoiseNode {
    // 1: [Scaled-plains-terrain module]: This scale/bias module greatly
    // flattens the output value from the plains terrain.  This output value
    // is measured in planetary elevation units.
    let sb0 = ScaleBias::new(plains_terrain.clone())
        .set_scale(0.00390625)
        .set_bias(0.0078125);

    // 2: [Scaled-plains-terrain group]: Caches the output value from the
    // scaled-plains-terrain module. This is the output value for the entire
    // scaled-plains-terrain group.
    NoiseNode::cached(sb0)
}

/// Function subgroup: scaled badlands terrain (2 noise functions)
///
/// This subgroup scales the output value from the badlands-terrain group so
/// that it can be added to the elevations defined by the continent-
/// definition group.
///
/// This subgroup scales the output value such that it is almost always
/// positive. This is done so that negative elevations are not applied to the
/// continent-definition group, preventing parts of the continent-definition
/// group from having negative terrain features "stamped" into it.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_scaled_badlands_terrain(badlands_terrain: &NoiseNode) -> NoiseNode {
    // 1: [Scaled-badlands-terrain module]: This scale/bias module scales the
    // output value from the badlands-terrain group so that it is measured
    // in planetary elevation units.
    let sb = ScaleBias::new(badlands_terrain.clone())
        .set_scale(0.0625)
        .set_bias(0.0625);

    // 2: [Scaled-badlands-terrain group]: Caches the output value from the
    // scaled-badlands-terrain module. This is the output value for the
    // entire scaled-badlands-terrain group.
    NoiseNode::cached(sb)
}
