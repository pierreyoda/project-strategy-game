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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use crate::simulation::ids::SimulationID;

    use super::{SimulationPropertyStorage, SimulationPropertyValue};

    fn property_id(name: &str) -> SimulationID {
        SimulationID::new_property_id(name.to_string())
    }

    fn build_mock_property_storage() -> SimulationPropertyStorage {
        SimulationPropertyStorage::new()
            .register_new(
                property_id("small_integer"),
                SimulationPropertyValue::SmallInteger(-12),
            )
            .register_new(
                property_id("integer"),
                SimulationPropertyValue::Integer(100_000),
            )
            .register_new(property_id("float"), SimulationPropertyValue::Float(0.75))
            .register_new(
                property_id("text"),
                SimulationPropertyValue::Text("iron mine".to_string()),
            )
    }

    /// Checks that the properties of `build_mock_property_storage` other than the given one are untouched.
    fn assert_other_mock_properties_untouched(
        properties: &SimulationPropertyStorage,
        except: &str,
    ) {
        if except != "small_integer" {
            assert!(matches!(
                properties.get_from_id(&property_id("small_integer")),
                Some(SimulationPropertyValue::SmallInteger(-12))
            ));
        }
        if except != "integer" {
            assert!(matches!(
                properties.get_from_id(&property_id("integer")),
                Some(SimulationPropertyValue::Integer(100_000))
            ));
        }
        if except != "float" {
            assert!(matches!(
                properties.get_from_id(&property_id("float")),
                Some(SimulationPropertyValue::Float(value)) if *value == 0.75
            ));
        }
        if except != "text" {
            assert!(matches!(
                properties.get_from_id(&property_id("text")),
                Some(SimulationPropertyValue::Text(text)) if text == "iron mine"
            ));
        }
    }

    #[test]
    fn test_simulation_property_storage_new_is_empty() {
        let mut properties = SimulationPropertyStorage::new();
        let id = property_id("integer");

        assert!(properties.get_from_id(&id).is_none());
        assert!(properties.get_from_id_mut(&id).is_none());
        assert!(!properties.mutate(&id, |_| {}));
    }

    #[test]
    fn test_simulation_property_storage_register_all_value_types() {
        let properties = build_mock_property_storage();
        assert_other_mock_properties_untouched(&properties, "");
    }

    #[test]
    fn test_simulation_property_storage_register_extreme_values() {
        let properties = SimulationPropertyStorage::new()
            .register_new(
                property_id("min_small_integer"),
                SimulationPropertyValue::SmallInteger(i16::MIN),
            )
            .register_new(
                property_id("max_small_integer"),
                SimulationPropertyValue::SmallInteger(i16::MAX),
            )
            .register_new(
                property_id("min_integer"),
                SimulationPropertyValue::Integer(i32::MIN),
            )
            .register_new(
                property_id("max_integer"),
                SimulationPropertyValue::Integer(i32::MAX),
            )
            .register_new(property_id("nan"), SimulationPropertyValue::Float(f64::NAN))
            .register_new(
                property_id("infinity"),
                SimulationPropertyValue::Float(f64::INFINITY),
            )
            .register_new(
                property_id("empty_text"),
                SimulationPropertyValue::Text(String::new()),
            )
            .register_new(
                property_id("unicode_text"),
                SimulationPropertyValue::Text("Ünshrouded ⛰".to_string()),
            );

        assert!(matches!(
            properties.get_from_id(&property_id("min_small_integer")),
            Some(SimulationPropertyValue::SmallInteger(i16::MIN))
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("max_small_integer")),
            Some(SimulationPropertyValue::SmallInteger(i16::MAX))
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("min_integer")),
            Some(SimulationPropertyValue::Integer(i32::MIN))
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("max_integer")),
            Some(SimulationPropertyValue::Integer(i32::MAX))
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("nan")),
            Some(SimulationPropertyValue::Float(value)) if value.is_nan()
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("infinity")),
            Some(SimulationPropertyValue::Float(value)) if *value == f64::INFINITY
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("empty_text")),
            Some(SimulationPropertyValue::Text(text)) if text.is_empty()
        ));
        assert!(matches!(
            properties.get_from_id(&property_id("unicode_text")),
            Some(SimulationPropertyValue::Text(text)) if text == "Ünshrouded ⛰"
        ));
    }

    #[test]
    fn test_simulation_property_storage_register_existing_id_replaces_value() {
        let properties = SimulationPropertyStorage::new()
            .register_new(property_id("rate"), SimulationPropertyValue::Integer(1))
            .register_new(property_id("rate"), SimulationPropertyValue::Float(2.5));

        assert!(matches!(
            properties.get_from_id(&property_id("rate")),
            Some(SimulationPropertyValue::Float(value)) if *value == 2.5
        ));
    }

    #[test]
    #[should_panic(expected = "SimulationID::Property")]
    fn test_simulation_property_storage_register_rejects_abstract_id() {
        SimulationPropertyStorage::new().register_new(
            SimulationID::new_abstract_id("integer"),
            SimulationPropertyValue::Integer(1),
        );
    }

    #[test]
    fn test_simulation_property_storage_get_unknown_property() {
        let mut properties = build_mock_property_storage();
        let unknown_ids = [
            property_id("unknown"),
            property_id("Integer"),
            property_id("integer "),
            property_id(""),
        ];
        for id in unknown_ids {
            assert!(properties.get_from_id(&id).is_none(), "{id:?}");
            assert!(properties.get_from_id_mut(&id).is_none(), "{id:?}");
        }
    }

    #[test]
    fn test_simulation_property_storage_does_not_mix_up_ids_of_other_kinds() {
        // `SimulationID` hashes an abstract ID like a property ID of the same name
        let mut properties = build_mock_property_storage();
        let abstract_id = SimulationID::new_abstract_id("integer");
        let calls = Cell::new(0);

        assert!(properties.get_from_id(&abstract_id).is_none());
        assert!(properties.get_from_id_mut(&abstract_id).is_none());
        assert!(!properties.mutate(&abstract_id, |_| calls.set(calls.get() + 1)));
        assert_eq!(calls.get(), 0);
        assert_other_mock_properties_untouched(&properties, "");
    }

    #[test]
    fn test_simulation_property_storage_get_mut() {
        let mut properties = build_mock_property_storage();

        if let Some(SimulationPropertyValue::SmallInteger(value)) =
            properties.get_from_id_mut(&property_id("small_integer"))
        {
            *value *= 2;
        } else {
            panic!("missing small integer property");
        }
        assert!(matches!(
            properties.get_from_id(&property_id("small_integer")),
            Some(SimulationPropertyValue::SmallInteger(-24))
        ));
        assert_other_mock_properties_untouched(&properties, "small_integer");

        *properties
            .get_from_id_mut(&property_id("small_integer"))
            .expect("missing small integer property") =
            SimulationPropertyValue::Text("converted".to_string());
        assert!(matches!(
            properties.get_from_id(&property_id("small_integer")),
            Some(SimulationPropertyValue::Text(text)) if text == "converted"
        ));
        assert_other_mock_properties_untouched(&properties, "small_integer");
    }

    #[test]
    fn test_simulation_property_storage_mutate() {
        let mut properties = build_mock_property_storage();

        assert!(properties.mutate(&property_id("integer"), |value| {
            if let SimulationPropertyValue::Integer(integer) = value {
                *integer += 1;
            }
        }));
        assert!(matches!(
            properties.get_from_id(&property_id("integer")),
            Some(SimulationPropertyValue::Integer(100_001))
        ));
        assert_other_mock_properties_untouched(&properties, "integer");

        assert!(properties.mutate(&property_id("text"), |value| {
            if let SimulationPropertyValue::Text(text) = value {
                text.push_str(" (depleted)");
            }
        }));
        assert!(matches!(
            properties.get_from_id(&property_id("text")),
            Some(SimulationPropertyValue::Text(text)) if text == "iron mine (depleted)"
        ));
    }

    #[test]
    fn test_simulation_property_storage_mutate_can_change_value_type() {
        let mut properties = build_mock_property_storage();

        assert!(properties.mutate(&property_id("float"), |value| {
            *value = SimulationPropertyValue::Integer(3);
        }));
        assert!(matches!(
            properties.get_from_id(&property_id("float")),
            Some(SimulationPropertyValue::Integer(3))
        ));
        assert_other_mock_properties_untouched(&properties, "float");
    }

    #[test]
    fn test_simulation_property_storage_mutate_unknown_property() {
        let mut properties = build_mock_property_storage();
        let calls = Cell::new(0);

        assert!(!properties.mutate(&property_id("unknown"), |_| calls.set(calls.get() + 1)));
        assert_eq!(calls.get(), 0);
        assert_other_mock_properties_untouched(&properties, "");
    }

    #[test]
    fn test_simulation_property_storage_mutate_calls_mutation_once() {
        let mut properties = build_mock_property_storage();
        let calls = Cell::new(0);

        assert!(properties.mutate(&property_id("integer"), |_| calls.set(calls.get() + 1)));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn test_simulation_property_storage_many_properties() {
        let name = |index: i32| format!("property_{index}");
        let properties = (0..1000).fold(SimulationPropertyStorage::new(), |properties, index| {
            properties.register_new(
                property_id(&name(index)),
                SimulationPropertyValue::Integer(index),
            )
        });

        for index in 0..1000 {
            assert!(
                matches!(
                    properties.get_from_id(&property_id(&name(index))),
                    Some(SimulationPropertyValue::Integer(value)) if *value == index
                ),
                "property {index}"
            );
        }
        assert!(properties.get_from_id(&property_id(&name(1000))).is_none());
    }
}
