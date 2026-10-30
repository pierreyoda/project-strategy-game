//! Nations generation: who lives on a generated planet, where they start, and how far their
//! influence reaches.
//!
//! Five passes over the same row-major tiles as the client's buffers, none of them touching a
//! `World`: landmasses, habitability, apportionment, capitals, then spheres of influence and the
//! territory owned on turn 1. The output is data for the caller - the WASM bridge, a test - to
//! write into the world through its own gate, the way the map generator hands over a `HexMap`.

use project_x_core::{
    game::state::TileIndex, hex_map::coordinates::CubeCoords, simulation::ids::NationId,
};

pub use self::{
    capitals::{Capital, SitingReport},
    habitability::Habitability,
    landmasses::{LandmassId, Landmasses},
    settings::{NationsGeneratorSettings, PerTerrain},
    surface::PlanetSurface,
    territory::Spheres,
};

mod capitals;
mod habitability;
mod landmasses;
pub mod renderers;
mod settings;
mod surface;
mod territory;

/// A nation, as generated: a start, not an identity. Names, definitions and leaders are the
/// caller's.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratedNation {
    /// `NationId::from_index(slot)`: the caller registers the nations in this order.
    id: NationId,
    capital: CubeCoords,
    landmass: LandmassId,
    /// Owned on turn 1, in row-major order.
    territory: Vec<TileIndex>,
    /// The whole sphere of influence, in row-major order: where the nation is meant to grow.
    sphere: Vec<TileIndex>,
    /// Habitability of the capital's site.
    site_score: f64,
}

impl GeneratedNation {
    pub fn id(&self) -> NationId {
        self.id
    }

    pub fn capital(&self) -> CubeCoords {
        self.capital
    }

    pub fn landmass(&self) -> LandmassId {
        self.landmass
    }

    pub fn territory(&self) -> &[TileIndex] {
        &self.territory
    }

    pub fn sphere(&self) -> &[TileIndex] {
        &self.sphere
    }

    pub fn site_score(&self) -> f64 {
        self.site_score
    }
}

/// How the generation went: what tests assert on, and what a balance pass would read.
#[derive(Clone, Debug, PartialEq)]
pub struct GenerationReport {
    pub siting: SitingReport,
    /// Land tiles no sphere reaches: islands without a capital.
    pub unclaimed_land: usize,
}

/// The nations of a planet.
#[derive(Clone, Debug)]
pub struct GeneratedNations {
    nations: Vec<GeneratedNation>,
    spheres: Spheres,
    landmasses: Landmasses,
    report: GenerationReport,
}

impl GeneratedNations {
    /// In the order of their IDs.
    pub fn nations(&self) -> &[GeneratedNation] {
        &self.nations
    }

    pub fn spheres(&self) -> &Spheres {
        &self.spheres
    }

    pub fn landmasses(&self) -> &Landmasses {
        &self.landmasses
    }

    pub fn report(&self) -> &GenerationReport {
        &self.report
    }

    /// The nation whose turn-1 territory holds the tile, if any.
    pub fn owner_of(&self, index: TileIndex) -> Option<NationId> {
        self.nations
            .iter()
            .find(|nation| nation.territory.binary_search(&index).is_ok())
            .map(GeneratedNation::id)
    }
}

#[derive(Debug)]
pub struct NationsGenerator {
    settings: NationsGeneratorSettings,
}

impl NationsGenerator {
    pub fn from_settings(settings: NationsGeneratorSettings) -> Self {
        Self { settings }
    }

    pub fn settings(&self) -> &NationsGeneratorSettings {
        &self.settings
    }

    pub fn generate(&self, surface: &PlanetSurface) -> GeneratedNations {
        let settings = &self.settings;
        let landmasses = Landmasses::find(surface);
        let habitability = Habitability::compute(surface, settings);
        let (capitals, siting) =
            capitals::place_capitals(surface, &landmasses, &habitability, settings);
        let spheres = Spheres::carve(surface, &capitals, settings);

        let nations = capitals
            .iter()
            .map(|capital| GeneratedNation {
                id: capital.nation,
                capital: surface.coords(capital.tile),
                landmass: capital.landmass,
                territory: territory::starting_territory(surface, &spheres, capital, settings),
                sphere: spheres.tiles_of(capital.nation),
                site_score: capital.site_score,
            })
            .collect();
        let unclaimed_land = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.is_land(index) && spheres.owner(index).is_none())
            .count();

        GeneratedNations {
            nations,
            spheres,
            landmasses,
            report: GenerationReport {
                siting,
                unclaimed_land,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::{
        game::state::TileIndex,
        simulation::ids::{EntityIndex, NationId},
    };

    use super::{NationsGenerator, NationsGeneratorSettings, PlanetSurface};
    use crate::{
        map::renderers::text::TextRenderer,
        map::{MapGenerator, MapGeneratorSettings},
    };

    fn planet(seed: u32, columns: i16, rows: i16) -> PlanetSurface {
        let settings = MapGeneratorSettings::new(seed, columns, rows);
        let map = MapGenerator::from_settings(MapGeneratorSettings::new(seed, columns, rows))
            .generate(|_| {});
        PlanetSurface::classify(&map, &settings)
    }

    #[test]
    fn test_nations_generator_settles_a_planet() {
        let surface = planet(3, 40, 26);
        let settings = NationsGeneratorSettings::new(3, 6);
        let generated = NationsGenerator::from_settings(settings).generate(&surface);
        println!(
            "{}",
            TextRenderer::from_env().render_nations(&generated, &surface)
        );

        let nations = generated.nations();
        assert_eq!(nations.len(), 6);
        assert_eq!(generated.report().siting.unplaced, 0);
        for (index, nation) in nations.iter().enumerate() {
            assert_eq!(nation.id(), NationId::from_index(index));
            let capital = surface.index(nation.capital()).unwrap();
            assert!(surface.is_land(capital), "capital on land");
            assert!(nation.site_score() > 0.0, "capital somewhere habitable");
            assert!(
                nation.territory().contains(&capital),
                "the capital is owned"
            );
            assert!(
                nation
                    .territory()
                    .iter()
                    .all(|tile| nation.sphere().binary_search(tile).is_ok()),
                "territory within the sphere"
            );
            assert!(
                nation.sphere().iter().all(|&tile| surface.is_land(tile)),
                "spheres are land"
            );
            assert_eq!(generated.spheres().owner(capital), Some(nation.id()));
            assert_eq!(generated.landmasses().of(capital), Some(nation.landmass()));
        }
        // the spheres partition the land every capital can reach
        let claimed: usize = nations.iter().map(|nation| nation.sphere().len()).sum();
        let land = (0..surface.tiles_count() as TileIndex)
            .filter(|&index| surface.is_land(index))
            .count();
        assert_eq!(claimed + generated.report().unclaimed_land, land);
    }

    #[test]
    fn test_nations_generator_is_deterministic() {
        let surface = planet(42, 24, 16);
        let generate = |seed| {
            NationsGenerator::from_settings(NationsGeneratorSettings::new(seed, 4))
                .generate(&surface)
                .nations()
                .to_vec()
        };
        assert_eq!(generate(1), generate(1));
    }

    #[test]
    fn test_nations_owner_of_reads_the_turn_1_territory() {
        let surface = planet(3, 24, 16);
        let generated =
            NationsGenerator::from_settings(NationsGeneratorSettings::new(0, 2)).generate(&surface);
        let first = &generated.nations()[0];
        let capital = surface.index(first.capital()).unwrap();
        assert_eq!(generated.owner_of(capital), Some(first.id()));
        let unowned = (0..surface.tiles_count() as TileIndex)
            .find(|&index| !surface.is_land(index))
            .unwrap();
        assert_eq!(generated.owner_of(unowned), None);
    }
}
