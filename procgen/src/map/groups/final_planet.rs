use noise::{
    Add, Clamp, Curve, Fbm, Max, MultiFractal, Perlin, RidgedMulti, ScaleBias, Select, Terrace,
};

use crate::map::{node::NoiseNode, settings::MapGeneratorSettings};

use super::{
    continents::ContinentGroup, rivers::RiverPositionsGroup, scaled::ScaledTerrainGroup,
    terrain_type::TerrainTypeGroup,
};

/// Function group: final planet
///
/// Stops at the unscaled final planet, like the noise-rs example: the conversion
/// to meters happens when sampling the planet.
pub struct FinalPlanetGroup {
    pub continental_shelf: NoiseNode,
    pub base_continent_elev: NoiseNode,
    pub continents_with_plains: NoiseNode,
    pub continents_with_hills: NoiseNode,
    pub continents_with_mountains: NoiseNode,
    pub continents_with_badlands: NoiseNode,
    pub continents_with_rivers: NoiseNode,
    pub unscaled_final_planet: NoiseNode,
}

impl FinalPlanetGroup {
    pub fn build(
        settings: &MapGeneratorSettings,
        continents: &ContinentGroup,
        terrain_type: &TerrainTypeGroup,
        scaled: &ScaledTerrainGroup,
        rivers: &RiverPositionsGroup,
    ) -> Self {
        let continent_def = &continents.continent_def;
        let terrain_type_def = &terrain_type.terrain_type_def;

        let continental_shelf = build_continental_shelf(settings, continent_def);
        let base_continent_elev =
            build_base_continent_elev(settings, continent_def, &continental_shelf);
        let continents_with_plains =
            build_continents_with_plains(&base_continent_elev, &scaled.scaled_plains_terrain);
        let continents_with_hills = build_continents_with_hills(
            settings,
            &base_continent_elev,
            &scaled.scaled_hilly_terrain,
            &continents_with_plains,
            terrain_type_def,
        );
        let continents_with_mountains = build_continents_with_mountains(
            settings,
            &base_continent_elev,
            &scaled.scaled_mountainous_terrain,
            continent_def,
            &continents_with_hills,
            terrain_type_def,
        );
        let continents_with_badlands = build_continents_with_badlands(
            settings,
            &base_continent_elev,
            &scaled.scaled_badlands_terrain,
            &continents_with_mountains,
        );
        let continents_with_rivers = build_continents_with_rivers(
            settings,
            &rivers.river_positions,
            &continents_with_badlands,
        );
        let unscaled_final_planet = build_unscaled_final_planet(&continents_with_rivers);
        Self {
            continental_shelf,
            base_continent_elev,
            continents_with_plains,
            continents_with_hills,
            continents_with_mountains,
            continents_with_badlands,
            continents_with_rivers,
            unscaled_final_planet,
        }
    }
}

/// Function subgroup: continental shelf (6 noise functions)
///
/// This module subgroup creates the continental shelves.
///
/// The output value from this module subgroup are measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continental_shelf(
    settings: &MapGeneratorSettings,
    continent_def: &NoiseNode,
) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_frequency = settings.get_continent_frequency();
    let continent_lacunarity = settings.get_continent_lacunarity();
    let shelf_level = settings.get_shelf_level();
    let sea_level = settings.get_sea_level();

    // 1: [Shelf-creator module]: This terracing module applies a terracing
    // curve to the continent-definition group at the specified shelf level.
    // This terrace becomes the continental shelf. Note that this terracing
    // module also places another terrace below the continental shelf near -1.0.
    // The bottom of this terrace is defined as the bottom of the ocean;
    // subsequent noise functions will later add oceanic trenches to the bottom of
    // the ocean.
    let te = Terrace::new(continent_def.clone())
        .add_control_point(-1.0)
        .add_control_point(-0.75)
        .add_control_point(shelf_level)
        .add_control_point(1.0);

    // 2: [Clamped-sea-bottom module]: This clamping module clamps the output
    // value from the shelf-creator module so that its possible range is from
    // the bottom of the ocean to sea level. This is done because this subgroup
    // is only concerned about the oceans.
    let cl = Clamp::new(te).set_bounds(-0.75, sea_level);

    // 3: [Oceanic-trench-basis module]: This ridged-multifractal-noise function
    // generates some coherent noise that will be used to generate the oceanic
    // trenches. The ridges represent the bottom of the trenches.
    let rm = RidgedMulti::<Perlin>::new(seed + 130)
        .set_frequency(continent_frequency * 4.375)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(16);

    // 4: [Oceanic-trench module]: This scale/bias module inverts the ridges
    // from the oceanic-trench-basis-module so that the ridges become trenches.
    // This noise function also reduces the depth of the trenches so that their
    // depths are measured in planetary elevation units.
    let sb = ScaleBias::new(rm).set_scale(-0.125).set_bias(-0.125);

    // 5: [Shelf-and-trenches module]: This addition module adds the oceanic
    // trenches to the clamped-sea-bottom module.
    let ad = Add::new(sb, cl);

    // 6: [Continental-shelf subgroup]: Caches the output value from the shelf-
    //    and-trenches module.
    NoiseNode::cached(ad)
}

/// Function subgroup: base continent elevations (3 noise functions)
///
/// This subgroup generates the base elevations for the continents, before
/// terrain features are added.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_base_continent_elev(
    settings: &MapGeneratorSettings,
    continent_def: &NoiseNode,
    continental_shelf: &NoiseNode,
) -> NoiseNode {
    let continent_height_scale = settings.get_continent_height_scale();
    let shelf_level = settings.get_shelf_level();

    // 1: [Base-scaled-continent-elevations module]: This scale/bias module
    // scales the output value from the continent-definition group so that it
    // is measured in planetary elevation units.
    let sb = ScaleBias::new(continent_def.clone())
        .set_scale(continent_height_scale)
        .set_bias(0.0);

    // 2: [Base-continent-with-oceans module]: This selector module applies the
    // elevations of the continental shelves to the base elevations of the
    // continent. It does this by selecting the output value from the
    // continental-shelf subgroup if the corresponding output value from the
    // continent-definition group is below the shelf level. Otherwise, it
    // selects the output value from the base-scaled-continent-elevations
    // module.
    let se = Select::new(sb, continental_shelf.clone(), continent_def.clone())
        .set_bounds(shelf_level - 1000.0, shelf_level)
        .set_falloff(0.03125);

    // 3: [Base-continent-elevation subgroup]: Caches the output value from the
    // base-continent-with-oceans module.
    NoiseNode::cached(se)
}

/// Function subgroup: continents with plains (2 noise functions)
///
/// This subgroup applies the scaled-plains-terrain group to the base-
/// continent-elevation subgroup.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continents_with_plains(
    base_continent_elev: &NoiseNode,
    scaled_plains_terrain: &NoiseNode,
) -> NoiseNode {
    // 1: [Continents-with-plains module]: This addition module adds the scaled-
    // plains-terrain group to the base-continent-elevation subgroup.
    let ad = Add::new(base_continent_elev.clone(), scaled_plains_terrain.clone());

    // 2: [Continents-with-plains subgroup]: Caches the output value from the
    // continents-with-plains module.
    NoiseNode::cached(ad)
}

/// Function subgroup: continents with hills (3 noise functions)
///
/// This subgroup applies the scaled-hilly-terrain group to the continents-
/// with-plains subgroup.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continents_with_hills(
    settings: &MapGeneratorSettings,
    base_continent_elev: &NoiseNode,
    scaled_hilly_terrain: &NoiseNode,
    continents_with_plains: &NoiseNode,
    terrain_type_def: &NoiseNode,
) -> NoiseNode {
    let hills_amount = settings.get_hills_amount();

    // 1: [Continents-with-hills module]: This addition module adds the scaled-
    // hilly-terrain group to the base-continent-elevation subgroup.
    let ad = Add::new(base_continent_elev.clone(), scaled_hilly_terrain.clone());

    // 2: [Select-high-elevations module]: This selector module ensures that the
    // hills only appear at higher elevations. It does this by selecting the
    // output value from the continent-with-hills module if the corresponding
    // output value from the terrain-type-definition group is above a certain
    // value. Otherwise, it selects the output value from the continents-with-
    // plains subgroup.
    let se = Select::new(continents_with_plains.clone(), ad, terrain_type_def.clone())
        .set_bounds(1.0 - hills_amount, 1001.0 - hills_amount)
        .set_falloff(0.25);

    // 3: [Continents-with-hills subgroup]: Caches the output value from the
    // select-high-elevations module.
    NoiseNode::cached(se)
}

/// Function subgroup: continents with mountains (5 noise functions)
///
/// This subgroup applies the scaled-mountainous-terrain group to the
/// continents-with-hills subgroup.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continents_with_mountains(
    settings: &MapGeneratorSettings,
    base_continent_elev: &NoiseNode,
    scaled_mountainous_terrain: &NoiseNode,
    continent_def: &NoiseNode,
    continents_with_hills: &NoiseNode,
    terrain_type_def: &NoiseNode,
) -> NoiseNode {
    let mountains_amount = settings.get_mountains_amount();

    // 1: [Continents-and-mountains module]: This addition module adds the
    // scaled-mountainous-terrain group to the base-continent-elevation
    // subgroup.
    let ad0 = Add::new(
        base_continent_elev.clone(),
        scaled_mountainous_terrain.clone(),
    );

    // 2: [Increase-mountain-heights module]: This curve module applies a curve
    // to the output value from the continent-definition group. This modified
    // output value is used by a subsequent noise function to add additional
    // height to the mountains based on the current continent elevation. The
    // higher the continent elevation, the higher the mountains.
    let cu = Curve::new(continent_def.clone())
        .add_control_point(-1.0, -0.0625)
        .add_control_point(0.0, 0.0000)
        .add_control_point(1.0 - mountains_amount, 0.0625)
        .add_control_point(1.0, 0.2500);

    // 3: [Add-increased-mountain-heights module]: This addition module adds the
    // increased-mountain-heights module to the continents-and-mountains module.
    // The highest continent elevations now have the highest mountains.
    let ad1 = Add::new(ad0, cu);

    // 4: [Select-high-elevations module]: This selector module ensures that
    // mountains only appear at higher elevations. It does this by selecting the
    // output value from the continent-with-mountains module if the
    // corresponding output value from the terrain-type-definition group is
    // above a certain value. Otherwise, it selects the output value from the
    // continents-with-hills subgroup. Note that the continents-with-hills
    // subgroup also contains the plains terrain.
    let se = Select::new(continents_with_hills.clone(), ad1, terrain_type_def.clone())
        .set_bounds(1.0 - mountains_amount, 1001.0 - mountains_amount)
        .set_falloff(0.25);

    // 5: [Continents-with-mountains subgroup]: Caches the output value from the
    // select-high-elevations module.
    NoiseNode::cached(se)
}

/// Function subgroup: continents with badlands (5 noise functions)
///
/// This subgroup applies the scaled-badlands-terrain group to the
/// continents-with-mountains subgroup.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continents_with_badlands(
    settings: &MapGeneratorSettings,
    base_continent_elev: &NoiseNode,
    scaled_badlands_terrain: &NoiseNode,
    continents_with_mountains: &NoiseNode,
) -> NoiseNode {
    let seed = settings.get_seed();
    let continent_lacunarity = settings.get_continent_lacunarity();
    let badlands_amount = settings.get_badlands_amount();

    // 1: [Badlands-positions module]: This BasicMulti module generates some
    // random noise, which is used by subsequent noise functions to specify the
    // locations of the badlands.
    let bm = Fbm::<Perlin>::new(seed + 140)
        .set_frequency(16.5)
        .set_persistence(0.5)
        .set_lacunarity(continent_lacunarity)
        .set_octaves(2);

    // 2: [Continents-and-badlands module]:  This addition module adds the
    // scaled-badlands-terrain group to the base-continent-elevation
    // subgroup.
    let ad = Add::new(base_continent_elev.clone(), scaled_badlands_terrain.clone());

    // 3: [Select-badlands-positions module]: This selector module places
    // badlands at random spots on the continents based on the BasicMulti noise
    // generated by the badlands-positions module. To do this, it selects the
    // output value from the continents-and-badlands module if the corresponding
    // output value from the badlands-position module is greater than a
    // specified value. Otherwise, this selector module selects the output value
    // from the continents-with-mountains subgroup. There is also a wide
    // transition between these two noise functions so that the badlands can blend
    // into the rest of the terrain on the continents.
    let se = Select::new(continents_with_mountains.clone(), ad, bm)
        .set_bounds(1.0 - badlands_amount, 1001.0 - badlands_amount)
        .set_falloff(0.25);

    // 4: [Apply-badlands module]: This maximum-value module causes the badlands
    // to "poke out" from the rest of the terrain. It does this by ensuring
    // that only the maximum of the output values from the continents-with-
    // mountains subgroup and the select-badlands-positions modules contribute
    // to the output value of this subgroup. One side effect of this process is
    // that the badlands will not appear in mountainous terrain.
    let ma = Max::new(continents_with_mountains.clone(), se);

    // 5: [Continents-with-badlands subgroup]: Caches the output value from the
    //    apply-badlands module.
    NoiseNode::cached(ma)
}

/// Function subgroup: continents with rivers (4 noise functions)
///
/// This subgroup applies the river-positions group to the continents-with-
/// badlands subgroup.
///
/// The output value from this module subgroup is measured in planetary
/// elevation units (-1.0 for the lowest underwater trenches and +1.0 for the
/// highest mountain peaks.)
fn build_continents_with_rivers(
    settings: &MapGeneratorSettings,
    river_positions: &NoiseNode,
    continents_with_badlands: &NoiseNode,
) -> NoiseNode {
    let river_depth = settings.get_river_depth();
    let sea_level = settings.get_sea_level();
    let continent_height_scale = settings.get_continent_height_scale();

    // 1: [Scaled-rivers module]: This scale/bias module scales the output value
    // from the river-positions group so that it is measured in planetary
    // elevation units and is negative; this is required for step 2.
    let sb = ScaleBias::new(river_positions.clone())
        .set_scale(river_depth / 2.0)
        .set_bias(-river_depth / 2.0);

    // 2: [Add-rivers-to-continents module]: This addition module adds the
    // rivers to the continents-with-badlands subgroup. Because the scaled-
    // rivers module only outputs a negative value, the scaled-rivers module
    // carves the rivers out of the terrain.
    let ad = Add::new(continents_with_badlands.clone(), sb);

    // 3: [Blended-rivers-to-continents module]: This selector module outputs
    // deep rivers near sea level and shallower rivers in higher terrain.  It
    // does this by selecting the output value from the continents-with-
    // badlands subgroup if the corresponding output value from the
    // continents-with-badlands subgroup is far from sea level.  Otherwise,
    // this selector module selects the output value from the add-rivers-to-
    // continents module.
    let se = Select::new(
        continents_with_badlands.clone(),
        ad,
        continents_with_badlands.clone(),
    )
    .set_bounds(sea_level, continent_height_scale + sea_level)
    .set_falloff(continent_height_scale - sea_level);

    // 4: [Continents-with-rivers subgroup]: Caches the output value from the
    // blended-rivers-to-continents module.
    NoiseNode::cached(se)
}

/// Function subgroup: unscaled final planet (1 noise function)
///
/// This subgroup simply caches the output value from the continent-with-
/// rivers subgroup to contribute to the final output value.
fn build_unscaled_final_planet(continents_with_rivers: &NoiseNode) -> NoiseNode {
    // 1: [Unscaled-final-planet subgroup]: Caches the output value from the
    //    continent-with-rivers subgroup.
    NoiseNode::cached(continents_with_rivers.clone())
}
