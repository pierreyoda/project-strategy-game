//! The Rhai engine a game's scripts run in, its limits, and loading them.

use std::{cell::RefCell, error::Error, fmt, rc::Rc};

use rhai::{AST, Engine, EvalAltResult, module_resolvers::DummyModuleResolver};

use project_x_core::{
    game::rng::{GameRng, GameSeed, RngPurpose},
    simulation::ids::NationId,
};

use crate::{api, controller::ScriptedController, scenario::Scenario};

/// A function a script of some kind has to declare, for the host to call.
pub struct Hook {
    pub name: &'static str,
    params: usize,
    /// How it is written, for the errors about it.
    pub signature: &'static str,
}

/// The hook an AI script plans with.
pub const PLAN: Hook = Hook {
    name: "plan",
    params: 3,
    signature: "plan(view, orders, rng)",
};
/// Who lives in a slot the nations generator placed.
pub const NATION: Hook = Hook {
    name: "nation",
    params: 1,
    signature: "nation(slot)",
};
/// What a slot starts with.
pub const STARTING_UNITS: Hook = Hook {
    name: "starting_units",
    params: 1,
    signature: "starting_units(slot)",
};

/// What a script is for, which decides the hooks it has to declare.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ScriptKind {
    /// Plays for a nation, through `fn plan(view, orders, rng)`: a whole AI nation, or - once a
    /// human nation can hand some of it over - the part of one in its charge, for automation or a
    /// disloyal general for instance.
    Ai,
    /// Says what a new game starts with: the templates, through the `scenario` handle its top level
    /// is run with, then who lives in each slot and what they start with, through
    /// `fn nation(slot)` and `fn starting_units(slot)`.
    Scenario,
}

impl fmt::Display for ScriptKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ai => f.write_str("AI"),
            Self::Scenario => f.write_str("scenario"),
        }
    }
}

/// A script, as a game is started with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptSource {
    /// What the script is called in errors and logs.
    pub name: String,
    pub kind: ScriptKind,
    pub source: String,
}

/// How far a script may go before it is stopped.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ScriptLimits {
    /// Operations past which a script is taken to be looping forever.
    ///
    /// **Not** a budget: a plan runs to completion however long it takes, since an AI interrupted
    /// mid-thought orders nonsense. This is where "long" is taken to mean "never ending" - far above
    /// anything a plan reaches, and the one knob a deliberately heavy script may raise. Without it,
    /// a `loop {}` would hang a worker that is never terminated, because its state *is* the game.
    pub runaway_operations: u64,
}

impl ScriptLimits {
    pub const DEFAULT_RUNAWAY_OPERATIONS: u64 = 50_000_000;
}

impl Default for ScriptLimits {
    fn default() -> Self {
        Self {
            runaway_operations: Self::DEFAULT_RUNAWAY_OPERATIONS,
        }
    }
}

/// Why a script did not load, or did not finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Compile {
        script: String,
        message: String,
    },
    MissingHook {
        script: String,
        hook: &'static str,
    },
    /// A second script of a kind a game takes one of - for now. Automation, generals and nation
    /// AIs will each bring their own AI script, and the setup will then say which plays what.
    Duplicate {
        script: String,
        kind: ScriptKind,
    },
    Runtime {
        script: String,
        message: String,
    },
    /// Stopped past `ScriptLimits::runaway_operations`.
    Runaway {
        script: String,
        operations: u64,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile { script, message } => {
                write!(f, "script `{script}` does not compile: {message}")
            }
            Self::MissingHook { script, hook } => {
                write!(f, "script `{script}` does not declare `fn {hook}`")
            }
            Self::Duplicate { script, kind } => {
                write!(
                    f,
                    "script `{script}` is a second {kind} script, and a game takes one"
                )
            }
            Self::Runtime { script, message } => write!(f, "script `{script}` failed: {message}"),
            Self::Runaway { script, operations } => write!(
                f,
                "script `{script}` was stopped after {operations} operations, taken for an endless loop"
            ),
        }
    }
}

impl Error for ScriptError {}

/// The engine every script of a game runs in, and what it printed during the call in progress.
///
/// One per game, shared by every controller: calls never overlap - the worker is single-threaded -
/// so a single sink is enough, drained by whoever made the call.
pub struct Runtime {
    pub engine: Engine,
    pub limits: ScriptLimits,
    printed: Rc<RefCell<Vec<String>>>,
}

impl Runtime {
    fn new(limits: ScriptLimits) -> Self {
        let printed = Rc::new(RefCell::new(Vec::new()));
        let mut engine = Engine::new();
        engine
            .set_max_operations(limits.runaway_operations)
            .set_max_call_levels(32)
            .set_max_expr_depths(64, 32)
            .set_max_string_size(64 * 1024)
            .set_max_array_size(4096)
            .set_max_map_size(1024)
            // a script reads nothing it was not handed: no file, no module
            .set_module_resolver(DummyModuleResolver::new());
        let sink = Rc::clone(&printed);
        engine.on_print(move |text| sink.borrow_mut().push(text.to_string()));
        let sink = Rc::clone(&printed);
        engine.on_debug(move |text, _, _| sink.borrow_mut().push(text.to_string()));
        api::register(&mut engine);
        Self {
            engine,
            limits,
            printed,
        }
    }

    /// What the scripts printed since this was last called.
    pub fn take_printed(&self) -> Vec<String> {
        self.printed.take()
    }

    /// What a script stopping on `error` amounts to, worded once for every kind of script.
    pub fn describe(&self, script: &str, error: &EvalAltResult) -> ScriptError {
        // an error inside the script's own functions comes wrapped once per call it went through
        let mut cause = error;
        while let EvalAltResult::ErrorInFunctionCall(_, _, inner, _) = cause {
            cause = inner;
        }
        let script = script.to_string();
        match cause {
            EvalAltResult::ErrorTooManyOperations(_) => ScriptError::Runaway {
                script,
                operations: self.limits.runaway_operations,
            },
            _ => ScriptError::Runtime {
                script,
                message: cause.to_string(),
            },
        }
    }
}

/// A game's scripts, compiled and checked.
pub struct ScriptEngine {
    runtime: Rc<Runtime>,
    ai: Option<AiScript>,
    scenario: Option<Scenario>,
}

struct AiScript {
    name: String,
    ast: Rc<AST>,
}

impl ScriptEngine {
    pub fn load(sources: &[ScriptSource]) -> Result<Self, ScriptError> {
        Self::load_with(sources, ScriptLimits::default())
    }

    /// Compile every script, and check each one declares the hooks its kind requires.
    ///
    /// Everything that can be refused is refused here, before a game exists: a script that does
    /// not load is a bug in the scripts, not something to find out on turn 40.
    ///
    /// A scenario's top level is run here too, so a template it defines wrong is refused before a
    /// planet is generated for nothing.
    pub fn load_with(sources: &[ScriptSource], limits: ScriptLimits) -> Result<Self, ScriptError> {
        let runtime = Rc::new(Runtime::new(limits));
        let (mut ai, mut scenario) = (None, None);
        for source in sources {
            let script = || source.name.clone();
            let mut ast =
                runtime
                    .engine
                    .compile(&source.source)
                    .map_err(|error| ScriptError::Compile {
                        script: script(),
                        message: error.to_string(),
                    })?;
            ast.set_source(source.name.as_str());

            let taken = match source.kind {
                ScriptKind::Ai => ai.is_some(),
                ScriptKind::Scenario => scenario.is_some(),
            };
            if taken {
                return Err(ScriptError::Duplicate {
                    script: script(),
                    kind: source.kind,
                });
            }
            match source.kind {
                ScriptKind::Ai => {
                    require(&ast, &source.name, &PLAN)?;
                    ai = Some(AiScript {
                        name: script(),
                        ast: Rc::new(ast),
                    });
                }
                ScriptKind::Scenario => {
                    require(&ast, &source.name, &NATION)?;
                    require(&ast, &source.name, &STARTING_UNITS)?;
                    scenario = Some(Scenario::load(Rc::clone(&runtime), script(), ast)?);
                }
            }
        }
        Ok(Self {
            runtime,
            ai,
            scenario,
        })
    }

    /// The scenario, if the game was given one.
    pub fn scenario(&self) -> Option<&Scenario> {
        self.scenario.as_ref()
    }

    /// A controller playing the whole of `nation` with the AI script, if the game was given one.
    ///
    /// It draws from the nation's own random stream, the one the stub controller would have had. A
    /// nation played by several controllers - its AI and a general, say - will need one stream per
    /// controller, or adding a general would shift every draw the nation's AI makes after it.
    pub fn controller_for(&self, nation: NationId, seed: GameSeed) -> Option<ScriptedController> {
        let ai = self.ai.as_ref()?;
        Some(ScriptedController::new(
            Rc::clone(&self.runtime),
            ai.name.clone(),
            Rc::clone(&ai.ast),
            nation,
            GameRng::new(seed, RngPurpose::Ai(nation)),
        ))
    }
}

/// Refuse a script that does not declare `hook`, with its parameters.
fn require(ast: &AST, script: &str, hook: &Hook) -> Result<(), ScriptError> {
    let declared = ast
        .iter_functions()
        .any(|function| function.name == hook.name && function.params.len() == hook.params);
    if declared {
        Ok(())
    } else {
        Err(ScriptError::MissingHook {
            script: script.to_string(),
            hook: hook.signature,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ScriptEngine, ScriptError, ScriptKind, ScriptSource};

    fn ai(name: &str, source: &str) -> ScriptSource {
        ScriptSource {
            name: name.to_string(),
            kind: ScriptKind::Ai,
            source: source.to_string(),
        }
    }

    fn load(sources: &[ScriptSource]) -> Result<(), ScriptError> {
        ScriptEngine::load(sources).map(|_| ())
    }

    #[test]
    fn test_no_script_at_all_is_a_game_without_scripted_ai() {
        let engine = ScriptEngine::load(&[]).expect("nothing to refuse");
        let nation = project_x_core::simulation::ids::EntityIndex::from_index(1);
        assert!(engine.controller_for(nation, 7).is_none());
    }

    #[test]
    fn test_a_script_that_does_not_compile_is_refused_by_name() {
        let Err(ScriptError::Compile { script, message }) =
            load(&[ai("broken", "fn plan(view, orders, rng) { let }")])
        else {
            panic!("a syntax error must be refused");
        };
        assert_eq!(script, "broken");
        assert!(message.contains("line 1"), "{message}");
    }

    #[test]
    fn test_an_ai_script_must_declare_plan_with_its_three_parameters() {
        for source in ["fn think(view) {}", "fn plan(view) {}", "plan();"] {
            assert_eq!(
                load(&[ai("idle", source)]),
                Err(ScriptError::MissingHook {
                    script: "idle".to_string(),
                    hook: "plan(view, orders, rng)",
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn test_a_game_takes_one_ai_script() {
        let plan = "fn plan(view, orders, rng) {}";
        assert_eq!(
            load(&[ai("first", plan), ai("second", plan)]),
            Err(ScriptError::Duplicate {
                script: "second".to_string(),
                kind: ScriptKind::Ai,
            })
        );
    }
}
