//! A scenario: what a new game starts with.
//!
//! Not *where*: the nations generator sites the capitals and the borders, and the scenario answers,
//! for each slot it placed, who lives there and what they start with. The world is written by the
//! caller, never by the script - through `WorldMut`, like everything else.

use std::{collections::BTreeSet, rc::Rc};

use rhai::{AST, Array, CallFnOptions, Dynamic, EvalAltResult, INT, Map, Scope};

use project_x_core::simulation::{
    buildings::BuildingTemplate, ids::SimulationID, military::UnitTemplate, nations::Nation,
    resources::ResourceDataStore,
};

use crate::{
    api::{Definitions, Fields, ScenarioHandle, UnitDefinition},
    engine::{Hook, NATION, Runtime, STARTING_UNITS, ScriptError},
};

/// What one nation starts as, as the scenario decided it for its slot.
#[derive(Debug)]
pub struct NationSetup {
    /// Identity, name, stockpile and income.
    pub nation: Nation,
    /// What its capital is called.
    pub capital: String,
    /// The templates of the units it starts with, in the order they are to be placed.
    pub starting_units: Vec<SimulationID>,
}

/// A scenario script, its top level run and its definitions collected.
pub struct Scenario {
    runtime: Rc<Runtime>,
    script: String,
    ast: AST,
    definitions: Definitions,
}

impl Scenario {
    /// Run the scenario's top level, which defines the templates.
    pub(crate) fn load(
        runtime: Rc<Runtime>,
        script: String,
        ast: AST,
    ) -> Result<Self, ScriptError> {
        let handle = ScenarioHandle::default();
        let mut scope = Scope::new();
        // a variable rather than a constant: Rhai refuses a method that takes `&mut` on a constant
        scope.push("scenario", handle.clone());
        let result = runtime.engine.run_ast_with_scope(&mut scope, &ast);
        // printed before a game exists, with no turn log to put it in
        runtime.take_printed();
        result.map_err(|error| runtime.describe(&script, &error))?;
        Ok(Self {
            definitions: handle.take(),
            runtime,
            script,
            ast,
        })
    }

    pub fn unit_templates(&self) -> impl Iterator<Item = UnitTemplate> + '_ {
        self.definitions.units.iter().map(UnitDefinition::template)
    }

    pub fn building_templates(&self) -> impl Iterator<Item = BuildingTemplate> + '_ {
        self.definitions
            .buildings
            .iter()
            .map(|building| building.template())
    }

    /// Who lives in `slot` - counted from 0, the local player's first - and what they start with.
    pub fn nation(&self, slot: usize) -> Result<NationSetup, ScriptError> {
        let nation = self.call(&NATION, slot, "a map")?;
        let roster = self.call(&STARTING_UNITS, slot, "an array of template IDs")?;
        self.setup(slot, &nation, roster)
            .map_err(|error| self.runtime.describe(&self.script, &error))
    }

    fn setup(
        &self,
        slot: usize,
        nation: &Map,
        roster: Array,
    ) -> Result<NationSetup, Box<EvalAltResult>> {
        let what = format!("slot {slot}'s nation");
        let fields = Fields::new(
            &what,
            nation,
            &["definition", "name", "capital", "stockpile", "income"],
        )?;
        let name = fields.text("name")?;
        let capital = fields
            .optional_text("capital")?
            .unwrap_or_else(|| format!("{name} capital"));
        let mut setup = Nation::new(
            SimulationID::new_abstract_id(&fields.text("definition")?),
            name,
        );
        let (stockpile, income) = (
            fields.quantities("stockpile")?,
            fields.quantities("income")?,
        );
        let resources = setup.resources_mut();
        // an income needs a stockpile to flow into, if only an empty one
        for resource in stockpile
            .keys()
            .chain(income.keys())
            .collect::<BTreeSet<_>>()
        {
            resources.replenish(*resource, stockpile.get(resource).copied().unwrap_or(0));
            if let Some(&income) = income.get(resource) {
                resources.set_income(*resource, income);
            }
        }

        let starting_units = roster
            .into_iter()
            .map(|unit| {
                let id = unit.clone().into_string().map_err(|_| {
                    format!("slot {slot}'s starting units must be template IDs, got {unit}")
                })?;
                if !self.definitions.units.iter().any(|known| known.id == id) {
                    return Err(format!(
                        "slot {slot} starts with `{id}`, which is no unit template the scenario defines"
                    ));
                }
                Ok(SimulationID::new_abstract_id(&id))
            })
            .collect::<Result<_, String>>()?;

        Ok(NationSetup {
            nation: setup,
            capital,
            starting_units,
        })
    }

    /// Call a hook with a slot, and check it returned a `T`, described as `expected`.
    ///
    /// The top level is not run again: the definitions are in.
    fn call<T: Clone + 'static>(
        &self,
        hook: &Hook,
        slot: usize,
        expected: &str,
    ) -> Result<T, ScriptError> {
        let result = self.runtime.engine.call_fn_with_options::<Dynamic>(
            CallFnOptions::new().eval_ast(false),
            &mut Scope::new(),
            &self.ast,
            hook.name,
            (slot as INT,),
        );
        // like at load: there is no turn log yet
        self.runtime.take_printed();
        let returned = result.map_err(|error| self.runtime.describe(&self.script, &error))?;
        let type_name = returned.type_name();
        returned
            .try_cast::<T>()
            .ok_or_else(|| ScriptError::Runtime {
                script: self.script.clone(),
                message: format!(
                    "`fn {}` must return {expected}, not {type_name}",
                    hook.signature
                ),
            })
    }
}
