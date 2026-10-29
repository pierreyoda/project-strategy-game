use super::layers::{dynamic::HexMapArtificialTileData, natural::HexMapNaturalTileData};

#[derive(Debug)]
pub struct HexMapTile {
    // In meters.
    elevation: i16,
    layer_natural: HexMapNaturalTileData,
    layer_artificial: HexMapArtificialTileData,
}

impl HexMapTile {
    pub fn from_properties(elevation: i16) -> Self {
        Self {
            elevation,
            layer_natural: HexMapNaturalTileData::new(),
            layer_artificial: HexMapArtificialTileData::new(),
        }
    }

    /// Elevation, in meters.
    pub fn elevation(&self) -> i16 {
        self.elevation
    }

    pub fn natural(&self) -> &HexMapNaturalTileData {
        &self.layer_natural
    }

    pub fn natural_mut(&mut self) -> &mut HexMapNaturalTileData {
        &mut self.layer_natural
    }

    pub fn artificial(&self) -> &HexMapArtificialTileData {
        &self.layer_artificial
    }

    pub fn artificial_mut(&mut self) -> &mut HexMapArtificialTileData {
        &mut self.layer_artificial
    }
}
