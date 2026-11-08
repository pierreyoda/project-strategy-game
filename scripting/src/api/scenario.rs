//! What a scenario defines: the templates every nation builds from.
//!
//! A scenario defines them at its top level, through the `scenario` handle it is run with. That is
//! the one place the handle exists: an AI script is never given one, so it cannot define anything,
//! and nor can a scenario's own functions, which run later, once the definitions are in.

use std::{cell::RefCell, rc::Rc};

use rhai::{Engine, INT, Map};

use project_x_core::simulation::{
    buildings::{BuildingLevel, BuildingTemplate, MAX_BUILDING_LEVELS},
    ids::SimulationID,
    military::UnitTemplate,
    resources::Resource,
    strength::{MAX_UNIT_COMPANIES, Recovery},
};

use crate::api::common::{Fields, resource_name};

use super::ScriptResult;

/// A unit template, as a scenario defines it: checked, so that `core`'s asserts never fire.
#[derive(Clone, Debug)]
pub struct UnitDefinition {
    pub id: String,
    kind: String,
    movement: u8,
    sight: u8,
    companies: u8,
    recovery: Recovery,
}

impl UnitDefinition {
    pub fn template(&self) -> UnitTemplate {
        UnitTemplate::new(
            SimulationID::new_abstract_id(&self.id),
            self.kind.clone(),
            self.movement,
            self.sight,
            self.companies,
            self.recovery,
        )
    }

    fn read(definition: &Map) -> ScriptResult<Self> {
        let fields = Fields::new(
            "a unit template",
            definition,
            &["id", "type", "movement", "sight", "companies", "recovery"],
        )?;
        let id = fields.text("id")?;
        let companies: u8 = fields.number("companies")?;
        if !(1..=MAX_UNIT_COMPANIES).contains(&companies) {
            return Err(format!(
                "unit template `{id}` has {companies} companies; it takes from 1 to \
                 {MAX_UNIT_COMPANIES}"
            )
            .into());
        }
        // every suppressed company, unless the template holds them back
        let recovery = match fields.value("recovery") {
            None => Recovery::All,
            Some(value) if value.clone().into_string().is_ok_and(|text| text == "all") => {
                Recovery::All
            }
            Some(value) => value
                .as_int()
                .ok()
                .filter(|&count| (1..=INT::from(companies)).contains(&count))
                .map(|count| Recovery::UpTo(count as u8))
                .ok_or_else(|| {
                    format!(
                        "unit template `{id}` recovers \"all\" its suppressed companies or a \
                         number of them from 1 to {companies} a turn, not {value}"
                    )
                })?,
        };
        Ok(Self {
            kind: fields.optional_text("type")?.unwrap_or_else(|| id.clone()),
            movement: fields.number("movement")?,
            sight: fields.number("sight")?,
            companies,
            recovery,
            id,
        })
    }
}

/// A building template, as a scenario defines it: checked, so that `core`'s asserts never fire.
#[derive(Clone, Debug)]
pub struct BuildingDefinition {
    id: String,
    kind: String,
    deposit: Option<Resource>,
    levels: Vec<BuildingLevel>,
}

impl BuildingDefinition {
    pub fn template(&self) -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id(&self.id),
            self.kind.clone(),
            self.deposit,
            self.levels.clone(),
        )
    }

    fn read(definition: &Map) -> ScriptResult<Self> {
        let fields = Fields::new(
            "a building template",
            definition,
            &["id", "type", "deposit", "levels"],
        )?;
        let id = fields.text("id")?;
        let kind = fields.optional_text("type")?.unwrap_or_else(|| id.clone());
        let deposit = fields.optional_resource("deposit")?;
        let levels = fields.maps("levels")?;
        if !(1..=MAX_BUILDING_LEVELS).contains(&levels.len()) {
            return Err(format!(
                "building template `{id}` has {} levels; it takes from 1 to {MAX_BUILDING_LEVELS}",
                levels.len()
            )
            .into());
        }
        let levels = levels
            .iter()
            .enumerate()
            .map(|(index, level)| read_level(&id, &kind, deposit, index + 1, level))
            .collect::<ScriptResult<_>>()?;
        Ok(Self {
            id,
            kind,
            deposit,
            levels,
        })
    }
}

/// Level `number` of the template `id`, of type `kind`, extracting `deposit` if any.
fn read_level(
    id: &str,
    kind: &str,
    deposit: Option<Resource>,
    number: usize,
    definition: &Map,
) -> ScriptResult<BuildingLevel> {
    let what = format!("level {number} of `{id}`");
    let fields = Fields::new(
        &what,
        definition,
        &["cost", "turns", "maintenance", "production", "sprite"],
    )?;
    let turns: u16 = fields.number("turns")?;
    if turns == 0 {
        return Err(format!("{what} takes no turns; reaching a level takes at least one").into());
    }
    let production = fields.quantities("production")?;
    // an extractor that extracts nothing is a typo, not a design
    if let Some(resource) = deposit
        && !production.get(&resource).is_some_and(|&amount| amount > 0)
    {
        return Err(format!(
            "{what} produces no {}, which `{id}` extracts",
            resource_name(resource)
        )
        .into());
    }
    Ok(BuildingLevel {
        cost: fields.quantities("cost")?,
        turns,
        maintenance: fields.quantities("maintenance")?,
        production,
        // the naming the client's art follows, unless the level points at other art
        sprite: fields
            .optional_text("sprite")?
            .unwrap_or_else(|| format!("{kind}_{number}")),
    })
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
                let unit = UnitDefinition::read(&definition)?;
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
                let building = BuildingDefinition::read(&definition)?;
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
