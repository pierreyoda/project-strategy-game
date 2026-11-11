//! A scenario: what a new game starts with.
//!
//! Not *where*: the nations generator sites the capitals and the borders, and the scenario answers,
//! for each slot it placed, who lives there and what they start with. The world is written by the
//! caller, never by the script - through `WorldMut`, like everything else.

use std::{collections::BTreeSet, fmt, rc::Rc};

use rhai::{AST, Array, CallFnOptions, Dynamic, EvalAltResult, INT, Map, Scope};

use project_x_core::simulation::{
    buildings::BuildingTemplate,
    districts::{DistrictRules, DistrictSite, DistrictTerms},
    ids::SimulationID,
    military::UnitTemplate,
    nations::Nation,
    resources::ResourceDataStore,
    settlements::SettlementStage,
};

use crate::{
    api::{Definitions, Fields, ScenarioHandle, UnitDefinition},
    engine::{DISTRICT, Hook, NATION, Runtime, STARTING_UNITS, ScriptError, declares},
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
    /// Shared with the rules it hands the game, which call its hooks long after the setup.
    ast: Rc<AST>,
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
            ast: Rc::new(ast),
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

    /// What its districts cost, through its `fn district(settlement)`: none if it declares none,
    /// and then the game has no districts.
    pub fn district_rules(&self) -> Option<ScriptedDistrictRules> {
        declares(&self.ast, &DISTRICT).then(|| ScriptedDistrictRules {
            runtime: Rc::clone(&self.runtime),
            script: self.script.clone(),
            ast: Rc::clone(&self.ast),
        })
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

/// What a district costs, as a scenario's `fn district(settlement)` says: the `DistrictRules` a
/// game is set up with.
///
/// The hook is handed `#{ districts, stage, capital, population }` - `stage` is what the
/// settlement's people amount to, "outpost" to "city"; `capital` whether its nation rules from it -
/// and returns `#{ cost, turns }`, or `throw`s why the settlement cannot have one, which is the
/// reason the order is refused with.
pub struct ScriptedDistrictRules {
    runtime: Rc<Runtime>,
    script: String,
    ast: Rc<AST>,
}

impl fmt::Debug for ScriptedDistrictRules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // the engine and the script's AST have nothing worth printing
        f.debug_struct("ScriptedDistrictRules")
            .field("script", &self.script)
            .finish_non_exhaustive()
    }
}

impl DistrictRules for ScriptedDistrictRules {
    fn terms(&self, site: &DistrictSite) -> Result<DistrictTerms, String> {
        let mut settlement = Map::new();
        settlement.insert("districts".into(), INT::from(site.districts).into());
        settlement.insert("stage".into(), stage_name(site.stage).into());
        settlement.insert("capital".into(), site.capital.into());
        settlement.insert("population".into(), INT::from(site.population).into());
        let result = self.runtime.engine.call_fn_with_options::<Dynamic>(
            CallFnOptions::new().eval_ast(false),
            &mut Scope::new(),
            &self.ast,
            DISTRICT.name,
            (settlement,),
        );
        // a rule has no turn log of its own to print to
        self.runtime.take_printed();
        let returned = result.map_err(|error| self.refusal(&error))?;
        self.read(returned)
    }
}

impl ScriptedDistrictRules {
    /// Why the hook stopped: the reason it threw, when it refused on purpose; what went wrong, when
    /// it failed.
    fn refusal(&self, error: &EvalAltResult) -> String {
        thrown(error).unwrap_or_else(|| self.runtime.describe(&self.script, error).to_string())
    }

    /// The terms the hook returned, checked, so that `core`'s asserts never fire.
    fn read(&self, returned: Dynamic) -> Result<DistrictTerms, String> {
        let failed = |message: String| {
            ScriptError::Runtime {
                script: self.script.clone(),
                message,
            }
            .to_string()
        };
        let type_name = returned.type_name();
        let terms = returned.try_cast::<Map>().ok_or_else(|| {
            failed(format!(
                "`fn {}` must return a map, not {type_name}",
                DISTRICT.signature
            ))
        })?;
        let checked = || -> Result<DistrictTerms, Box<EvalAltResult>> {
            let fields = Fields::new("a district's terms", &terms, &["cost", "turns"])?;
            Ok(DistrictTerms {
                cost: fields.quantities("cost")?,
                turns: fields.number("turns")?,
            })
        };
        checked().map_err(|error| failed(thrown(&error).unwrap_or_else(|| error.to_string())))
    }
}

/// What an error was thrown with - by the script's `throw`, or by a check of ours - through
/// however many of its functions it went up; `None` for an error the engine raised.
fn thrown(error: &EvalAltResult) -> Option<String> {
    let mut cause = error;
    while let EvalAltResult::ErrorInFunctionCall(_, _, inner, _) = cause {
        cause = inner;
    }
    match cause {
        EvalAltResult::ErrorRuntime(reason, _) => Some(reason.to_string()),
        _ => None,
    }
}

/// A settlement's stage by the name the district hook knows it by.
fn stage_name(stage: SettlementStage) -> &'static str {
    match stage {
        SettlementStage::Outpost => "outpost",
        SettlementStage::Colony => "colony",
        SettlementStage::Town => "town",
        SettlementStage::City => "city",
        SettlementStage::Capital => "capital",
    }
}
