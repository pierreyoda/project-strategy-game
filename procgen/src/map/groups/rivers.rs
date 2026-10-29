use noise::{Curve, Min, MultiFractal, Perlin, RidgedMulti, Seedable, Turbulence};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: river positions
pub struct RiverPositionsGroup {
    pub river_positions: NoiseNode,
}

impl RiverPositionsGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        Self {
            river_positions: build_river_positions(settings),
        }
    }
}

/// Function subgroup: river positions (7 noise functions)
///
/// This subgroup generates the river positions.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_river_positions(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_lacunarity = settings.get_continent_lacunarity();

    // 1: [Large-river-basis module]: This ridged-multifractal-noise function
    // creates the large, deep rivers.
    let rm0 = RidgedMulti::<Perlin>::new(seed + 100)
        .set_frequency(18.75)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(1);

    // 2: [Large-river-curve module]: This curve module applies a curve to the
    // output value from the large-river-basis module so that the ridges become
    // inverted. This creates the rivers. This curve also compresses the edge of
    // the rivers, producing a sharp transition from the land to the river
    // bottom.
    let cu0 = Curve::new(rm0)
        .add_control_point(-2.000, 2.000)
        .add_control_point(-1.000, 1.000)
        .add_control_point(-0.125, 0.875)
        .add_control_point(0.000, -1.000)
        .add_control_point(1.000, -1.500)
        .add_control_point(2.000, -2.000);

    // 3: [Small-river-basis module]: This ridged-multifractal-noise function
    // creates the small, shallow rivers.
    let rm1 = RidgedMulti::<Perlin>::new(seed + 101)
        .set_frequency(43.25)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(1);

    // 4: [Small-river-curve module]: This curve module applies a curve to the
    // output value from the small-river-basis module so that the ridges become
    // inverted. This creates the rivers. This curve also compresses the edge of
    // the rivers, producing a sharp transition from the land to the river
    // bottom.
    let cu1 = Curve::new(rm1)
        .add_control_point(-2.000, 2.0000)
        .add_control_point(-1.000, 1.5000)
        .add_control_point(-0.125, 1.4375)
        .add_control_point(0.000, 0.5000)
        .add_control_point(1.000, 0.2500)
        .add_control_point(2.000, 0.0000);

    // 5: [Combined-rivers module]: This minimum-value module causes the small
    // rivers to cut into the large rivers.  It does this by selecting the
    // minimum output values from the large-river-curve module and the small-
    // river-curve module.
    let mi = Min::new(cu0, cu1);

    // 6: [Warped-rivers module]: This turbulence module warps the output value
    //    from the combined-rivers module, which twists the rivers.  The high
    //    roughness produces less-smooth rivers.
    let tu = Turbulence::<_, Perlin>::new(mi)
        .set_seed(seed + 102)
        .set_frequency(9.25)
        .set_power(1.0 / 57.75)
        .set_roughness(6);

    // 7: [River-positions group]: Caches the output value from the warped-
    //    rivers module.  This is the output value for the entire river-
    //    positions group.
    NoiseNode::cached(tu)
}
