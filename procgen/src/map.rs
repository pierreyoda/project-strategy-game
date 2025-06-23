use settings::MapGeneratorSettings;

mod groups;
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

    pub fn generate<F>(on_event: F)
    where
        F: Fn(MapGeneratorEvent) -> (),
    {
    }
}
