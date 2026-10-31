//! The `NationController` an AI script plays for a nation through.

use std::{fmt, rc::Rc};

use rhai::{AST, Dynamic, Scope};

use project_x_core::{
    game::{
        ai::{NationController, NationView},
        commands::GameCommand,
        rng::GameRng,
        turn::StepBudget,
    },
    simulation::ids::NationId,
};

use crate::{
    api::{Orders, ScriptRng, ScriptView},
    engine::{PLAN, Runtime},
};

/// Plays for a nation with an AI script's `plan`: the whole of an AI nation for now, the part of a
/// human one handed over to automation or a disloyal general later.
///
/// The script is handed what its nation knows and a place to put its orders; it never holds the
/// world. Its orders go through `commands::apply` like a player's, and what it printed - or the
/// error it stopped on - is drained into the turn log under its nation.
pub struct ScriptedController {
    runtime: Rc<Runtime>,
    script: String,
    ast: Rc<AST>,
    nation: NationId,
    rng: ScriptRng,
    log: Vec<String>,
}

impl ScriptedController {
    pub(crate) fn new(
        runtime: Rc<Runtime>,
        script: String,
        ast: Rc<AST>,
        nation: NationId,
        rng: GameRng,
    ) -> Self {
        Self {
            runtime,
            script,
            ast,
            nation,
            rng: ScriptRng::new(rng),
            log: Vec::new(),
        }
    }
}

impl fmt::Debug for ScriptedController {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // the engine and the script's AST have nothing worth printing
        f.debug_struct("ScriptedController")
            .field("script", &self.script)
            .field("nation", &self.nation)
            .finish_non_exhaustive()
    }
}

impl NationController for ScriptedController {
    fn name(&self) -> &str {
        &self.script
    }

    /// Run the script's `plan` to completion.
    ///
    /// The step's budget is deliberately not a limit on it: a script cannot be suspended halfway
    /// and resumed on the next step, and an AI cut off mid-thought orders nonsense. A script that
    /// fails still has the orders it gave before failing applied.
    fn plan(&mut self, view: &NationView<'_>, _budget: StepBudget) -> Vec<GameCommand> {
        let orders = Orders::default();
        let result = self.runtime.engine.call_fn::<Dynamic>(
            &mut Scope::new(),
            &self.ast,
            PLAN.name,
            (ScriptView::new(view), orders.clone(), self.rng.clone()),
        );
        // what it printed was printed before whatever stopped it
        self.log.extend(self.runtime.take_printed());
        if let Err(error) = result {
            self.log
                .push(self.runtime.describe(&self.script, &error).to_string());
        }
        orders.take()
    }

    fn take_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.log)
    }
}
