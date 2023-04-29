use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

/// ID of an entity, like a leader or a population group.
///
/// Not "physically" present on the game map, but can be attached to entities that are (eg. leader of a building or military unit).
///
/// Since there will be many entities in the game simulation, an efficient scalar is chosen
/// (not a String, and &'static str is not possible due to scripting constraints).
///
/// **NB** The entity ID scalar being an `u32`, there can be at most "u32::max = 4 294 967 295" entities in a given game,
/// which should be more than enough. Still, it can be changed to an "u64" if needed some day, and just by changing the type here.
///
/// See: https://doc.rust-lang.org/std/u32/constant.MAX.html
pub type SimulationEntityID = u32;

/// ID of an on-map entity, which is under the hood stored in the played `HexMap`.
///
/// Can be buildings, or military units.
///
/// See `SimulationEntityID` for the choice of "u32" here.
pub type SimulationMapEntityID = u32;

/// ID of an "abstract" simulation aspect, i.e. which is not physically represented in the game but still tangible for the simulation.
/// It can be a leader's trait, for instance, or an atomic simulation system like an economy concept.
///
/// More often than not, the IDs of this category will come from scripts, which is why a String is chosen here, for
/// convenience - it's easier to give unique names without a relatively complex machinery, with probably poor DevX.
/// It's a trade-off made for easier game script development.
///
/// Cannot be `&static str` due to scripting constraints, so use the heap-allocated String is used.
pub type SimulationAbstractID = String;

/// ID of a utility component attached to a simulation component.
///
/// See  `SimulationPropertyStorage` for instance.
///
/// Uses a String for better debugging. "Prefixed" by the simulation component the container is attached to.
/// TODO: find a better, more generic name than "container" (adapt to needs raising in the POC)
pub type SimulationAttachedContainerID = String;

/// ID of a property attached to a simulation object, entity or concept.
///
/// It's a String for the same reason as `SimulationAbstractID`, since properties are for scripts to define and use.
pub type SimulationPropertyID = String;

/// Global (simulation-wide) identification for a simulation component (entity/object/unit/concept/...) in the simulation.
///
/// **Must not change** for any simulation component, whatever it does.
///
/// Regardless of the underlying type of ID, all IDs must be unique when it comes to their common representation, the hash.
///
/// Similarly, the output of the `Display` trait implementation is unique across the whole simulation.
///
/// ## Usage
///
/// - Avoid cloning any `SimulationID`, except when necessary.
/// - Use the static methods provided to instantiate a new `SimulationID`
/// (would for instance facilitate adding a unicity asserter in the simulation, by just moving the used methods around)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SimulationID {
    EntityID(SimulationEntityID),
    MapEntityID(SimulationMapEntityID),
    Abstract(SimulationAbstractID),
    AttachedContainer(SimulationAttachedContainerID),
    Property(SimulationPropertyID),
}

impl SimulationID {
    pub fn new_entity_id(unique_id: SimulationEntityID) -> Self {
        Self::EntityID(unique_id)
    }

    pub fn new_map_entity_id(unique_id: SimulationMapEntityID) -> Self {
        Self::MapEntityID(unique_id)
    }

    pub fn new_abstract_id(unique_id: &str) -> Self {
        Self::Abstract(unique_id.to_string())
    }

    /// The given `key` must be unique among all components attached to the parent.
    pub fn new_attached_container_id(parent_id: &SimulationID, key: &str) -> Self {
        Self::AttachedContainer(parent_id.generate_prefixed_attached_unique_string_id(key))
    }

    pub fn new_property_id(unique_id: String) -> Self {
        Self::Property(unique_id)
    }

    pub fn generate_prefixed_attached_unique_string_id(&self, key: &str) -> String {
        todo!()
    }

    pub fn as_unique_string(&self) -> String {
        format!("{}", self)
    }
}

impl Hash for SimulationID {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::EntityID(entity_id) => entity_id.hash(state),
            Self::MapEntityID(map_entity_id) => map_entity_id.hash(state),
            Self::Abstract(abstract_id) => abstract_id.hash(state),
            Self::AttachedContainer(attached_container_id) => attached_container_id.hash(state),
            Self::Property(property_id) => property_id.hash(state),
        }
    }
}

/// Convenience Trait for any simulation aspect that has a `SimulationID`.
pub trait WithSimulationID {
    fn id(&self) -> &SimulationID;
}

impl Display for SimulationID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
