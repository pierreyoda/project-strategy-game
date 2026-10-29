use noise::{Billow, MultiFractal, Multiply, Perlin, ScaleBias};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

/// Function group: plains terrain
pub struct PlainsGroup {
    pub plains_terrain: NoiseNode,
}

impl PlainsGroup {
    pub fn build(settings: &MapGeneratorSettings) -> Self {
        Self {
            plains_terrain: build_plains_terrain(settings),
        }
    }
}

/// Function subgroup: plains terrain (7 noise functions)
///
/// This subgroup generates the plains terrain.
///
/// Because this subgroup will eventually be flattened considerably, the
/// types and combinations of noise functions that generate the plains are not
/// really that important; they only need to "look" interesting.
///
/// -1.0 represents the lowest elevations and +1.0 represents the highest
/// elevations.
fn build_plains_terrain(settings: &MapGeneratorSettings) -> NoiseNode {
    let seed = settings.get_seed();
    let plains_lacunarity = settings.get_plains_lacunarity();

    // 1: [Plains-basis-0 module]: This billow-noise function, along with the
    // plains-basis-1 module, produces the plains.
    let bi0 = Billow::<Perlin>::new(seed + 70)
        .set_frequency(1097.5)
        .set_persistence(0.5)
        .set_lacunarity(plains_lacunarity)
        .set_octaves(8);

    // 2: [Positive-plains-basis-0 module]: This scale/bias module makes the
    // output value from the plains-basis-0 module positive since this output
    // value will be multiplied together with the positive-plains-basis-1
    // module.
    let sb0 = ScaleBias::new(bi0).set_scale(0.5).set_bias(0.5);

    // 3: [Plains-basis-1 module]: This billow-noise function, along with the
    // plains-basis-2 module, produces the plains.
    let bi1 = Billow::<Perlin>::new(seed + 71)
        .set_frequency(1097.5)
        .set_persistence(0.5)
        .set_lacunarity(plains_lacunarity)
        .set_octaves(8);

    // 4: [Positive-plains-basis-1 module]: This scale/bias module makes the
    // output value from the plains-basis-1 module positive since this output
    // value will be multiplied together with the positive-plains-basis-0
    // module.
    let sb1 = ScaleBias::new(bi1).set_scale(0.5).set_bias(0.5);

    // 5: [Combined-plains-basis module]: This multiplication module combines
    // the two plains basis modules together.
    let mu = Multiply::new(sb0, sb1);

    // 6: [Rescaled-plains-basis module]: This scale/bias module maps the output
    // value that ranges from 0.0 to 1.0 back to a value that ranges from
    // -1.0 to +1.0.
    let sb2 = ScaleBias::new(mu).set_scale(2.0).set_bias(-1.0);

    // 7: [Plains-terrain group]: Caches the output value from the rescaled-
    // plains-basis module.  This is the output value for the entire plains-
    // terrain group.
    NoiseNode::cached(sb2)
}
