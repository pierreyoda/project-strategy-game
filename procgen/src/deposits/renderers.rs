//! Debugging renderers for the generated deposits, as more views of the map's own renderers:
//! `TextRenderer::render_deposits` and, with the `debug-images` feature,
//! `ImageRenderer::render_deposits`.

use project_x_core::game::state::TileIndex;

use crate::map::{PlanetSurface, TerrainKind};

use super::{DepositKind, DepositOrigin, GeneratedDeposits};

#[cfg(feature = "debug-images")]
pub mod image;
pub mod text;

const WATER: [u8; 3] = [28, 52, 96];
const ICE: [u8; 3] = [214, 224, 232];
const FLAT: [u8; 3] = [104, 120, 84];
const HILLS: [u8; 3] = [128, 116, 88];
const MOUNTAINS: [u8; 3] = [92, 84, 80];
const CAPITAL: [u8; 3] = [240, 240, 240];

/// What a tile shows, and how to draw it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Cell {
    Terrain(TerrainKind),
    Deposit(DepositKind, DepositOrigin),
    Capital,
}

impl Cell {
    /// The capital over the deposit over the terrain. Of a tile with two deposits, the first.
    fn of(
        deposits: &GeneratedDeposits,
        surface: &PlanetSurface,
        capitals: &[TileIndex],
        index: TileIndex,
    ) -> Self {
        if capitals.contains(&index) {
            return Self::Capital;
        }
        match deposits.on(index).next() {
            Some(deposit) => Self::Deposit(deposit.kind(), deposit.origin()),
            None => Self::Terrain(surface.terrain_at(index)),
        }
    }

    /// Lowercase for a natural deposit, uppercase for one laid on purpose.
    fn glyph(self) -> char {
        match self {
            Self::Terrain(TerrainKind::DeepWater | TerrainKind::ShallowWater) => '~',
            Self::Terrain(TerrainKind::Flat) => '.',
            Self::Terrain(TerrainKind::Hills) => ':',
            Self::Terrain(TerrainKind::Mountains) => '^',
            Self::Terrain(TerrainKind::Ice | TerrainKind::RoughIce) => '*',
            Self::Deposit(kind, origin) => {
                let letter = kind_letter(kind);
                if origin == DepositOrigin::Natural {
                    letter
                } else {
                    letter.to_ascii_uppercase()
                }
            }
            Self::Capital => '@',
        }
    }

    fn color(self) -> [u8; 3] {
        match self {
            Self::Terrain(TerrainKind::DeepWater | TerrainKind::ShallowWater) => WATER,
            Self::Terrain(TerrainKind::Flat) => FLAT,
            Self::Terrain(TerrainKind::Hills) => HILLS,
            Self::Terrain(TerrainKind::Mountains) => MOUNTAINS,
            Self::Terrain(TerrainKind::Ice | TerrainKind::RoughIce) => ICE,
            Self::Deposit(kind, _) => kind_color(kind),
            Self::Capital => CAPITAL,
        }
    }
}

fn kind_letter(kind: DepositKind) -> char {
    match kind {
        DepositKind::Water => 'w',
        DepositKind::Metals => 'm',
        DepositKind::Oil => 'o',
        DepositKind::Uranium => 'u',
    }
}

/// The client's resource colors, where it has them.
fn kind_color(kind: DepositKind) -> [u8; 3] {
    match kind {
        DepositKind::Water => [64, 160, 230],
        DepositKind::Metals => [169, 182, 196],
        DepositKind::Oil => [224, 138, 74],
        DepositKind::Uranium => [140, 220, 60],
    }
}

#[cfg(test)]
mod tests {
    use super::{Cell, kind_color};
    use crate::{
        deposits::{DepositKind, DepositOrigin},
        map::TerrainKind,
    };

    #[test]
    fn test_cells_tell_natural_deposits_from_those_laid_on_purpose() {
        assert_eq!(
            Cell::Deposit(DepositKind::Metals, DepositOrigin::Natural).glyph(),
            'm'
        );
        assert_eq!(
            Cell::Deposit(DepositKind::Water, DepositOrigin::Guaranteed).glyph(),
            'W'
        );
        assert_eq!(
            Cell::Deposit(DepositKind::Oil, DepositOrigin::Balancing).glyph(),
            'O'
        );
        assert_eq!(Cell::Terrain(TerrainKind::Mountains).glyph(), '^');
        assert_eq!(
            Cell::Deposit(DepositKind::Uranium, DepositOrigin::Natural).color(),
            kind_color(DepositKind::Uranium)
        );
    }
}
