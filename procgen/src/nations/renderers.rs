//! Debugging renderers for the generated nations, as more views of the map's own renderers:
//! `TextRenderer::render_nations` and, with the `debug-images` feature,
//! `ImageRenderer::render_nations`.

use project_x_core::{game::state::TileIndex, simulation::ids::EntityIndex};

use crate::map::TerrainKind;

use super::{GeneratedNations, PlanetSurface};

#[cfg(feature = "debug-images")]
pub mod image;
pub mod text;

const WATER: [u8; 3] = [28, 52, 96];
const ICE: [u8; 3] = [214, 224, 232];
const UNCLAIMED: [u8; 3] = [118, 112, 100];

/// What a tile is to the nations, and how to draw it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Cell {
    Water,
    Ice,
    /// Land no sphere reaches.
    Unclaimed,
    /// Within a nation's sphere of influence only.
    Sphere(usize),
    /// Owned on turn 1.
    Territory(usize),
    Capital(usize),
}

impl Cell {
    fn of(nations: &GeneratedNations, surface: &PlanetSurface, index: TileIndex) -> Self {
        if !surface.is_land(index) {
            return Self::Water;
        }
        let Some(owner) = nations.spheres().owner(index) else {
            return match surface.terrain_at(index) {
                TerrainKind::Ice | TerrainKind::RoughIce => Self::Ice,
                _ => Self::Unclaimed,
            };
        };
        let nation = owner.index();
        let generated = &nations.nations()[nation];
        if surface.index(generated.capital()) == Some(index) {
            Self::Capital(nation)
        } else if generated.territory().binary_search(&index).is_ok() {
            Self::Territory(nation)
        } else {
            Self::Sphere(nation)
        }
    }

    /// Letter of a nation: `a` to `z`, then around again.
    fn letter(nation: usize) -> char {
        (b'a' + (nation % 26) as u8) as char
    }

    fn glyph(self) -> char {
        match self {
            Self::Water => '~',
            Self::Ice => '*',
            Self::Unclaimed => '.',
            Self::Sphere(nation) => Self::letter(nation),
            Self::Territory(nation) => Self::letter(nation).to_ascii_uppercase(),
            Self::Capital(_) => '@',
        }
    }

    fn color(self) -> [u8; 3] {
        match self {
            Self::Water => WATER,
            Self::Ice => ICE,
            Self::Unclaimed => UNCLAIMED,
            Self::Sphere(nation) => dim(nation_color(nation)),
            Self::Territory(nation) | Self::Capital(nation) => nation_color(nation),
        }
    }
}

/// A color per nation, hues spread by the golden angle so that neighbors in the numbering are
/// far apart on the wheel.
fn nation_color(nation: usize) -> [u8; 3] {
    let hue = (nation as f64 * 137.507_764).rem_euclid(360.0);
    hsv_to_rgb(hue, 0.65, 0.85)
}

/// The same color, in the shade: a sphere, not a possession.
fn dim(color: [u8; 3]) -> [u8; 3] {
    color.map(|channel| (f64::from(channel) * 0.45 + 28.0) as u8)
}

fn hsv_to_rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let chroma = value * saturation;
    let sector = hue / 60.0;
    let x = chroma * (1.0 - (sector.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, x, 0.0),
        1 => (x, chroma, 0.0),
        2 => (0.0, chroma, x),
        3 => (0.0, x, chroma),
        4 => (x, 0.0, chroma),
        _ => (chroma, 0.0, x),
    };
    let m = value - chroma;
    [r, g, b].map(|channel| ((channel + m) * 255.0).round() as u8)
}

#[cfg(test)]
mod tests {
    use super::{Cell, hsv_to_rgb, nation_color};

    #[test]
    fn test_nation_letters_wrap_around_the_alphabet() {
        assert_eq!(Cell::letter(0), 'a');
        assert_eq!(Cell::letter(25), 'z');
        assert_eq!(Cell::letter(26), 'a');
        assert_eq!(Cell::Territory(1).glyph(), 'B');
    }

    #[test]
    fn test_nation_colors_differ_between_neighbors() {
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv_to_rgb(240.0, 1.0, 1.0), [0, 0, 255]);
        let colors: Vec<_> = (0..8).map(nation_color).collect();
        for (i, color) in colors.iter().enumerate() {
            for other in &colors[..i] {
                let distance: u32 = color
                    .iter()
                    .zip(other)
                    .map(|(&a, &b)| u32::from(a.abs_diff(b)))
                    .sum();
                assert!(distance > 60, "{color:?} vs {other:?}");
            }
        }
    }
}
