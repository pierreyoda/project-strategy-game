//! Storage of runtime simulation instances, indexed by the dense IDs of the `ids` module.
//!
//! Entities live here and nowhere else: tiles, nations and commands all refer to them by ID. That is
//! what makes the world mutable turn after turn, and what makes a projection cheap to build, since an
//! ID is already an index into the output buffers.

use std::fmt::Debug;

use super::ids::EntityIndex;

/// Dense store of entities of one kind.
///
/// Slots are never shifted: an ID handed out stays valid until the entity is removed, and removal
/// leaves a hole that the next insertion reuses.
///
/// **NB** IDs are not generational yet: an ID kept across a removal would silently address the
/// entity that took the freed slot. Commands validate their IDs against the store on every call, so
/// nothing in the loop holds one across a turn - but anything that starts to will need
/// generations here first.
#[derive(Debug)]
pub struct EntityStore<I: EntityIndex, T> {
    slots: Vec<Option<T>>,
    free: Vec<I>,
    len: usize,
}

impl<I: EntityIndex, T> EntityStore<I, T> {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(capacity),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Number of entities currently stored.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Highest slot ever handed out, plus one: the length the projection buffers must cover.
    pub fn slots_count(&self) -> usize {
        self.slots.len()
    }

    pub fn insert(&mut self, entity: T) -> I {
        if let Some(id) = self.free.pop() {
            self.slots[id.index()] = Some(entity);
            self.len += 1;
            id
        } else {
            let id = I::from_index(self.slots.len());
            self.slots.push(Some(entity));
            self.len += 1;
            id
        }
    }

    pub fn get(&self, id: I) -> Option<&T> {
        self.slots.get(id.index())?.as_ref()
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut T> {
        self.slots.get_mut(id.index())?.as_mut()
    }

    pub fn contains(&self, id: I) -> bool {
        self.get(id).is_some()
    }

    pub fn remove(&mut self, id: I) -> Option<T> {
        let slot = self.slots.get_mut(id.index())?;
        let entity = slot.take()?;
        self.free.push(id);
        self.len -= 1;
        Some(entity)
    }

    /// Iterate over the entities in ID order, which is the order resolution and projection rely on.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| slot.as_ref().map(|entity| (I::from_index(index), entity)))
    }

    /// Iterate over the IDs in order. Collected by callers that need to mutate while iterating.
    pub fn ids(&self) -> impl Iterator<Item = I> + '_ {
        self.iter().map(|(id, _)| id)
    }
}

impl<I: EntityIndex, T> Default for EntityStore<I, T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::simulation::ids::{EntityIndex, UnitId};

    use super::EntityStore;

    #[test]
    fn test_entity_store_insert_get_remove() {
        let mut store: EntityStore<UnitId, &str> = EntityStore::new();
        let a = store.insert("a");
        let b = store.insert("b");
        assert_eq!((a.index(), b.index()), (0, 1));
        assert_eq!(store.len(), 2);
        assert_eq!(store.get(a), Some(&"a"));

        assert_eq!(store.remove(a), Some("a"));
        assert_eq!(store.get(a), None);
        assert!(!store.contains(a));
        assert_eq!(store.len(), 1);
        assert_eq!(store.remove(a), None);

        // the freed slot is reused, and the store never shifts the entities that stayed
        let c = store.insert("c");
        assert_eq!(c, a);
        assert_eq!(store.get(b), Some(&"b"));
        assert_eq!(store.slots_count(), 2);
    }

    #[test]
    fn test_entity_store_iterates_in_id_order() {
        let mut store: EntityStore<UnitId, u8> = EntityStore::new();
        for value in 0..5 {
            store.insert(value);
        }
        store.remove(UnitId::from_index(2));
        let entries: Vec<(usize, u8)> = store.iter().map(|(id, &v)| (id.index(), v)).collect();
        assert_eq!(entries, vec![(0, 0), (1, 1), (3, 3), (4, 4)]);
    }
}
