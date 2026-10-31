//! A scenario script: the templates it defines, and the nation it answers for each slot.

use project_x_core::simulation::{
    ids::SimulationID,
    resources::{Resource, ResourceDataStore},
};
use project_x_scripting::{ScriptEngine, ScriptError, ScriptKind, ScriptLimits, ScriptSource};

const DEFAULT_SCENARIO: &str = include_str!("../../scripts/scenarios/default.rhai");
const SCRIPT_NAME: &str = "under_test";

/// Hooks that answer something valid, for scripts that test anything but them.
const HOOKS: &str = r#"
    fn nation(slot) { #{ definition: `nation_${slot}`, name: `Nation ${slot}` } }
    fn starting_units(slot) { [] }
"#;

fn scenario(source: &str) -> ScriptSource {
    ScriptSource {
        name: SCRIPT_NAME.to_string(),
        kind: ScriptKind::Scenario,
        source: source.to_string(),
    }
}

fn load(source: &str) -> Result<ScriptEngine, ScriptError> {
    ScriptEngine::load(&[scenario(source)])
}

/// The message of a runtime error, for asserting on what it says.
fn failure<T>(result: Result<T, ScriptError>) -> String {
    match result {
        Err(ScriptError::Runtime { script, message }) => {
            assert_eq!(script, SCRIPT_NAME);
            message
        }
        Err(other) => panic!("expected a runtime error, got {other:?}"),
        Ok(_) => panic!("expected a runtime error, it went through"),
    }
}

#[test]
fn test_the_default_scenario_defines_the_divisions_and_the_buildings() {
    let engine = load(DEFAULT_SCENARIO).expect("the default scenario loads");
    let scenario = engine.scenario().expect("a scenario was given");

    let units: Vec<String> = scenario
        .unit_templates()
        .map(|template| {
            format!(
                "{} {} {}",
                template.r#type(),
                template.movement(),
                template.sight()
            )
        })
        .collect();
    assert_eq!(
        units,
        [
            "infantry_division 2 2",
            "motorized_division 4 3",
            "light_tank_division 3 2"
        ]
    );
    let buildings: Vec<String> = scenario
        .building_templates()
        .map(|template| format!("{template:?}"))
        .collect();
    assert_eq!(buildings.len(), 2);
    assert!(buildings[0].contains("\"mine\"") && buildings[1].contains("\"solar_plant\""));
}

#[test]
fn test_the_default_scenario_answers_for_every_slot() {
    let engine = load(DEFAULT_SCENARIO).expect("the default scenario loads");
    let scenario = engine.scenario().expect("a scenario was given");

    let setup = scenario.nation(2).expect("slot 2");
    assert_eq!(setup.nation.name(), "Nation 3");
    assert_eq!(
        setup.nation.definition(),
        &SimulationID::new_abstract_id("nation_2")
    );
    assert_eq!(setup.capital, "Nation 3 capital");
    let resources = setup.nation.resources();
    assert_eq!(resources.quantity_of(Resource::Credits), 250);
    assert_eq!(resources[&Resource::Food].income(), Some(6));
    assert_eq!(
        resources.quantity_of(Resource::Uranium),
        0,
        "never mentioned"
    );
    assert_eq!(
        setup.starting_units,
        [
            "infantry_division",
            "motorized_division",
            "light_tank_division"
        ]
        .map(SimulationID::new_abstract_id)
    );
}

#[test]
fn test_what_a_scenario_leaves_out_takes_a_default() {
    let engine = load(
        r#"
        scenario.define_unit_template(#{ id: "scout", movement: 2, sight: 1 });
        fn nation(slot) { #{ definition: "lone", name: "Lone", income: #{ food: 3 } } }
        fn starting_units(slot) { ["scout"] }
        "#,
    )
    .expect("loads");
    let scenario = engine.scenario().unwrap();

    let scout = scenario.unit_templates().next().unwrap();
    assert_eq!(scout.r#type(), "scout", "the type defaults to the ID");
    let setup = scenario.nation(0).unwrap();
    assert_eq!(setup.capital, "Lone capital");
    // an income with no stockpile still flows, into an empty one
    assert_eq!(setup.nation.resources()[&Resource::Food].quantity(), 0);
    assert_eq!(setup.nation.resources()[&Resource::Food].income(), Some(3));
}

#[test]
fn test_a_scenario_must_declare_both_hooks() {
    let only_nation = r#"fn nation(slot) { #{} }"#;
    assert_eq!(
        load(only_nation).err(),
        Some(ScriptError::MissingHook {
            script: SCRIPT_NAME.to_string(),
            hook: "starting_units(slot)",
        })
    );
    assert_eq!(
        load("fn starting_units(slot) { [] }").err(),
        Some(ScriptError::MissingHook {
            script: SCRIPT_NAME.to_string(),
            hook: "nation(slot)",
        })
    );
}

#[test]
fn test_a_template_defined_wrong_is_refused_at_load() {
    for (definition, expected) in [
        (
            r#"scenario.define_unit_template(#{ id: "scout", movement: 2, sight: 1, speed: 9 })"#,
            "a unit template has no `speed`",
        ),
        (
            r#"scenario.define_unit_template(#{ movement: 2, sight: 1 })"#,
            "a unit template needs `id`",
        ),
        (
            r#"scenario.define_unit_template(#{ id: "scout", movement: 300, sight: 1 })"#,
            "a unit template's `movement` is out of range: 300",
        ),
        (
            r#"scenario.define_building_template(#{ id: "farm", production: #{ grain: 2 } })"#,
            "names `grain`, which is no resource",
        ),
        (
            r#"scenario.define_building_template(#{ id: "farm", cost: #{ credits: -1 } })"#,
            "`cost.credits` must be from 0 to 65535, got -1",
        ),
        (
            r#"scenario.define_building_template(#{ id: "farm" });
               scenario.define_building_template(#{ id: "farm" })"#,
            "building template `farm` is defined twice",
        ),
    ] {
        let message = failure(load(&format!("{definition};\n{HOOKS}")));
        assert!(message.contains(expected), "{definition}: {message}");
    }
}

#[test]
fn test_a_nation_answered_wrong_is_refused_for_its_slot() {
    for (hooks, expected) in [
        (
            r#"fn nation(slot) { "Atlantis" } fn starting_units(slot) { [] }"#,
            "`fn nation(slot)` must return a map, not string",
        ),
        (
            r#"fn nation(slot) { #{ definition: "a", name: "A", flag: "red" } }
               fn starting_units(slot) { [] }"#,
            "slot 1's nation has no `flag`",
        ),
        (
            r#"fn nation(slot) { #{ definition: "a", name: "A" } }
               fn starting_units(slot) { ["dragon"] }"#,
            "slot 1 starts with `dragon`, which is no unit template the scenario defines",
        ),
        (
            r#"fn nation(slot) { #{ definition: "a", name: "A" } }
               fn starting_units(slot) { "scout" }"#,
            "`fn starting_units(slot)` must return an array of template IDs, not string",
        ),
        (
            r#"fn nation(slot) { if slot > 0 { throw "only one nation fits" } #{} }
               fn starting_units(slot) { [] }"#,
            "only one nation fits",
        ),
    ] {
        let engine = load(hooks).expect("the hooks are only called for a slot");
        let message = failure(engine.scenario().unwrap().nation(1));
        assert!(message.contains(expected), "{hooks}: {message}");
    }
}

#[test]
fn test_a_scenario_that_never_ends_is_stopped_at_load() {
    let limits = ScriptLimits {
        runaway_operations: 10_000,
    };
    let result = ScriptEngine::load_with(&[scenario(&format!("loop {{}}\n{HOOKS}"))], limits);
    assert_eq!(
        result.err(),
        Some(ScriptError::Runaway {
            script: SCRIPT_NAME.to_string(),
            operations: 10_000,
        })
    );
}

#[test]
fn test_a_game_takes_one_scenario_beside_its_ai_script() {
    let ai = ScriptSource {
        name: "ai".to_string(),
        kind: ScriptKind::Ai,
        source: "fn plan(view, orders, rng) {}".to_string(),
    };
    let engine = ScriptEngine::load(&[scenario(DEFAULT_SCENARIO), ai]).expect("one of each");
    assert!(engine.scenario().is_some());

    assert_eq!(
        ScriptEngine::load(&[scenario(DEFAULT_SCENARIO), scenario(DEFAULT_SCENARIO)]).err(),
        Some(ScriptError::Duplicate {
            script: SCRIPT_NAME.to_string(),
            kind: ScriptKind::Scenario,
        })
    );
}
