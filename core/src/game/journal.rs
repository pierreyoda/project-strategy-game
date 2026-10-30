//! What changed, and since when.
//!
//! The client is handed a *delta* rather than a fresh snapshot every turn, and a delta needs to know
//! what the turn touched. Comparing two full worlds would cost a pass over the whole map every turn,
//! for a handful of tiles; so the simulation records what it touches **as it touches it**, through
//! the `WorldMut` gate in `state`.
//!
//! The journal keeps the last few turns' worth of records. A client that asks for a delta from
//! further back than that is told to take a full snapshot instead - which is also what happens when
//! so much changed that a delta would cost more than one.

use std::collections::{BTreeSet, VecDeque};

use crate::simulation::ids::{BuildingId, EntityRef, NationId, SettlementId, UnitId};

/// Monotonic version of the world, bumped by every committed batch of mutations.
///
/// The client holds the version of what it last rendered and asks for everything since.
pub type StateVersion = u64;

/// How many committed batches are kept. Beyond that, a client gets a full snapshot.
///
/// **NB** Batches, not turns: a turn commits one batch, and every order the player gives commits
/// another, since it applies immediately. Sized so that a burst of orders cannot push a client that
/// renders every turn out of the window.
pub const JOURNAL_CAPACITY: usize = 64;

/// Everything one committed batch of mutations touched.
#[derive(Clone, Debug, Default)]
pub struct TurnChanges {
    /// Version this batch produced. Zero while the batch is still being filled.
    version: StateVersion,
    /// Dirty tiles, as row-major indices into `RectangularHexMapShape::coords`.
    tiles: BTreeSet<u32>,
    nations: BTreeSet<NationId>,
    settlements: BTreeSet<SettlementId>,
    buildings: BTreeSet<BuildingId>,
    units: BTreeSet<UnitId>,
    removed: BTreeSet<EntityRef>,
}

impl TurnChanges {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(&self) -> StateVersion {
        self.version
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.is_empty()
            && self.nations.is_empty()
            && self.settlements.is_empty()
            && self.buildings.is_empty()
            && self.units.is_empty()
            && self.removed.is_empty()
    }

    pub fn tiles(&self) -> impl Iterator<Item = u32> + '_ {
        self.tiles.iter().copied()
    }

    pub fn tiles_count(&self) -> usize {
        self.tiles.len()
    }

    pub fn nations(&self) -> impl Iterator<Item = NationId> + '_ {
        self.nations.iter().copied()
    }

    pub fn settlements(&self) -> impl Iterator<Item = SettlementId> + '_ {
        self.settlements.iter().copied()
    }

    pub fn buildings(&self) -> impl Iterator<Item = BuildingId> + '_ {
        self.buildings.iter().copied()
    }

    pub fn units(&self) -> impl Iterator<Item = UnitId> + '_ {
        self.units.iter().copied()
    }

    pub fn removed(&self) -> impl Iterator<Item = EntityRef> + '_ {
        self.removed.iter().copied()
    }

    pub fn mark_tile(&mut self, index: u32) {
        self.tiles.insert(index);
    }

    pub fn mark_nation(&mut self, id: NationId) {
        self.nations.insert(id);
    }

    pub fn mark_settlement(&mut self, id: SettlementId) {
        self.settlements.insert(id);
    }

    pub fn mark_building(&mut self, id: BuildingId) {
        self.buildings.insert(id);
    }

    pub fn mark_unit(&mut self, id: UnitId) {
        self.units.insert(id);
    }

    pub fn mark_removed(&mut self, entity: EntityRef) {
        self.removed.insert(entity);
    }

    /// Fold another batch into this one, the other being the more recent of the two.
    pub fn absorb(&mut self, other: &Self) {
        self.tiles.extend(other.tiles.iter().copied());
        self.nations.extend(other.nations.iter().copied());
        self.settlements.extend(other.settlements.iter().copied());
        self.buildings.extend(other.buildings.iter().copied());
        self.units.extend(other.units.iter().copied());
        self.removed.extend(other.removed.iter().copied());
        self.version = self.version.max(other.version);
    }
}

/// The last `JOURNAL_CAPACITY` committed batches, plus the one being filled.
#[derive(Debug)]
pub struct ChangeJournal {
    version: StateVersion,
    /// Version of the world the oldest kept batch starts from: a client older than this one cannot
    /// be caught up with a delta.
    oldest_base: StateVersion,
    pending: TurnChanges,
    committed: VecDeque<TurnChanges>,
    capacity: usize,
}

impl ChangeJournal {
    pub fn new() -> Self {
        Self::with_capacity(JOURNAL_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 0, "the journal must keep at least one batch");
        Self {
            version: 0,
            oldest_base: 0,
            pending: TurnChanges::new(),
            committed: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Version of the last committed batch.
    pub fn version(&self) -> StateVersion {
        self.version
    }

    /// The batch being filled, which `WorldMut` writes into.
    pub fn pending_mut(&mut self) -> &mut TurnChanges {
        &mut self.pending
    }

    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Seal the batch being filled, bumping the version.
    ///
    /// A batch that touched nothing is not committed: a version the client would have to re-fetch
    /// for nothing is worse than no version at all.
    pub fn commit(&mut self) -> StateVersion {
        if self.pending.is_empty() {
            return self.version;
        }
        self.version += 1;
        let mut sealed = std::mem::take(&mut self.pending);
        sealed.version = self.version;
        if self.committed.len() == self.capacity
            && let Some(dropped) = self.committed.pop_front()
        {
            self.oldest_base = dropped.version;
        }
        self.committed.push_back(sealed);
        self.version
    }

    /// Everything that changed after `since`, or `None` when that version is too old to be caught
    /// up from - in which case the caller must build a full snapshot instead.
    pub fn changes_since(&self, since: StateVersion) -> Option<TurnChanges> {
        if since > self.version || since < self.oldest_base {
            return None;
        }
        let mut changes = TurnChanges::new();
        changes.version = self.version;
        for batch in self.committed.iter().filter(|batch| batch.version > since) {
            changes.absorb(batch);
        }
        changes.version = self.version;
        Some(changes)
    }
}

impl Default for ChangeJournal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::simulation::ids::{EntityIndex, UnitId};

    use super::ChangeJournal;

    #[test]
    fn test_journal_commits_only_non_empty_batches() {
        let mut journal = ChangeJournal::new();
        assert_eq!(journal.version(), 0);
        assert_eq!(journal.commit(), 0);

        journal.pending_mut().mark_tile(12);
        assert_eq!(journal.commit(), 1);
        assert_eq!(journal.commit(), 1);
    }

    #[test]
    fn test_journal_accumulates_changes_since_a_version() {
        let mut journal = ChangeJournal::new();
        journal.pending_mut().mark_tile(1);
        let first = journal.commit();
        journal.pending_mut().mark_tile(2);
        journal.pending_mut().mark_unit(UnitId::from_index(7));
        let second = journal.commit();

        let all = journal.changes_since(0).expect("not too old");
        assert_eq!(all.tiles().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(all.version(), second);

        let latest = journal.changes_since(first).expect("not too old");
        assert_eq!(latest.tiles().collect::<Vec<_>>(), vec![2]);
        assert_eq!(latest.units().count(), 1);

        assert!(
            journal
                .changes_since(second)
                .expect("up to date")
                .is_empty()
        );
        assert!(journal.changes_since(second + 1).is_none());
    }

    #[test]
    fn test_journal_forgets_beyond_its_capacity() {
        let mut journal = ChangeJournal::with_capacity(2);
        for tile in 0..3 {
            journal.pending_mut().mark_tile(tile);
            journal.commit();
        }
        // the batch that produced version 1 has been dropped
        assert!(journal.changes_since(0).is_none());
        assert_eq!(
            journal
                .changes_since(1)
                .expect("still kept")
                .tiles()
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}
