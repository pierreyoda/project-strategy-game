//! What a scenario defines: the templates every nation builds from.
//!
//! A scenario defines them at its top level, through the `scenario` handle it is run with. That is
//! the one place the handle exists: an AI script is never given one, so it cannot define anything,
//! and nor can a scenario's own functions, which run later, once the definitions are in.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use rhai::{Engine, INT, Map};

use project_x_core::simulation::{
    buildings::BuildingTemplate,
    ids::SimulationID,
    military::UnitTemplate,
    resources::{Resource, ResourceQuantity},
};

use crate::api::common::Fields;

use super::ScriptResult;

/// A unit template, as a scenario defines it.
#[derive(Clone, Debug)]
pub struct UnitDefinition {
    pub id: String,
    kind: String,
    movement: u8,
    sight: u8,
}

impl UnitDefinition {
    pub fn template(&self) -> UnitTemplate {
        UnitTemplate::new(
            SimulationID::new_abstract_id(&self.id),
            self.kind.clone(),
            self.movement,
            self.sight,
        )
    }
}

/// A building template, as a scenario defines it.
#[derive(Clone, Debug)]
pub struct BuildingDefinition {
    id: String,
    kind: String,
    cost: BTreeMap<Resource, ResourceQuantity>,
    maintenance: BTreeMap<Resource, ResourceQuantity>,
    production: BTreeMap<Resource, ResourceQuantity>,
}

impl BuildingDefinition {
    pub fn template(&self) -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id(&self.id),
            self.kind.clone(),
            self.cost.clone(),
            self.maintenance.clone(),
            self.production.clone(),
        )
    }
}

/// Everything a scenario defined, in the order it did.
#[derive(Debug, Default)]
pub struct Definitions {
    pub units: Vec<UnitDefinition>,
    pub buildings: Vec<BuildingDefinition>,
}

/// The handle a scenario's top level defines through. Shared, like `Orders`.
#[derive(Clone, Debug, Default)]
pub struct ScenarioHandle(Rc<RefCell<Definitions>>);

impl ScenarioHandle {
    /// Everything defined so far. Empties the handle.
    pub fn take(&self) -> Definitions {
        self.0.take()
    }
}

pub fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<ScenarioHandle>("Scenario")
        .register_fn(
            "define_unit_template",
            |scenario: &mut ScenarioHandle, definition: Map| -> ScriptResult<()> {
                let fields = Fields::new(
                    "a unit template",
                    &definition,
                    &["id", "type", "movement", "sight"],
                )?;
                let id = fields.text("id")?;
                let unit = UnitDefinition {
                    kind: fields.optional_text("type")?.unwrap_or_else(|| id.clone()),
                    movement: fields.number("movement")?,
                    sight: fields.number("sight")?,
                    id,
                };
                let mut definitions = scenario.0.borrow_mut();
                if definitions.units.iter().any(|known| known.id == unit.id) {
                    return Err(format!("unit template `{}` is defined twice", unit.id).into());
                }
                definitions.units.push(unit);
                Ok(())
            },
        )
        .register_fn(
            "define_building_template",
            |scenario: &mut ScenarioHandle, definition: Map| -> ScriptResult<()> {
                let fields = Fields::new(
                    "a building template",
                    &definition,
                    &["id", "type", "cost", "maintenance", "production"],
                )?;
                let id = fields.text("id")?;
                let building = BuildingDefinition {
                    kind: fields.optional_text("type")?.unwrap_or_else(|| id.clone()),
                    cost: fields.quantities("cost")?,
                    maintenance: fields.quantities("maintenance")?,
                    production: fields.quantities("production")?,
                    id,
                };
                let mut definitions = scenario.0.borrow_mut();
                if definitions
                    .buildings
                    .iter()
                    .any(|known| known.id == building.id)
                {
                    return Err(
                        format!("building template `{}` is defined twice", building.id).into(),
                    );
                }
                definitions.buildings.push(building);
                Ok(())
            },
        );
}
