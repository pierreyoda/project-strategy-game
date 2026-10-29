//! A leader is an individual entity in the people part of the game simulation.
//!
//! The term "leader" was chosen to better distinguish from the "population" group-level entity,
//! but it concerns anyone which is vital (or just interesting) to model at an individual scale.
//!
//! For many reasons (including performance, but also gameplay-related concerns like UI), the simulated leader "pool"
//! will tend to limit itself to echelons in the society where the leader acts for a "big" group of aspects ot their State.
//!
//! For instance, mayors of significant cities will be important to be represented, but not their entire cabinet.

use crate::simulation::{
    economy::MaintenanceCosts,
    ids::{SimulationID, WithSimulationID},
};

#[derive(Debug)]
pub enum IndividualName {
    /// Firstname/Surname.
    ///
    /// Note that even for humans in our real world, this does not take all cases into account,
    /// but this is where the needed cut in complexity happens: every leader has
    /// a firstname and surname in the game.
    HumanLike(String, String),
}

/// Basic information identifying a Leader or Ancestor.
///
/// All information contained here is publicly accessible in the game's world.
///
/// It carries no ID: a leader is a runtime instance, so it is identified by the `LeaderId` of the
/// store it lives in, and an ancestor only ever exists inside the `Lineage` that owns it.
#[derive(Debug)]
pub struct IndividualIDCard {
    name: IndividualName,
}

impl IndividualIDCard {
    pub fn name(&self) -> &IndividualName {
        &self.name
    }
}

/// An ancestor of a Leader
///
/// Leader present at game start will become an Ancestor if it dies (this may be rare, depending on turns scale).
///
/// An Ancestor is not necessarily dead, but does not have an active impact on the simulation.
/// For instance, your capital's mayors' parents may be alive but their impact, even on their own son/daughter,
/// will not be explicitly modelled (would be too costly performance-wise to do that for every leaders).
/// Still in that case, the parent(s)' influence on that mayor will already be represented in the Leader's properties
/// through the education they imparted in the mayor's childhood.
///
/// An Ancestor will be represented by much less "data" compared to a Leader,
/// which is needed for performance reasons first and foremost (including in-memory or on-disk storage size).
#[derive(Debug)]
pub struct Ancestor {
    id_card: IndividualIDCard,
    /// See `Leader.traits`.
    traits: Vec<Trait>,
}

impl Ancestor {
    pub fn id_card(&self) -> &IndividualIDCard {
        &self.id_card
    }

    pub fn traits(&self) -> &[Trait] {
        &self.traits
    }
}

/// The lineage of a Leader.
///
/// There is no optimization of storage or runtime access speed, so a leader lineage
/// should not be too deep - for now.
#[derive(Debug)]
pub struct Lineage {
    /// Stored as (parent1, parent2).
    ///
    /// In practically all cases, this corresponds to (mother, father)
    /// in the biological sense.
    ///
    /// However, room is left there to model peculiar situations like a leader being adopted.
    /// For instance, In the Roman Empire of old, their first emperor was Augustus, who was adopted by Ceasar in his will.
    /// Being able to represent a similar situation may be interesting someday, like if you have
    /// an Emperor at the head of the Human civilization.
    ancestors: Vec<(Ancestor, Ancestor)>,
}

impl Lineage {
    pub fn ancestors(&self) -> &[(Ancestor, Ancestor)] {
        &self.ancestors
    }
}

#[derive(Debug)]
pub struct Leader {
    id_card: IndividualIDCard,
    lineage: Option<Lineage>,
    /// See `Leader.traits`
    traits: Vec<Trait>,
    upkeep: MaintenanceCosts,
}

impl Leader {
    pub fn id_card(&self) -> &IndividualIDCard {
        &self.id_card
    }

    pub fn lineage(&self) -> Option<&Lineage> {
        self.lineage.as_ref()
    }

    pub fn traits(&self) -> &[Trait] {
        &self.traits
    }

    pub fn upkeep(&self) -> &MaintenanceCosts {
        &self.upkeep
    }
}

/// A Trait is a singular characteristic of a Leader.
///
/// Script-defined, so identified by a `SimulationID`: unlike a leader, a trait is a definition.
#[derive(Debug)]
pub enum Trait {
    /// A trait pertaining to a singular physical characteristic of an individual.
    /// It must be interesting to keep track of in the simulation.
    ///
    /// Examples: wounded in battle, cannot have children.
    Physical(TraitPhysical),
    /// A trait pertaining to a singular personality aspect of an individual.
    ///
    /// Examples: honest, corrupt.
    TraitPersonality(TraitPersonality),
    /// A trait pertaining to a singular, publicly-known ability for an individual.
    ///
    /// Examples: excellent logistician.
    Ability(TraitAbility),
    /// A trait pertaining to lifestyle styles, by choice or not.
    ///
    /// Examples: athletic.
    Lifestyle(TraitLifestyle),
    /// A custom trait, not belonging to any other category.
    ///
    /// Made for scripting purposes.
    Custom(TraitCustom),
}

impl WithSimulationID for Trait {
    fn id(&self) -> &SimulationID {
        match self {
            Self::Physical(t) => &t.id,
            Self::TraitPersonality(t) => &t.id,
            Self::Ability(t) => &t.id,
            Self::Lifestyle(t) => &t.id,
            Self::Custom(t) => &t.id,
        }
    }
}

#[derive(Debug)]
pub struct TraitPhysical {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
}

#[derive(Debug)]
pub struct TraitPersonality {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
}

#[derive(Debug)]
pub struct TraitAbility {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
}

#[derive(Debug)]
pub struct TraitLifestyle {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
}

#[derive(Debug)]
pub struct TraitCustom {
    /// Must be a `SimulationID::Abstract`.
    id: SimulationID,
}
