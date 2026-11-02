//! A script's orders: the only thing it can do to the world.
//!
//! Nothing is validated here. The orders are applied, in the order they were given, through
//! `commands::apply` - like a player's - and one that breaks a rule comes back as a
//! `CommandRejected` event, the bug report it is.

use std::{cell::RefCell, rc::Rc};

use rhai::{Array, Engine, INT, ImmutableString, Map};

use project_x_core::{
    game::commands::GameCommand,
    simulation::ids::{BuildingId, SimulationID, UnitId},
};

use super::{ScriptResult, coords, entity_id};

/// The orders given during one `plan`.
///
/// A shared handle, so that the host still holds what was ordered when the script stops on an
/// error halfway through: those orders go through all the same.
#[derive(Clone, Debug, Default)]
pub struct Orders(Rc<RefCell<Vec<GameCommand>>>);

impl Orders {
    /// Everything ordered so far, in order. Empties the handle.
    pub fn take(&self) -> Vec<GameCommand> {
        self.0.take()
    }

    fn push(&self, command: GameCommand) {
        self.0.borrow_mut().push(command);
    }
}

pub fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<Orders>("Orders")
        .register_fn(
            "move_unit",
            |orders: &mut Orders, unit: INT, path: Array| -> ScriptResult<()> {
                let path = path
                    .iter()
                    .map(|step| {
                        step.read_lock::<rhai::Map>()
                            .ok_or_else(|| {
                                format!("a path is made of coordinates, got {step}").into()
                            })
                            .and_then(|step| coords::from_map(&step))
                    })
                    .collect::<ScriptResult<Vec<_>>>()?;
                orders.push(GameCommand::MoveUnit {
                    unit: entity_id::<UnitId>(unit, "unit")?,
                    path,
                });
                Ok(())
            },
        )
        .register_fn(
            "found_settlement",
            |orders: &mut Orders, founder: INT, name: ImmutableString| -> ScriptResult<()> {
                orders.push(GameCommand::FoundSettlement {
                    founder: entity_id::<UnitId>(founder, "unit")?,
                    name: name.to_string(),
                });
                Ok(())
            },
        )
        .register_fn(
            "construct_building",
            // on a tile of the nation's own land: it belongs to the nearest settlement
            |orders: &mut Orders, template: ImmutableString, tile: Map| -> ScriptResult<()> {
                orders.push(GameCommand::ConstructBuilding {
                    template: SimulationID::new_abstract_id(&template),
                    tile: coords::from_map(&tile)?,
                });
                Ok(())
            },
        )
        .register_fn(
            "upgrade_building",
            |orders: &mut Orders, building: INT| -> ScriptResult<()> {
                orders.push(GameCommand::UpgradeBuilding {
                    building: entity_id::<BuildingId>(building, "building")?,
                });
                Ok(())
            },
        );
}
