//! How far a unit gets: the entry cost of the ground, the one-unit-per-tile rule, and the tiles a
//! unit can end its turn on.
//!
//! `commands::move_unit` validates an order with `step_cost` and `reachable` previews it with the
//! same function, so the map can never promise a step the order would refuse.

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashSet},
};

use crate::{
    game::state::{TileIndex, World},
    hex_map::{coordinates::CubeCoords, terrain::TerrainKind},
    simulation::{
        fog::TileVisibility,
        ids::{NationId, UnitId},
    },
};

/// The entry cost of ground no land unit crosses.
pub const IMPASSABLE: u8 = u8::MAX;

/// Points a unit spends to enter a tile of this kind.
///
/// A table for now; script-defined per template later, through `UnitTemplate::attributes`.
pub fn entry_cost(kind: TerrainKind) -> u8 {
    match kind {
        TerrainKind::Flat => 1,
        TerrainKind::Hills | TerrainKind::Ice => 2,
        TerrainKind::Mountains | TerrainKind::RoughIce => 3,
        TerrainKind::DeepWater | TerrainKind::ShallowWater => IMPASSABLE,
    }
}

/// Why one step cannot be taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRefusal {
    /// Beyond the poles: the map wraps east-west only.
    OffMap,
    NotAdjacent,
    Impassable,
    /// Another unit stands there: one unit per tile.
    Occupied,
    NoMovementLeft,
}

impl StepRefusal {
    pub fn reason(self) -> &'static str {
        match self {
            Self::OffMap => "the path leaves the map",
            Self::NotAdjacent => "the path skips a tile",
            Self::Impassable => "the ground cannot be crossed",
            Self::Occupied => "the tile is already occupied",
            Self::NoMovementLeft => "not enough movement points left",
        }
    }
}

/// The tiles some unit stands on, leaving out `except` - the unit about to move, whose own tile is
/// no obstacle to itself.
pub fn occupied_tiles(world: &World, except: Option<UnitId>) -> HashSet<TileIndex> {
    world
        .units()
        .iter()
        .filter(|(id, _)| Some(*id) != except)
        .filter_map(|(_, unit)| world.tile_index(unit.position()))
        .collect()
}

/// The points one step from `from` onto `to` costs a unit with `left` points, or why it cannot be
/// taken. The wrapped coordinates of the tile entered come with the cost.
///
/// A step needs at least one point and costs the ground's entry cost, capped by what is left: a
/// unit can always finish on the hill it reached rather than stand one point short of it.
pub fn step_cost(
    world: &World,
    occupied: &HashSet<TileIndex>,
    from: CubeCoords,
    to: CubeCoords,
    left: u8,
) -> Result<(u8, CubeCoords), StepRefusal> {
    if left == 0 {
        return Err(StepRefusal::NoMovementLeft);
    }
    let shape = world.shape();
    let to = shape.wrap(to).ok_or(StepRefusal::OffMap)?;
    let index = world.tile_index(to).ok_or(StepRefusal::OffMap)?;
    if !shape
        .neighbors(from)
        .any(|neighbor| world.tile_index(neighbor) == Some(index))
    {
        return Err(StepRefusal::NotAdjacent);
    }
    let kind = world
        .tile(to)
        .map(|tile| tile.natural().terrain())
        .ok_or(StepRefusal::OffMap)?;
    let cost = entry_cost(kind);
    if cost == IMPASSABLE {
        return Err(StepRefusal::Impassable);
    }
    if occupied.contains(&index) {
        return Err(StepRefusal::Occupied);
    }
    Ok((cost.min(left), to))
}

/// The tiles a unit can end this turn on, with the cheapest way to each.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reachable {
    /// Row-major indices, ascending, the unit's own tile included.
    pub tiles: Vec<TileIndex>,
    /// Points spent to reach each one, parallel to `tiles`.
    pub costs: Vec<u8>,
    /// The tile before each one on its cheapest path, parallel to `tiles`; the origin names itself.
    pub parents: Vec<TileIndex>,
}

impl Reachable {
    /// The path from the unit's tile to `tile`, destination last and the origin left out: what
    /// `GameCommand::MoveUnit` takes. `None` for a tile that is not reachable.
    pub fn path_to(&self, tile: TileIndex) -> Option<Vec<TileIndex>> {
        let position = |tile: TileIndex| self.tiles.binary_search(&tile).ok();
        let mut at = position(tile)?;
        let mut path = Vec::new();
        while self.parents[at] != self.tiles[at] {
            path.push(self.tiles[at]);
            at = position(self.parents[at])?;
        }
        path.reverse();
        Some(path)
    }
}

/// Where `viewer`'s `unit` can get to this turn, by the rules `move_unit` applies.
///
/// Built **for the viewer**, like a projection: a tile the viewer has never explored counts as
/// impassable, so the answer never says anything about ground the viewer has not seen. An order
/// into the unknown can still be given, and is judged against the truth. `None` for a unit the
/// viewer does not own, which is also the only kind the client asks about.
pub fn reachable(world: &World, viewer: NationId, unit: UnitId) -> Option<Reachable> {
    let moved = world.units().get(unit)?;
    if moved.owner() != viewer {
        return None;
    }
    let origin_index = world.tile_index(moved.position())?;
    let budget = moved.movement_left();
    let known = |index: TileIndex| {
        world
            .fog_of(viewer)
            .is_some_and(|fog| fog.visibility(index as usize) != TileVisibility::Unexplored)
    };
    let occupied = occupied_tiles(world, Some(unit));

    // Dijkstra over a budget this small is a handful of relaxations: no need for a decrease-key
    let mut best: Vec<Option<(u8, TileIndex)>> = vec![None; world.tiles_count()];
    best[origin_index as usize] = Some((0, origin_index));
    let mut frontier = BinaryHeap::new();
    frontier.push(Reverse((0u8, origin_index)));
    while let Some(Reverse((spent, index))) = frontier.pop() {
        if best[index as usize].is_some_and(|(cost, _)| cost < spent) {
            continue;
        }
        let Some(coords) = world.tile_coords(index) else {
            continue;
        };
        for neighbor in world.shape().neighbors(coords) {
            let Ok((cost, entered)) = step_cost(world, &occupied, coords, neighbor, budget - spent)
            else {
                continue;
            };
            let Some(entered_index) = world.tile_index(entered) else {
                continue;
            };
            if !known(entered_index) {
                continue;
            }
            let total = spent + cost;
            if best[entered_index as usize].is_none_or(|(known_cost, _)| total < known_cost) {
                best[entered_index as usize] = Some((total, index));
                frontier.push(Reverse((total, entered_index)));
            }
        }
    }

    let mut result = Reachable::default();
    for (index, entry) in best.iter().enumerate() {
        if let Some((cost, parent)) = entry {
            result.tiles.push(index as TileIndex);
            result.costs.push(*cost);
            result.parents.push(*parent);
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::{IMPASSABLE, entry_cost};
    use crate::hex_map::terrain::TerrainKind;

    #[test]
    fn test_water_is_the_only_impassable_ground() {
        for kind in [
            TerrainKind::Flat,
            TerrainKind::Hills,
            TerrainKind::Mountains,
            TerrainKind::Ice,
            TerrainKind::RoughIce,
        ] {
            assert!(entry_cost(kind) < IMPASSABLE, "{kind:?}");
        }
        assert_eq!(entry_cost(TerrainKind::DeepWater), IMPASSABLE);
        assert_eq!(entry_cost(TerrainKind::ShallowWater), IMPASSABLE);
        assert!(entry_cost(TerrainKind::Hills) > entry_cost(TerrainKind::Flat));
    }
}
