use noise::{
    Add, Clamp, Curve, Fbm, Max, MultiFractal, Perlin, RidgedMulti, ScaleBias, Seedable, Terrace,
    Turbulence, Worley, core::worley::ReturnType,
};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: badlands terrain
pub struct BadlandsGroup {
    pub badlands_sand: NoiseNode,
    pub badlands_cliffs: NoiseNode,
    pub badlands_terrain: NoiseNode,
}

impl BadlandsGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        let badlands_sand = build_badlands_sand(settings);
        let badlands_cliffs = build_badlands_cliffs(settings);
        let badlands_terrain = build_badlands_terrain(&badlands_sand, &badlands_cliffs);
        Self {
            badlands_sand,
            badlands_cliffs,
            badlands_terrain,
        }
    }
}

/// Function subgroup: badlands sand (6 noise functions)
///
/// This subgroup generates the sandy terrain for the badlands.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_badlands_sand(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let badlands_lacunarity = settings.get_badlands_lacunarity();

    // 1: [Sand-dunes module]: This ridged-multifractal-noise function generates
    // sand dunes. This ridged-multifractal noise is generated with a single
    // octave, which makes very smooth dunes.
    let rm = RidgedMulti::<Perlin>::new(seed + 80)
        .set_frequency(6163.5)
        .set_lacunarity(badlands_lacunarity)
        .set_octaves(1);

    // 2: [Scaled-sand-dunes module]: This scale/bias module shrinks the dune
    // heights by a small amount. This is necessary so that the subsequent
    // noise functions in this subgroup can add some detail to the dunes.
    let sb0 = ScaleBias::new(rm).set_scale(0.875).set_bias(0.0);

    // 3: [Dune-detail module]: This noise function uses Voronoi polygons to
    // generate the detail to add to the dunes. By enabling the distance
    // algorithm, small polygonal pits are generated; the edges of the pits
    // are joined to the edges of nearby pits.
    let wo = Worley::new(seed + 81)
        .set_frequency(16183.25)
        .set_return_type(ReturnType::Distance);

    // 4: [Scaled-dune-detail module]: This scale/bias module shrinks the dune
    // details by a large amount. This is necessary so that the subsequent
    // noise functions in this subgroup can add this detail to the sand-dunes
    // module.
    let sb1 = ScaleBias::new(wo).set_scale(0.25).set_bias(0.25);

    // 5: [Dunes-with-detail module]: This addition module combines the scaled-
    // sand-dunes module with the scaled-dune-detail module.
    let ad = Add::new(sb0, sb1);

    // 6: [Badlands-sand subgroup]: Caches the output value from the dunes-with-
    // detail module.
    NoiseNode::cached(ad)
}

/// Function subgroup: badlands cliffs (7 noise functions)
///
/// This subgroup generates the cliffs for the badlands.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_badlands_cliffs(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_frequency = settings.get_continent_frequency();
    let badlands_lacunarity = settings.get_badlands_lacunarity();
    let badlands_twist = settings.get_badlands_twist();

    // 1: [Cliff-basis module]: This Perlin-noise function generates some coherent
    // noise that will be used to generate the cliffs.
    let fb = Fbm::<Perlin>::new(seed + 90)
        .set_frequency(continent_frequency * 839.0)
        .set_persistence(0.5)
        .set_lacunarity(badlands_lacunarity)
        .set_octaves(6);

    // 2: [Cliff-shaping module]: Next, this curve module applies a curve to
    // the output value from the cliff-basis module. This curve is initially
    // very shallow, but then its slope increases sharply. At the highest
    // elevations, the curve becomes very flat again. This produces the
    // stereotypical Utah-style desert cliffs.
    let cu = Curve::new(fb)
        .add_control_point(-2.000, -2.000)
        .add_control_point(-1.000, -1.000)
        .add_control_point(-0.000, -0.750)
        .add_control_point(0.500, -0.250)
        .add_control_point(0.625, 0.875)
        .add_control_point(0.750, 1.000)
        .add_control_point(2.000, 1.250);

    // 3: [Clamped-cliffs module]: This clamping module makes the tops of the
    // cliffs very flat by clamping the output value from the cliff-shaping
    // module.
    let cl = Clamp::new(cu).set_bounds(-999.125, 0.875);

    // 4: [Terraced-cliffs module]: Next, this terracing module applies some
    // terraces to the clamped-cliffs module in the lower elevations before the
    // sharp cliff transition.
    let te = Terrace::new(cl)
        .add_control_point(-1.000)
        .add_control_point(-0.875)
        .add_control_point(-0.750)
        .add_control_point(-0.500)
        .add_control_point(0.000)
        .add_control_point(1.000);

    // 5: [Coarse-turbulence module]: This turbulence module warps the output
    // value from the terraced-cliffs module, adding some coarse detail to it.
    let tu0 = Turbulence::<_, Perlin>::new(te)
        .set_seed(seed + 91)
        .set_frequency(16111.0)
        .set_power(1.0 / 141539.0 * badlands_twist)
        .set_roughness(3);

    // 6: [Warped-cliffs module]: This turbulence module warps the output value
    // from the coarse-turbulence module. This turbulence has a higher
    // frequency, but lower power, than the coarse-turbulence module, adding
    // some fine detail to it.
    let tu1 = Turbulence::<_, Perlin>::new(tu0)
        .set_seed(seed + 92)
        .set_frequency(36107.0)
        .set_power(1.0 / 211543.0 * badlands_twist)
        .set_roughness(3);

    // 7: [Badlands-cliffs subgroup]: Caches the output value from the warped-
    // cliffs module.
    NoiseNode::cached(tu1)
}

/// Function subgroup: badlands terrain (3 noise functions)
///
/// Generates the final badlands terrain.
///
/// Using a scale/bias module, the badlands sand is flattened considerably,
/// then the sand elevations are lowered to around -1.0. The maximum value
/// from the flattened sand module and the cliff module contributes to the
/// final elevation. This causes sand to appear at the low elevations since
/// the sand is slightly higher than the cliff base.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_badlands_terrain(badlands_sand: &NoiseNode, badlands_cliffs: &NoiseNode) -> NoiseNode {
    // 1: [Scaled-sand-dunes module]: This scale/bias module considerably
    // flattens the output value from the badlands-sands subgroup and lowers
    // this value to near -1.0.
    let sb = ScaleBias::new(badlands_sand.clone())
        .set_scale(0.25)
        .set_bias(-0.75);

    // 2: [Dunes-and-cliffs module]: This maximum-value module causes the dunes
    // to appear in the low areas and the cliffs to appear in the high areas.
    // It does this by selecting the maximum of the output values from the
    // scaled-sand-dunes module and the badlands-cliffs subgroup.
    let ma = Max::new(badlands_cliffs.clone(), sb);

    // 3: [Badlands-terrain group]: Caches the output value from the dunes-and-
    // cliffs module. This is the output value for the entire badlands-terrain
    // group.
    NoiseNode::cached(ma)
}
