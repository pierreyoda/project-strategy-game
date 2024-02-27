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
}
