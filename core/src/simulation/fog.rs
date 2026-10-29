//! Fog of war: what each nation knows of the map, one record per nation.
//!
//! The journal records the truth; this is what the projection filters it through, so that a client
//! is never handed a tile its viewing nation cannot see. Keeping it out of the projection would mean
//! shipping an omniscient world and hiding it client-side, which leaks the moment anyone opens the
//! devtools.

use crate::simulation::ids::NationId;

/// What a nation currently knows of a tile.
///
/// The discriminants are the wire values: the client reads them straight out of a `Uint8Array`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TileVisibility {
    /// Never seen: the client must draw nothing at all.
    Unexplored = 0,
    /// Seen at some point, not watched right now: terrain is known, what stands on it may be stale.
    Fogged = 1,
    /// Watched by a unit or a in range of settlement or other owned infrastructure right now.
    Visible = 2,
}

/// A compact set of tile indices, in the row-major order of `RectangularHexMapShape::coords`.
#[derive(Clone, Debug)]
pub struct TileBitSet {
    words: Vec<u64>,
    len: usize,
}

impl TileBitSet {
    pub fn with_len(len: usize) -> Self {
        Self {
            words: vec![0; len.div_ceil(64)],
            len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn contains(&self, index: usize) -> bool {
        index < self.len && (self.words[index / 64] >> (index % 64)) & 1 == 1
    }

    /// Returns whether the index was not already set.
    pub fn insert(&mut self, index: usize) -> bool {
        if index >= self.len || self.contains(index) {
            return false;
        }
        self.words[index / 64] |= 1 << (index % 64);
        true
    }

    pub fn clear(&mut self) {
        self.words.fill(0);
    }

    /// Iterate over the set indices, in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.len).filter(|&index| self.contains(index))
    }

    /// Iterate over the indices set in exactly one of the two sets, in ascending order.
    ///
    /// Word-wise: a sighting pass compares two full-map sets every turn, and walking them tile by
    /// tile would be a pass over the whole map per nation.
    pub fn symmetric_difference<'a>(&'a self, other: &'a Self) -> impl Iterator<Item = usize> + 'a {
        self.words
            .iter()
            .zip(other.words.iter())
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .flat_map(|(word_index, (a, b))| {
                let mut differing = a ^ b;
                std::iter::from_fn(move || {
                    if differing == 0 {
                        return None;
                    }
                    let bit = differing.trailing_zeros() as usize;
                    differing &= differing - 1;
                    Some(word_index * 64 + bit)
                })
            })
            .filter(|&index| index < self.len)
    }

    fn swap_into(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.words, &mut other.words);
    }
}

/// What one nation knows of the map.
#[derive(Clone, Debug)]
pub struct NationFog {
    explored: TileBitSet,
    visible: TileBitSet,
    /// What was visible before the sighting pass in progress, so that it can report what changed.
    previously_visible: TileBitSet,
}

impl NationFog {
    pub fn with_len(len: usize) -> Self {
        Self {
            explored: TileBitSet::with_len(len),
            visible: TileBitSet::with_len(len),
            previously_visible: TileBitSet::with_len(len),
        }
    }

    pub fn visibility(&self, index: usize) -> TileVisibility {
        if self.visible.contains(index) {
            TileVisibility::Visible
        } else if self.explored.contains(index) {
            TileVisibility::Fogged
        } else {
            TileVisibility::Unexplored
        }
    }

    pub fn explored(&self) -> &TileBitSet {
        &self.explored
    }

    pub fn visible(&self) -> &TileBitSet {
        &self.visible
    }

    /// Start a new sighting pass: everything watched last turn falls back to fogged, and is kept
    /// aside so that `end_recompute` can tell what actually moved.
    pub fn begin_recompute(&mut self) {
        self.visible.swap_into(&mut self.previously_visible);
        self.visible.clear();
    }

    /// Mark a tile as watched right now, and as explored for good.
    pub fn reveal(&mut self, index: usize) {
        self.explored.insert(index);
        self.visible.insert(index);
    }

    /// Close the sighting pass, reporting the tiles whose visibility changed: those, and only
    /// those, have to be resent to the client.
    ///
    /// Re-dirtying every watched tile every turn would be far simpler but would make the delta
    /// roughly the size of every nation's field of view, turn after turn.
    pub fn end_recompute(&self) -> impl Iterator<Item = usize> + '_ {
        self.visible.symmetric_difference(&self.previously_visible)
    }
}

/// Fog records of every nation, kept apart from the nations themselves: it is a view of the world,
/// not a property of the nation, and it is rebuilt wholesale every turn.
#[derive(Debug)]
pub struct FogOfWar {
    tiles_count: usize,
    nations: Vec<NationFog>,
}

impl FogOfWar {
    pub fn new(tiles_count: usize) -> Self {
        Self {
            tiles_count,
            nations: Vec::new(),
        }
    }

    /// Make sure every nation up to `nation` has a record.
    pub fn register(&mut self, nation: NationId) {
        use crate::simulation::ids::EntityIndex;
        while self.nations.len() <= nation.index() {
            self.nations.push(NationFog::with_len(self.tiles_count));
        }
    }

    pub fn of(&self, nation: NationId) -> Option<&NationFog> {
        use crate::simulation::ids::EntityIndex;
        self.nations.get(nation.index())
    }

    pub fn of_mut(&mut self, nation: NationId) -> Option<&mut NationFog> {
        use crate::simulation::ids::EntityIndex;
        self.nations.get_mut(nation.index())
    }
}

#[cfg(test)]
mod tests {
    use super::{NationFog, TileBitSet, TileVisibility};

    #[test]
    fn test_tile_bitset_across_word_boundaries() {
        let mut set = TileBitSet::with_len(130);
        for index in [0, 63, 64, 129] {
            assert!(set.insert(index), "{index}");
            assert!(!set.insert(index), "{index}");
            assert!(set.contains(index), "{index}");
        }
        assert!(!set.contains(1));
        assert!(!set.insert(130));
        assert!(!set.contains(130));
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![0, 63, 64, 129]);

        set.clear();
        assert_eq!(set.iter().count(), 0);
    }

    #[test]
    fn test_tile_bitset_symmetric_difference() {
        let mut a = TileBitSet::with_len(130);
        let mut b = TileBitSet::with_len(130);
        for index in [1, 64, 70, 129] {
            a.insert(index);
        }
        for index in [1, 65, 70] {
            b.insert(index);
        }
        assert_eq!(
            a.symmetric_difference(&b).collect::<Vec<_>>(),
            vec![64, 65, 129]
        );
        assert_eq!(a.symmetric_difference(&a).count(), 0);
    }

    #[test]
    fn test_nation_fog_falls_back_to_fogged() {
        let mut fog = NationFog::with_len(8);
        assert_eq!(fog.visibility(3), TileVisibility::Unexplored);

        fog.begin_recompute();
        fog.reveal(3);
        assert_eq!(fog.visibility(3), TileVisibility::Visible);
        assert_eq!(fog.end_recompute().collect::<Vec<_>>(), vec![3]);

        // watched again from the same place: nothing moved, so nothing to resend
        fog.begin_recompute();
        fog.reveal(3);
        assert_eq!(fog.end_recompute().count(), 0);

        // no longer watched: explored, and a change the client has to be told about
        fog.begin_recompute();
        assert_eq!(fog.visibility(3), TileVisibility::Fogged);
        assert_eq!(fog.end_recompute().collect::<Vec<_>>(), vec![3]);
    }
}
