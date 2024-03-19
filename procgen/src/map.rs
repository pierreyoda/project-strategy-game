#[derive(Debug)]
pub struct MapGeneratorSettings {
    width: usize,
    height: usize,
}

pub struct MapGenerator {
    settings: MapGeneratorSettings,
}

impl MapGenerator {
    pub fn from_settings(settings: MapGeneratorSettings) -> Self {
        Self { settings }
    }
}
