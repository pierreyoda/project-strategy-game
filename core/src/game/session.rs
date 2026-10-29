//! The façade the client holds: one session, one world, one turn loop.
//!
//! Everything the WASM bridge can do, it does here. The bridge owns a `GameSession` and nothing
//! else - no reference into the world, no entity handle - which is what keeps the boundary a
//! command/event/buffer boundary rather than an object graph.

use std::collections::BTreeMap;

use crate::{
    game::{
        GameTurnScalar,
        ai::{ControllerSlot, NationView},
        commands::{self, CommandRejection, GameCommand},
        events::{EventBuffer, GameEvent},
        journal::{ChangeJournal, StateVersion},
        projection::{self, Projection, WorldSnapshot},
        rng::GameSeed,
        state::{World, WorldMut},
        turn::{StepBudget, TurnPhase, TurnProgress, TurnState},
    },
    simulation::{
        ids::{EntityIndex, NationId, UnitId},
        resources::ResourceDataStore,
    },
};

/// How far a settlement watches around itself.
const SETTLEMENT_SIGHT: u32 = 2;

/// One playthrough.
#[derive(Debug)]
pub struct GameSession {
    world: World,
    journal: ChangeJournal,
    events: EventBuffer,
    seed: GameSeed,
    /// The nation this client plays. Orders are only taken from it.
    local_player: NationId,
    controllers: BTreeMap<NationId, ControllerSlot>,
    /// Every accepted command, in order: `seed + command log` replays the playthrough exactly.
    command_log: Vec<LoggedCommand>,
    /// The resolution in progress, if any.
    turn_state: Option<TurnState>,
}

/// An accepted command, with what it needs to be replayed in the right place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedCommand {
    pub turn: GameTurnScalar,
    pub nation: NationId,
    pub command: GameCommand,
}

impl GameSession {
    pub fn new(world: World, seed: GameSeed, local_player: NationId) -> Self {
        let mut session = Self {
            world,
            journal: ChangeJournal::new(),
            events: EventBuffer::new(),
            seed,
            local_player,
            controllers: BTreeMap::new(),
            command_log: Vec::new(),
            turn_state: None,
        };
        session
            .controllers
            .insert(local_player, ControllerSlot::Human);
        // the first turn starts the way every other one ends: everyone can move, and everyone looks
        session.refill_movement();
        session.recompute_all_fog();
        session.journal.commit();
        session.events.push(GameEvent::TurnStarted {
            turn: session.world.turn(),
        });
        session
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn seed(&self) -> GameSeed {
        self.seed
    }

    pub fn turn(&self) -> GameTurnScalar {
        self.world.turn()
    }

    pub fn version(&self) -> StateVersion {
        self.journal.version()
    }

    pub fn local_player(&self) -> NationId {
        self.local_player
    }

    pub fn command_log(&self) -> &[LoggedCommand] {
        &self.command_log
    }

    /// Who plays a nation. A nation with no controller is left inert.
    pub fn set_controller(&mut self, nation: NationId, controller: ControllerSlot) {
        self.controllers.insert(nation, controller);
    }

    pub fn controller(&self, nation: NationId) -> Option<&ControllerSlot> {
        self.controllers.get(&nation)
    }

    /// Take an order from the local player.
    ///
    /// Applied immediately: this is a single-player session with no server to wait for, so deferring
    /// the player's own orders to the end of the turn would buy nothing and cost responsiveness.
    pub fn submit(&mut self, command: GameCommand) -> Result<(), CommandRejection> {
        let issuer = self.local_player;
        if self.world.active_nation() != issuer {
            return Err(CommandRejection::NotYourTurn {
                active: self.world.active_nation(),
            });
        }
        self.apply_command(&command, issuer)?;
        self.command_log.push(LoggedCommand {
            turn: self.world.turn(),
            nation: issuer,
            command,
        });
        self.journal.commit();
        Ok(())
    }

    /// Everything buffered since the last drain.
    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        self.events.drain()
    }

    /// Start resolving a turn. Does nothing if one is already being resolved.
    pub fn begin_turn(&mut self) {
        if self.turn_state.is_some() {
            return;
        }
        let ai_nations: Vec<NationId> = self
            .controllers
            .iter()
            .filter(|(_, controller)| controller.is_ai())
            .map(|(&nation, _)| nation)
            .collect();
        self.turn_state = Some(TurnState::begin(ai_nations));
    }

    /// Whether a turn is being resolved right now.
    pub fn is_resolving(&self) -> bool {
        self.turn_state.is_some()
    }

    /// Do one bounded slice of turn resolution.
    ///
    /// The caller loops on this: it is where progress is reported from, and the only point at which
    /// the worker driving it can read its inbox.
    pub fn step(&mut self, budget: StepBudget) -> TurnProgress {
        let Some(mut state) = self.turn_state.take() else {
            return TurnProgress::Idle;
        };

        if let Some(phase) = state.current_phase().cloned() {
            match phase {
                // the player's orders applied as they were given: there is nothing left to commit
                TurnPhase::PlayerCommit => state.finish_phase(),
                TurnPhase::AiPlanning { nation } => {
                    if self.step_ai(nation, budget) {
                        state.finish_phase();
                    }
                }
                TurnPhase::Economy => {
                    if self.step_economy(&mut state, budget) {
                        state.finish_phase();
                    }
                }
                // TODO: supply propagation along the logistics network
                TurnPhase::Logistics => state.finish_phase(),
                // TODO: combat resolution, attrition
                TurnPhase::Military => state.finish_phase(),
                // TODO: growth, migration, unrest
                TurnPhase::Population => state.finish_phase(),
                TurnPhase::Fog => {
                    self.recompute_all_fog();
                    state.finish_phase();
                }
                TurnPhase::Finalize => {
                    self.finalize_turn();
                    state.finish_phase();
                }
            }
        }

        if state.is_done() {
            let version = self.journal.commit();
            return TurnProgress::Done {
                turn: self.world.turn(),
                version,
            };
        }
        let percent = state.percent();
        let phase = state
            .current_phase()
            .cloned()
            .unwrap_or(TurnPhase::Finalize);
        self.turn_state = Some(state);
        TurnProgress::Working { phase, percent }
    }

    /// Everything the viewer can see, from scratch.
    pub fn snapshot(&self, viewer: NationId) -> WorldSnapshot {
        projection::snapshot(&self.world, viewer, self.journal.version())
    }

    /// What the viewer has to be sent to catch up with `since`.
    ///
    /// Returns a full snapshot when `since` is too old to catch up from, or when so much changed
    /// that a delta would cost more. That choice is made here rather than client-side so that there
    /// is exactly one rule for it.
    pub fn project(&self, viewer: NationId, since: StateVersion) -> Projection {
        match self.journal.changes_since(since) {
            Some(changes) if projection::delta_is_worth_it(&self.world, &changes) => {
                Projection::Delta(projection::delta(&self.world, viewer, &changes))
            }
            _ => Projection::Full(self.snapshot(viewer)),
        }
    }

    fn apply_command(
        &mut self,
        command: &GameCommand,
        issuer: NationId,
    ) -> Result<(), CommandRejection> {
        let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
        commands::apply(command, issuer, &mut world, &mut self.events)
    }

    /// Let one AI nation plan, and apply what it came up with. Returns whether it is done.
    fn step_ai(&mut self, nation: NationId, budget: StepBudget) -> bool {
        let (planned, needs_more) = match self.controllers.get_mut(&nation) {
            Some(ControllerSlot::Ai(controller)) => {
                let view = NationView::new(&self.world, nation);
                let planned = controller.plan(&view, budget);
                (planned, controller.needs_more_steps())
            }
            _ => (Vec::new(), false),
        };

        for command in &planned {
            // the AI goes through the very same validation a player does: a command it gets wrong
            // is a bug report, not a world quietly corrupted
            if let Err(rejection) = self.apply_command(command, nation) {
                self.events.push(GameEvent::CommandRejected {
                    nation,
                    reason: rejection.to_string(),
                });
            } else {
                self.command_log.push(LoggedCommand {
                    turn: self.world.turn(),
                    nation,
                    command: command.clone(),
                });
            }
        }
        !needs_more
    }

    /// Tick every nation's stockpiles, a few per step. Returns whether all of them are done.
    fn step_economy(&mut self, state: &mut TurnState, budget: StepBudget) -> bool {
        let nations: Vec<NationId> = self.world.nations().ids().collect();
        let start = state.cursor();
        let end = (start + budget.work_units as usize).min(nations.len());
        for &nation in &nations[start.min(nations.len())..end] {
            let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
            let shortage = world
                .nation_mut(nation)
                .and_then(|nation| nation.resources_mut().update());
            if let Some(resource) = shortage {
                self.events
                    .push(GameEvent::ResourceShortage { nation, resource });
            }
        }
        state.advance_cursor(end - start.min(end));
        end >= nations.len()
    }

    fn finalize_turn(&mut self) {
        self.refill_movement();
        let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
        let turn = world.advance_turn();
        world.set_active_nation(self.local_player);
        self.events.push(GameEvent::TurnStarted { turn });
    }

    /// Give every unit its movement points back.
    fn refill_movement(&mut self) {
        let refills: Vec<(UnitId, u8)> = self
            .world
            .units()
            .iter()
            .map(|(id, unit)| {
                let movement = self
                    .world
                    .unit_template(unit.template())
                    .map_or(1, |template| template.movement());
                (id, movement)
            })
            .collect();
        let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
        for (id, movement) in refills {
            if let Some(unit) = world.unit_mut(id) {
                unit.refill_movement(movement);
            }
        }
    }

    /// Redo every nation's sighting pass, dirtying only the tiles whose visibility actually moved.
    fn recompute_all_fog(&mut self) {
        let nations: Vec<NationId> = self.world.nations().ids().collect();
        for nation in nations {
            // read what this nation watches before touching its fog record
            let mut watched: Vec<u32> = Vec::new();
            for (_, unit) in self.world.units().iter() {
                if unit.owner() != nation {
                    continue;
                }
                let sight = self
                    .world
                    .unit_template(unit.template())
                    .map_or(1, |template| template.sight());
                watched.extend(self.world.tiles_within(unit.position(), sight as u32));
            }
            for (_, settlement) in self.world.settlements().iter() {
                if settlement.owner() != nation {
                    continue;
                }
                watched.extend(
                    self.world
                        .tiles_within(settlement.position(), SETTLEMENT_SIGHT),
                );
            }

            let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
            let changed: Vec<usize> = {
                let Some(fog) = world.fog_mut(nation) else {
                    continue;
                };
                fog.begin_recompute();
                for index in watched {
                    fog.reveal(index as usize);
                }
                fog.end_recompute().collect()
            };
            // only the tiles whose visibility moved: re-dirtying a whole field of view every turn
            // would make the delta the size of the map in all but name
            for index in changed {
                world.mark_tile_index(index as u32);
            }
        }
    }
}

/// Convenience for the bridge and for tests: the stub AI for every nation but the local player.
pub fn fill_with_stub_ai(session: &mut GameSession) {
    use crate::game::ai::stub_ai_for;
    let seed = session.seed();
    let local = session.local_player();
    let nations: Vec<NationId> = session.world().nations().ids().collect();
    for nation in nations {
        if nation != local {
            debug_assert!(nation.index() <= NationId::MAX_INDEX);
            session.set_controller(nation, stub_ai_for(seed, nation));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        game::{
            ai::{ControllerSlot, NationController, NationView},
            commands::{CommandRejection, GameCommand},
            events::GameEvent,
            projection::{NO_NATION, Projection},
            rng::GameSeed,
            state::World,
            turn::{StepBudget, TurnProgress},
        },
        hex_map::{
            HexMap, HexMapStorage,
            coordinates::{CubeCoords, OffsetCoords},
            shape::RectangularHexMapShape,
            tile::HexMapTile,
        },
        simulation::{
            fog::TileVisibility,
            ids::{NationId, SimulationID, UnitId},
            military::{Unit, UnitTemplate},
            nations::Nation,
        },
    };

    use super::{GameSession, fill_with_stub_ai};

    const SEED: GameSeed = 20_260_129;
    const SCOUT: &str = "scout";

    fn scout_template() -> UnitTemplate {
        UnitTemplate::new(
            SimulationID::new_abstract_id(SCOUT),
            "scout".to_string(),
            2,
            1,
        )
    }

    fn at(col: i16, row: i16) -> CubeCoords {
        OffsetCoords::new(col, row).into()
    }

    /// Two nations, one scout each, far enough apart not to see one another.
    fn two_nation_session() -> (GameSession, NationId, NationId, UnitId, UnitId) {
        let shape = RectangularHexMapShape::new(10, 8);
        let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
        for coords in shape.coords() {
            tiles.insert(coords, HexMapTile::from_properties(120));
        }
        let mut world = World::new(shape, HexMap::from_tiles(tiles));
        world.register_unit_template(scout_template());

        let first = world.register_nation(Nation::new(
            SimulationID::new_abstract_id("first"),
            "First".to_string(),
        ));
        let second = world.register_nation(Nation::new(
            SimulationID::new_abstract_id("second"),
            "Second".to_string(),
        ));
        let template = SimulationID::new_abstract_id(SCOUT);
        let own = world.place_unit(Unit::recruit(template.clone(), first, at(1, 1)));
        let rival = world.place_unit(Unit::recruit(template, second, at(8, 6)));

        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);
        (session, first, second, own, rival)
    }

    /// Drive a whole turn the way the worker does, returning the phases it went through.
    fn resolve_turn(session: &mut GameSession) -> (Vec<String>, TurnProgress) {
        session.begin_turn();
        let mut phases = Vec::new();
        // a turn that needs this many steps is a turn that is not advancing
        for _ in 0..256 {
            match session.step(StepBudget::default()) {
                TurnProgress::Working { phase, percent } => {
                    assert!(percent <= 100);
                    phases.push(phase.name().to_string());
                }
                done @ TurnProgress::Done { .. } => return (phases, done),
                TurnProgress::Idle => panic!("the session stopped resolving mid-turn"),
            }
        }
        panic!("the turn never finished");
    }

    #[test]
    fn test_an_order_applies_at_once_and_dirties_both_tiles() {
        let (mut session, first, _, own, _) = two_nation_session();
        let before = session.version();
        let destination = at(2, 1);

        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![destination],
            })
            .expect("a legal move");

        assert_eq!(
            session.world().units().get(own).unwrap().position(),
            destination
        );
        assert!(session.version() > before);
        assert_eq!(session.command_log().len(), 1);

        let Projection::Delta(delta) = session.project(first, before) else {
            panic!("a single move must not cost a full snapshot");
        };
        let from = session.world().tile_index(at(1, 1)).unwrap();
        let to = session.world().tile_index(destination).unwrap();
        assert!(delta.tile_indices.contains(&from));
        assert!(delta.tile_indices.contains(&to));
        assert_eq!(delta.unit_ids, vec![own.raw()]);
        assert_eq!(delta.unit_tiles, vec![to]);
    }

    #[test]
    fn test_an_illegal_order_changes_nothing() {
        let (mut session, _, _, own, rival) = two_nation_session();
        let origin = session.world().units().get(own).unwrap().position();

        // not adjacent
        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(5, 5)],
            }),
            Err(CommandRejection::IllegalMove {
                reason: "the path skips a tile"
            })
        );
        // someone else's unit
        assert!(matches!(
            session.submit(GameCommand::MoveUnit {
                unit: rival,
                path: vec![at(7, 6)],
            }),
            Err(CommandRejection::NotYours(_))
        ));
        // further than its movement allowance
        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1), at(3, 1), at(4, 1)],
            }),
            Err(CommandRejection::IllegalMove {
                reason: "not enough movement points left"
            })
        );

        assert_eq!(session.world().units().get(own).unwrap().position(), origin);
        assert!(session.command_log().is_empty());
    }

    #[test]
    fn test_a_turn_resolves_phase_by_phase() {
        let (mut session, _, _, _, _) = two_nation_session();
        assert_eq!(session.turn(), 1);
        assert_eq!(session.step(StepBudget::default()), TurnProgress::Idle);

        let (phases, done) = resolve_turn(&mut session);
        // the stub AI plays the one nation the local player does not
        assert_eq!(
            phases,
            vec![
                "ai_planning",
                "economy",
                "logistics",
                "military",
                "population",
                "fog",
                "finalize",
            ]
        );
        let TurnProgress::Done { turn, version } = done else {
            panic!("the turn must end on `Done`");
        };
        assert_eq!(turn, 2);
        assert_eq!(version, session.version());
        assert_eq!(session.turn(), 2);
        assert!(!session.is_resolving());
        assert!(
            session
                .drain_events()
                .contains(&GameEvent::TurnStarted { turn: 2 })
        );
    }

    #[test]
    fn test_a_turn_refills_what_the_orders_spent() {
        let (mut session, _, _, own, _) = two_nation_session();
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1), at(3, 1)],
            })
            .expect("a legal move");
        assert_eq!(session.world().units().get(own).unwrap().movement_left(), 0);

        resolve_turn(&mut session);
        assert_eq!(session.world().units().get(own).unwrap().movement_left(), 2);
    }

    /// A controller that asks for something it is not allowed to do.
    #[derive(Debug)]
    struct CheatingController {
        target: UnitId,
    }

    impl NationController for CheatingController {
        fn name(&self) -> &str {
            "cheating"
        }

        fn plan(&mut self, _view: &NationView<'_>, _budget: StepBudget) -> Vec<GameCommand> {
            vec![GameCommand::MoveUnit {
                unit: self.target,
                path: vec![at(2, 1)],
            }]
        }
    }

    #[test]
    fn test_the_ai_is_held_to_the_rules_the_player_is() {
        let (mut session, _, second, own, _) = two_nation_session();
        let origin = session.world().units().get(own).unwrap().position();
        session.set_controller(
            second,
            ControllerSlot::Ai(Box::new(CheatingController { target: own })),
        );

        resolve_turn(&mut session);

        // the other nation's unit did not budge, and the attempt came back as a rejection
        assert_eq!(session.world().units().get(own).unwrap().position(), origin);
        let rejected = session.drain_events().into_iter().any(
            |event| matches!(event, GameEvent::CommandRejected { nation, .. } if nation == second),
        );
        assert!(rejected, "an illegal AI order must surface as a rejection");
    }

    #[test]
    fn test_the_projection_shows_only_what_the_viewer_can_see() {
        let (session, first, second, own, rival) = two_nation_session();
        let snapshot = session.snapshot(first);

        // its own scout, and not the one on the far side of the map
        assert_eq!(snapshot.unit_ids, vec![own.raw()]);
        assert!(!snapshot.unit_ids.contains(&rival.raw()));

        let watched = session.world().tile_index(at(1, 1)).unwrap() as usize;
        let unseen = session.world().tile_index(at(8, 6)).unwrap() as usize;
        assert_eq!(snapshot.visibility[watched], TileVisibility::Visible as u8);
        assert_eq!(
            snapshot.visibility[unseen],
            TileVisibility::Unexplored as u8
        );
        // nothing of an unexplored tile leaks, not even its elevation
        assert_eq!(snapshot.elevations[unseen], 0);
        assert_eq!(snapshot.tile_owner[unseen], NO_NATION);
        assert_eq!(snapshot.elevations[watched], 120);

        // and the other nation sees the mirror image of that
        let rival_snapshot = session.snapshot(second);
        assert_eq!(rival_snapshot.unit_ids, vec![rival.raw()]);
        assert_eq!(
            rival_snapshot.visibility[watched],
            TileVisibility::Unexplored as u8
        );
    }

    #[test]
    fn test_a_version_that_cannot_be_caught_up_with_falls_back_to_a_snapshot() {
        let (session, first, _, _, _) = two_nation_session();
        assert!(matches!(
            session.project(first, session.version()),
            Projection::Delta(_)
        ));
        // ahead of the session: nothing sane can be sent but everything
        assert!(matches!(
            session.project(first, session.version() + 1),
            Projection::Full(_)
        ));
    }

    #[test]
    fn test_a_playthrough_is_reproducible_from_its_seed() {
        let log_of_three_turns = || {
            let (mut session, _, _, own, _) = two_nation_session();
            session
                .submit(GameCommand::MoveUnit {
                    unit: own,
                    path: vec![at(2, 1)],
                })
                .expect("a legal move");
            for _ in 0..3 {
                resolve_turn(&mut session);
            }
            (
                session.command_log().to_vec(),
                session.snapshot(session.local_player()).visibility,
            )
        };
        assert_eq!(log_of_three_turns(), log_of_three_turns());
    }

    #[test]
    fn test_orders_are_only_taken_from_the_nation_being_played() {
        let (mut session, _, second, _, rival) = two_nation_session();
        session.set_controller(second, ControllerSlot::Human);
        // the active nation is the local player, so an order about another nation's unit is refused
        // as "not yours" rather than silently applied
        assert!(matches!(
            session.submit(GameCommand::FoundSettlement {
                founder: rival,
                name: "Nowhere".to_string(),
            }),
            Err(CommandRejection::NotYours(_))
        ));
    }
}
