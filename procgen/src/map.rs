use project_x_core::hex_map::{HexMap, shape::RectangularHexMapShape};

pub use self::{
    graph::{PlanetGraph, SubGroupId},
    node::NoiseNode,
    projection::CylinderProjection,
    sampling::ElevationField,
    settings::{MAX_SEED, MapGeneratorSeedScalar, MapGeneratorSettings},
};

mod graph;
mod groups;
mod node;
mod projection;
pub mod renderers;
mod sampling;
mod settings;

#[derive(Debug)]
pub enum MapGeneratorEvent {
    /// Latest absolute progress percentage.
    Progress(u8),
}

#[derive(Debug)]

pub struct MapGenerator {
    settings: MapGeneratorSettings,
}

impl MapGenerator {
    pub fn from_settings(settings: MapGeneratorSettings) -> Self {
        Self { settings }
    }

    /// Generate the map, reporting the progress to `on_event` along the way.
    ///
    /// Almost all the time is spent sampling the noise functions: the progress goes
    /// from 0% to 100% as the rows get sampled, each percentage being reported once.
    pub fn generate<F>(&self, mut on_event: F) -> HexMap
    where
        F: FnMut(MapGeneratorEvent),
    {
        let settings = &self.settings;
        let shape = RectangularHexMapShape::new(settings.get_columns(), settings.get_rows());
        on_event(MapGeneratorEvent::Progress(0));

        let graph = PlanetGraph::build(settings);
        let rows = shape.rows() as usize;
        let mut progress = 0;
        let field = ElevationField::sample(shape, graph.unscaled_final_planet(), |row| {
            let row_progress = ((row as usize + 1) * 100 / rows) as u8;
            if row_progress > progress {
                progress = row_progress;
                on_event(MapGeneratorEvent::Progress(progress));
            }
        });

        field.into_hex_map(settings)
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::hex_map::{HexMap, shape::RectangularHexMapShape};

    use super::{
        MapGenerator, MapGeneratorEvent, MapGeneratorSettings, renderers::text::TextRenderer,
    };

    fn generate(settings: MapGeneratorSettings) -> (HexMap, Vec<u8>) {
        let mut progress = Vec::new();
        let map = MapGenerator::from_settings(settings).generate(|event| {
            let MapGeneratorEvent::Progress(percentage) = event;
            progress.push(percentage);
        });
        (map, progress)
    }

    fn elevations(map: &HexMap, shape: RectangularHexMapShape) -> Vec<i16> {
        shape
            .coords()
            .map(|coords| map.tile(&coords).expect("missing tile").elevation())
            .collect()
    }

    #[test]
    fn test_map_generator_generates_a_planet() {
        let shape = RectangularHexMapShape::new(16, 10);
        let (map, progress) = generate(MapGeneratorSettings::new(0, 16, 10));
        let settings = MapGeneratorSettings::new(0, 16, 10);
        println!(
            "{}",
            TextRenderer::from_env().render_hex_map(&map, &settings)
        );

        assert_eq!(progress, [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]);
        let elevations = elevations(&map, shape);
        assert!(
            elevations.iter().any(|&elevation| elevation < 0),
            "no ocean"
        );
        assert!(elevations.iter().any(|&elevation| elevation > 0), "no land");
    }

    #[test]
    fn test_map_generator_reports_each_percentage_once() {
        // more rows than percentages
        let (_, progress) = generate(MapGeneratorSettings::new(0, 3, 150));
        assert_eq!(progress, (0..=100).collect::<Vec<u8>>());
    }

    #[test]
    fn test_map_generator_is_deterministic() {
        let shape = RectangularHexMapShape::new(16, 10);
        let (map, _) = generate(MapGeneratorSettings::new(42, 16, 10));
        let (same_seed_map, _) = generate(MapGeneratorSettings::new(42, 16, 10));
        let (other_seed_map, _) = generate(MapGeneratorSettings::new(43, 16, 10));

        assert_eq!(elevations(&map, shape), elevations(&same_seed_map, shape));
        assert_ne!(elevations(&map, shape), elevations(&other_seed_map, shape));
    }
}
