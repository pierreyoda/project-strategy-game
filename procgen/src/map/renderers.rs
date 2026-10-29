//! Debugging renderers.

use noise::utils::ColorGradient;

use super::{
    MapGeneratorSettings,
    terrain::{TERRAIN_GRADIENT_BOUNDS, elevation_band_bounds},
};

#[cfg(feature = "debug-images")]
pub mod image;
pub mod text;

/// Elevation bands and colors of elevations in meters, following noise's terrain gradient.
struct TerrainLegend {
    bounds: [f64; TERRAIN_GRADIENT_BOUNDS.len()],
    scale: f64,
    bias: f64,
    sea_level: f64,
    gradient: ColorGradient,
}

impl TerrainLegend {
    /// Number of elevation bands.
    const BANDS: usize = TERRAIN_GRADIENT_BOUNDS.len() + 1;

    fn new(settings: &MapGeneratorSettings) -> Self {
        let (scale, bias) = settings.get_elevation_scale_bias();
        let sea_level = settings.get_sea_level();
        Self {
            bounds: elevation_band_bounds(settings),
            scale,
            bias,
            sea_level,
            gradient: ColorGradient::new().build_terrain_gradient(),
        }
    }

    /// Upper bounds of each band but the last one, in meters.
    fn bounds(&self) -> [f64; TERRAIN_GRADIENT_BOUNDS.len()] {
        self.bounds
    }

    /// Band of the given elevation in meters, from 0 (the deepest) to `BANDS - 1` (the highest).
    fn band(&self, elevation: i16) -> usize {
        let elevation = elevation as f64;
        self.bounds
            .iter()
            .position(|&bound| elevation < bound)
            .unwrap_or(self.bounds.len())
    }

    fn color(&self, elevation: i16) -> [u8; 3] {
        // back to a position on the gradient: planetary elevation units, around sea level
        let position = (elevation as f64 - self.bias) / self.scale - self.sea_level;
        // noise's gradient gives transparent black right at its upper bound (1.0), unlike beyond
        let position = if position >= 1.0 {
            f64::INFINITY
        } else {
            position
        };
        let [r, g, b, _] = self.gradient.get_color(position);
        [r, g, b]
    }
}

/// Range of raw noise values, to spread them between their minimum and their maximum.
struct ValueRange {
    min: f64,
    max: f64,
}

impl ValueRange {
    fn of(values: &[f64]) -> Self {
        Self {
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }

    /// Position of the given value in the range, from 0.0 (minimum) to 1.0 (maximum).
    fn ratio(&self, value: f64) -> f64 {
        let range = self.max - self.min;
        if range > 0.0 {
            (value - self.min) / range
        } else {
            0.0
        }
    }

    /// From black (minimum) to white (maximum).
    fn gray(&self, value: f64) -> [u8; 3] {
        [(self.ratio(value) * 255.0).round() as u8; 3]
    }
}

#[cfg(test)]
mod tests {
    use crate::map::MapGeneratorSettings;

    use super::{TerrainLegend, ValueRange};

    #[test]
    fn test_terrain_legend_bands() {
        let legend = TerrainLegend::new(&MapGeneratorSettings::default());
        assert_eq!(
            legend.bounds(),
            [-128.0, 0.0, 512.0, 1024.0, 2048.0, 3072.0]
        );
        let cases = [
            (i16::MIN, 0),
            (-129, 0),
            (-128, 1),
            (-1, 1),
            (0, 2),
            (511, 2),
            (512, 3),
            (1023, 3),
            (1024, 4),
            (2047, 4),
            (2048, 5),
            (3071, 5),
            (3072, 6),
            (i16::MAX, 6),
        ];
        for (elevation, band) in cases {
            assert_eq!(legend.band(elevation), band, "{elevation} m");
        }
    }

    #[test]
    fn test_terrain_legend_colors_follow_terrain_gradient() {
        let legend = TerrainLegend::new(&MapGeneratorSettings::default());
        assert_eq!(legend.color(-8192), [0, 0, 0]);
        assert_eq!(legend.color(-128), [6, 58, 127]);
        assert_eq!(legend.color(0), [70, 120, 60]);
        assert_eq!(legend.color(8192), [255, 255, 255]);
        assert_eq!(legend.color(i16::MAX), [255, 255, 255]);
    }

    #[test]
    fn test_value_range() {
        let range = ValueRange::of(&[0.5, -1.5, 2.5, 0.0]);
        assert_eq!((range.min, range.max), (-1.5, 2.5));
        assert_eq!(range.ratio(-1.5), 0.0);
        assert_eq!(range.ratio(0.5), 0.5);
        assert_eq!(range.ratio(2.5), 1.0);
        assert_eq!(range.gray(-1.5), [0; 3]);
        assert_eq!(range.gray(2.5), [255; 3]);

        let constant = ValueRange::of(&[3.0, 3.0]);
        assert_eq!(constant.ratio(3.0), 0.0);
    }
}
