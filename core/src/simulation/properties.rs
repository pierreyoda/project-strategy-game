///! Dynamic properties in a game simulation component (object/entity/...).
///!
///! For instance, a property can be the resource extraction rate for a mining facility building.
///!
///! Aimed at being used from scripts, so Rust types **must not** leak to the "API surface" of properties.
use std::collections::HashMap;

use super::ids::SimulationID;

/// The different types of values that ban be stored for a `SimulationProperty`.
#[derive(Debug)]
pub enum SimulationPropertyValue {
    /// A small integer
    SmallInteger(i16),
    /// An integer.
    Integer(i32),
    /// A floating point number property.
    ///
    /// Uses `f64` for precision.
    Float(f64),
    /// A text property.
    ///
    /// Stored as a String, so heap-allocated.
    Text(String),
}

/// Storage for simulation properties.
///
/// Drop-in for simulation components that need it.
/// TODO: unit tests
#[derive(Debug)]
pub struct SimulationPropertyStorage {
    /// Storage data structure, with `O(1)` access.
    properties: HashMap<SimulationID, SimulationPropertyValue>,
}

impl SimulationPropertyStorage {
    /// Create e new properties storage, to be attached to a parent simulation component.
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }

    /// Register a new property.
    pub fn register_new(
        mut self,
        id: SimulationID,
        initial_value: SimulationPropertyValue,
    ) -> Self {
        assert!(matches!(id, SimulationID::Property(_)));
        // TODO: check ID type, return script-aware error if needed
        self.properties.insert(id, initial_value);
        self
    }

    pub fn get_from_id(&self, id: &SimulationID) -> Option<&SimulationPropertyValue> {
        self.properties.get(id)
    }

    pub fn get_from_id_mut(&mut self, id: &SimulationID) -> Option<&mut SimulationPropertyValue> {
        self.properties.get_mut(id)
    }

    pub fn mutate<F>(&mut self, id: &SimulationID, mutation: F) -> bool
    where
        F: Fn(&mut SimulationPropertyValue),
    {
        if let Some(property_value) = self.properties.get_mut(id) {
            mutation(property_value);
            true
        } else {
            false
        }
    }
}
