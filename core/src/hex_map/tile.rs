#[derive(Debug)]
pub struct HexMapTile {
    // In meters.
    elevation: i16,
}

impl HexMapTile {
    pub fn from_properties(elevation: i16) -> Self {
        Self { elevation }
    }
}
