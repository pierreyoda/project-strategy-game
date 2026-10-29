//! Terrain classification of the tiles of a generated map.

use project_x_core::hex_map::coordinates::{CubeCoordsScalar, OffsetCoords};

use super::settings::MapGeneratorSettings;

/// Upper bounds of each elevation band but the last one, taken from the positions of
/// noise's terrain gradient (in planetary elevation units, relative to sea level).
pub(crate) const TERRAIN_GRADIENT_BOUNDS: [f64; 6] = [
    -256.0 / 16384.0,
    0.0,
    1024.0 / 16384.0,
    2048.0 / 16384.0,
    4096.0 / 16384.0,
    6144.0 / 16384.0,
];

/// Share of the rows covered in ice at each pole.
const POLAR_ROWS_RATIO: f64 = 0.05;

/// Upper bounds of each elevation band but the last one, in meters.
pub(crate) fn elevation_band_bounds(
    settings: &MapGeneratorSettings,
) -> [f64; TERRAIN_GRADIENT_BOUNDS.len()] {
    let (scale, bias) = settings.get_elevation_scale_bias();
    let sea_level = settings.get_sea_level();
    TERRAIN_GRADIENT_BOUNDS.map(|bound| (sea_level + bound).mul_add(scale, bias))
}

/// Terrain of a tile.
///
/// The discriminants are part of the WASM bridge's API, mirrored by the client's `TerrainKind`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TerrainKind {
    DeepWater = 0,
    ShallowWater = 1,
    Flat = 2,
    Hills = 3,
    Mountains = 4,
    Ice = 5,
    /// Ice at the elevations of hills and mountains.
    RoughIce = 6,
}

/// Classifies the tiles of a map generated with the given settings, from their elevation
/// and their latitude.
#[derive(Copy, Clone, Debug)]
pub struct TerrainClassifier {
    rows: CubeCoordsScalar,
    /// Number of rows covered in ice at each pole.
    polar_rows: CubeCoordsScalar,
    /// In meters: deep water below the first bound, then shallow water, flat terrain, hills,
    /// and mountains from the last bound.
    bounds: [f64; 4],
}

impl TerrainClassifier {
    pub fn new(settings: &MapGeneratorSettings) -> Self {
        let rows = settings.get_rows();
        let polar_rows = (f64::from(rows) * POLAR_ROWS_RATIO).round() as CubeCoordsScalar;
        // elevation bands: deep water, shallow water, two of flat terrain, hills, two of mountains
        let bands = elevation_band_bounds(settings);
        Self {
            rows,
            polar_rows: polar_rows.max(1),
            bounds: [bands[0], bands[1], bands[3], bands[4]],
        }
    }

    /// Terrain of the tile at the given coordinates, with the given elevation in meters.
    pub fn classify(&self, coords: OffsetCoords, elevation: i16) -> TerrainKind {
        let elevation = f64::from(elevation);
        let kind = match self.bounds.iter().position(|&bound| elevation < bound) {
            Some(0) => TerrainKind::DeepWater,
            Some(1) => TerrainKind::ShallowWater,
            Some(2) => TerrainKind::Flat,
            Some(3) => TerrainKind::Hills,
            _ => TerrainKind::Mountains,
        };

        let row = coords.row();
        if row >= self.polar_rows && row < self.rows - self.polar_rows {
            kind
        } else if matches!(kind, TerrainKind::Hills | TerrainKind::Mountains) {
            TerrainKind::RoughIce
        } else {
            TerrainKind::Ice
        }
    }
}

#[cfg(test)]
mod tests {
    use project_x_core::hex_map::coordinates::OffsetCoords;

    use super::{TerrainClassifier, TerrainKind};
    use crate::map::MapGeneratorSettings;

    #[test]
    fn test_terrain_kind_discriminants() {
        // part of the WASM bridge's API
        let kinds = [
            TerrainKind::DeepWater,
            TerrainKind::ShallowWater,
            TerrainKind::Flat,
            TerrainKind::Hills,
            TerrainKind::Mountains,
            TerrainKind::Ice,
            TerrainKind::RoughIce,
        ];
        for (discriminant, kind) in kinds.into_iter().enumerate() {
            assert_eq!(kind as u8, discriminant as u8, "{kind:?}");
        }
    }

    #[test]
    fn test_terrain_classifier_follows_elevation_bands() {
        let classifier = TerrainClassifier::new(&MapGeneratorSettings::default());
        let equator = OffsetCoords::new(0, 26);
        let cases = [
            (i16::MIN, TerrainKind::DeepWater),
            (-129, TerrainKind::DeepWater),
            (-128, TerrainKind::ShallowWater),
            (-1, TerrainKind::ShallowWater),
            (0, TerrainKind::Flat),
            (1023, TerrainKind::Flat),
            (1024, TerrainKind::Hills),
            (2047, TerrainKind::Hills),
            (2048, TerrainKind::Mountains),
            (i16::MAX, TerrainKind::Mountains),
        ];
        for (elevation, kind) in cases {
            assert_eq!(
                classifier.classify(equator, elevation),
                kind,
                "{elevation} m"
            );
        }
    }

    #[test]
    fn test_terrain_classifier_covers_poles_in_ice() {
        // 52 rows: 3 polar rows at each pole
        let classifier = TerrainClassifier::new(&MapGeneratorSettings::default());
        for row in 0..52 {
            let ice = classifier.classify(OffsetCoords::new(0, row), 0) == TerrainKind::Ice;
            assert_eq!(ice, !(3..49).contains(&row), "row {row}");
        }

        // at least one polar row at each pole
        let classifier = TerrainClassifier::new(&MapGeneratorSettings::new(0, 4, 3));
        let kinds = (0..3).map(|row| classifier.classify(OffsetCoords::new(0, row), 0));
        assert_eq!(
            kinds.collect::<Vec<_>>(),
            [TerrainKind::Ice, TerrainKind::Flat, TerrainKind::Ice]
        );
    }

    #[test]
    fn test_terrain_classifier_roughens_ice_at_hills_elevations() {
        let classifier = TerrainClassifier::new(&MapGeneratorSettings::default());
        let pole = OffsetCoords::new(0, 0);
        let cases = [
            (i16::MIN, TerrainKind::Ice),
            (-1, TerrainKind::Ice),
            (1023, TerrainKind::Ice),
            (1024, TerrainKind::RoughIce),
            (2048, TerrainKind::RoughIce),
            (i16::MAX, TerrainKind::RoughIce),
        ];
        for (elevation, kind) in cases {
            assert_eq!(classifier.classify(pole, elevation), kind, "{elevation} m");
        }
    }
}
