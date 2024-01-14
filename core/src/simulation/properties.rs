///! Dynamic properties in a game simulation component (object/entity/...).
///!
///! For instance, a property can be the resource extraction rate for a mining facility building.
///!
///! Aimed at being used from scripts, so Rust types **must not** leak to the "API surface" of properties.
use std::collections::HashMap;

use super::ids::{SimulationID, WithSimulationID};

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
    /// A text number property.
    ///
    /// Stored as a String, so heap-allocated.
    Text(String),
}

#[derive(Debug)]
pub struct SimulationProperty {
    /// Must be a `SimulationID::Property`.
    id: SimulationID,
    /// Current value of he property.
    value: SimulationPropertyValue,
}

impl SimulationProperty {
    pub fn new(id: SimulationID, initial_value: SimulationPropertyValue) -> Self {
        Self {
            id,
            value: initial_value,
        }
    }
}

impl WithSimulationID for SimulationProperty {
    fn id(&self) -> &SimulationID {
        &self.id
    }
}

/// TODO:
///
/// Short-lived object used Only during gameplay, i.e. not serialized in a game save.
// TODO: modify value from script closure: convenience utility in the scripting crate wrapper
#[derive(Debug)]
pub struct SimulationPropertyHandle<'a> {
    noop_log_prefix: Option<&'a str>,
    property: Option<&'a mut SimulationProperty>,
}

impl<'a> SimulationPropertyHandle<'a> {
    fn from_property(property: &'a mut SimulationProperty) -> Self {
        Self {
            noop_log_prefix: None,
            property: Some(property),
        }
    }

    fn as_container_noop(log_prefix: &'a str) -> Self {
        Self {
            noop_log_prefix: Some(log_prefix),
            property: None,
        }
    }

    pub fn set_value() {
        todo!()
    }
}

pub trait IntoSimulationPropertyKey {}

// TODO: impl Display for SimulationProperty

/// Storage for simulation properties.
///
/// Drop-in for simulation components that need it.
/// TODO: unit tests
#[derive(Debug)]
pub struct SimulationPropertyStorage<'a> {
    /// ID of the parent simulation component.
    ///
    /// Used as some form of "prefix" for this properties storage,
    ///
    /// NB: a clone and not a reference, to avoid dealing with since a `SimulationID` does not change.
    /// This is less efficient but:
    /// - avoids dealing with any sort of Rust lifetime (even though it is a simple case here)
    /// - more importantly, this struct will be (de)serializable with no further effort
    parent_id: SimulationID,
    /// Must be `SimulationID::Abstract`.
    ///
    /// This struct has an ID so that it can generate a unique `SimulationPropertyID`
    /// for newly inserted properties with ease, with no need of the calling Rust code or script
    /// to worry about uniqueness of the stored properties IDs, on which it has no control.
    ///
    /// Again, you can think of this ID as a prefix for the generated IDs of each property stored.
    id: SimulationID,
    /// Storage data structure, with `O(1)` access.
    ///
    /// To avoid duplicating the stores properties' String-based IDs in the keys,
    /// we use an inner form of IDS base on small unsigned integers.
    /// This is transparent when using this storage, both in Rust or in scripts.
    ///
    /// To do this, we just track the order in which the properties are inserted.
    properties: HashMap<u16, SimulationProperty>,
    /// A NOOP property handle that will log warnings every time you try to interact with it.
    ///
    /// NB: this is not to be serialized.
    /// TODO: automatically when implementing save states
    noop_property_handle: SimulationPropertyHandle<'a>,
}

impl<'a> SimulationPropertyStorage<'a> {
    /// Create e new properties storage, to be attached to a parent simulation component.
    pub fn new(parent_id: &SimulationID, key: &str) -> Self {
        Self {
            parent_id: parent_id.clone(),
            id: SimulationID::new_attached_container_id(parent_id, key),
            properties: HashMap::new(),
            noop_property_handle: SimulationPropertyHandle::as_container_noop(todo!()),
        }
    }

    /// Register a new property.
    pub fn register_new(mut self, key: &str, initial_value: SimulationPropertyValue) -> Self {
        // starts at 0
        let property_index = self.properties.len();
        let property_id =
            SimulationID::new_property_id(self.id.generate_prefixed_attached_unique_string_id(key));
        let property = SimulationProperty::new(property_id, initial_value);
        // TODO: check bounds, return script-aware error if needed
        self.properties.insert(property_index as u16, property);
        self
    }

    /// Get a property from its string ID.
    ///
    /// `Into` would be nice to use here for the `id` parameter, but this would result in copying a String,
    /// so do an avoidable heap-allocation.
    ///
    /// Does not use Rust error handling, since:
    /// - we do not want to raise a game-stopping error if the property is not found,
    /// which can happen depending on the calling function handling of the `Result`
    /// - properties are defined and used in scripts, where the DSL has no control flow / syntactic
    /// way of dealing with an eventual error returned, as opposed to Rust
    pub fn get_from_id<I: Into<SimulationID>>(id: &I) -> Option<&mut SimulationPropertyHandle> {
        todo!()
    }
}
