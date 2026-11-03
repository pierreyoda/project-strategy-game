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
        economy,
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
        // the player is looking at the result right now, so their own sighting is redone at once
        // rather than at the turn's `Fog` phase: a scout that steps forward reveals what it sees.
        // The AI's own fog waits for that phase, since nothing is watching it move.
        self.recompute_fog_of(issuer);
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
                for message in controller.take_log() {
                    self.events.push(GameEvent::ControllerLog {
                        nation,
                        controller: controller.name().to_string(),
                        message,
                    });
                }
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
            // its income is in: its buildings pay and make on top of it
            economy::run_buildings(&mut world, nation, &mut self.events);
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

    /// Redo every nation's sighting pass.
    fn recompute_all_fog(&mut self) {
        let nations: Vec<NationId> = self.world.nations().ids().collect();
        for nation in nations {
            self.recompute_fog_of(nation);
        }
    }

    /// Redo one nation's sighting pass, dirtying only the tiles whose visibility actually moved.
    fn recompute_fog_of(&mut self, nation: NationId) {
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
            watched.extend(self.world.tiles_in_sight(unit.position(), sight as u32));
        }
        for (_, settlement) in self.world.settlements().iter() {
            if settlement.owner() != nation {
                continue;
            }
            watched.extend(
                self.world
                    .tiles_in_sight(settlement.position(), SETTLEMENT_SIGHT),
            );
        }

        let mut world = WorldMut::new(&mut self.world, self.journal.pending_mut());
        let changed: Vec<usize> = {
            let Some(fog) = world.fog_mut(nation) else {
                return;
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
            genesis::{Homeland, WorldGenesis},
            journal::TurnChanges,
            projection::{NO_NATION, Projection},
            rng::GameSeed,
            state::{World, WorldMut},
            turn::{StepBudget, TurnProgress},
        },
        hex_map::{
            HexMap, HexMapStorage,
            coordinates::{CubeCoords, OffsetCoords},
            shape::RectangularHexMapShape,
            terrain::TerrainKind,
            tile::HexMapTile,
        },
        simulation::{
            buildings::{Building, BuildingTemplate},
            fog::TileVisibility,
            ids::{
                BuildingId, EntityIndex, EntityRef, NationId, SettlementId, SimulationID, UnitId,
            },
            military::{Unit, UnitTemplate},
            movement,
            nations::Nation,
            resources::{Resource, ResourceDataStorage, ResourceDataStore, ResourceDeposit},
        },
    };

    use crate::simulation::ids::DepositId;

    use super::{GameSession, fill_with_stub_ai};

    const SEED: GameSeed = 20_260_129;
    const SCOUT: &str = "scout";
    const MINE: &str = "mine";

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

    /// A free building, so that a test can construct one without a stockpile to budget for.
    fn mine_template() -> crate::simulation::buildings::BuildingTemplate {
        crate::simulation::buildings::BuildingTemplate::new(
            SimulationID::new_abstract_id(MINE),
            "mine".to_string(),
            None,
            vec![crate::simulation::buildings::BuildingLevel {
                cost: Default::default(),
                turns: 1,
                maintenance: Default::default(),
                production: Default::default(),
                sprite: "mine_1".to_string(),
            }],
        )
    }

    /// Two nations, one scout each, far enough apart not to see one another.
    fn two_nation_session() -> (GameSession, NationId, NationId, UnitId, UnitId) {
        let (world, first, second, own, rival) = two_nation_world();
        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);
        (session, first, second, own, rival)
    }

    /// The world of `two_nation_session`, before the session starts watching it.
    fn two_nation_world() -> (World, NationId, NationId, UnitId, UnitId) {
        let shape = RectangularHexMapShape::new(10, 8);
        let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
        for coords in shape.coords() {
            tiles.insert(coords, HexMapTile::from_properties(120));
        }
        let mut world = World::new(shape, HexMap::from_tiles(tiles));
        world.register_unit_template(scout_template());
        world.register_building_template(mine_template());

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
        (world, first, second, own, rival)
    }

    /// `two_nation_session` on the ground `terrain` paints by column and row, with `extra` scouts
    /// of the first nation placed at the given columns and rows.
    fn session_on(
        terrain: impl Fn(i16, i16) -> TerrainKind,
        extra: &[(i16, i16)],
    ) -> (GameSession, NationId, NationId, UnitId, UnitId, Vec<UnitId>) {
        let (mut world, first, second, own, rival) = two_nation_world();
        let kinds: Vec<TerrainKind> = world
            .shape()
            .coords()
            .map(|coords| {
                let offset = OffsetCoords::from(coords);
                terrain(offset.col(), offset.row())
            })
            .collect();
        world.paint_terrain(&kinds);
        let template = SimulationID::new_abstract_id(SCOUT);
        let extras = extra
            .iter()
            .map(|&(col, row)| {
                world.place_unit(Unit::recruit(template.clone(), first, at(col, row)))
            })
            .collect();
        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);
        (session, first, second, own, rival, extras)
    }

    fn movement_left(session: &GameSession, unit: UnitId) -> u8 {
        session.world().units().get(unit).unwrap().movement_left()
    }

    fn position(session: &GameSession, unit: UnitId) -> CubeCoords {
        session.world().units().get(unit).unwrap().position()
    }

    #[test]
    fn test_a_hill_costs_two_points_and_water_is_not_entered() {
        let ground = |col: i16, row: i16| match (col, row) {
            (2, 1) => TerrainKind::Hills,
            (1, 2) => TerrainKind::ShallowWater,
            _ => TerrainKind::Flat,
        };
        let (mut session, _, _, own, _, _) = session_on(ground, &[]);
        assert_eq!(movement_left(&session, own), 2);

        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(1, 2)],
            }),
            Err(CommandRejection::IllegalMove {
                reason: "the ground cannot be crossed",
            })
        );
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            })
            .expect("a hill is slow, not forbidden");
        assert_eq!(position(&session, own), at(2, 1));
        assert_eq!(movement_left(&session, own), 0, "the hill took both points");
        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(3, 1)],
            }),
            Err(CommandRejection::IllegalMove {
                reason: "not enough movement points left",
            })
        );
    }

    #[test]
    fn test_the_last_point_still_enters_a_hill() {
        let ground = |col: i16, row: i16| match (col, row) {
            (3, 1) => TerrainKind::Hills,
            _ => TerrainKind::Flat,
        };
        let (mut session, _, _, own, _, _) = session_on(ground, &[]);
        // one flat step leaves one point; the hill costs two, and takes the one
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1), at(3, 1)],
            })
            .expect("a unit is never stranded a point short of the tile it reached");
        assert_eq!(position(&session, own), at(3, 1));
        assert_eq!(movement_left(&session, own), 0);
    }

    #[test]
    fn test_a_tile_holding_a_unit_is_not_entered_nor_crossed() {
        let (mut session, _, _, own, _, extras) = session_on(|_, _| TerrainKind::Flat, &[(2, 1)]);
        assert_eq!(extras.len(), 1);
        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            }),
            Err(CommandRejection::TileOccupied)
        );
        assert_eq!(
            session.submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1), at(3, 1)],
            }),
            Err(CommandRejection::TileOccupied),
            "no passing through a friend either: there is no stacking"
        );
        // and the whole order is refused: the unit has not budged
        assert_eq!(position(&session, own), at(1, 1));
        assert_eq!(movement_left(&session, own), 2);
    }

    #[test]
    fn test_reachable_agrees_with_the_rules_and_hides_the_unknown() {
        let ground = |col: i16, row: i16| match (col, row) {
            (0, 1) => TerrainKind::DeepWater,
            (2, 1) => TerrainKind::Hills,
            _ => TerrainKind::Flat,
        };
        let (session, first, second, own, rival, _) = session_on(ground, &[(1, 2)]);
        let world = session.world();
        let index = |col, row| world.tile_index(at(col, row)).unwrap();
        let reach = movement::reachable(world, first, own).expect("the viewer's own unit");

        assert!(reach.tiles.contains(&index(1, 1)), "the origin is in");
        assert_eq!(reach.path_to(index(1, 1)), Some(vec![]));
        assert!(
            reach.tiles.contains(&index(2, 1)),
            "the hill, for both points"
        );
        assert_eq!(
            reach.costs[reach.tiles.binary_search(&index(2, 1)).unwrap()],
            2
        );
        assert!(!reach.tiles.contains(&index(0, 1)), "water");
        assert!(!reach.tiles.contains(&index(1, 2)), "a friend's tile");
        assert!(
            !reach.tiles.contains(&index(3, 1)),
            "two flat steps away, but never seen: the scout sees one tile"
        );
        // every tile offered is one the viewer knows, and every path offered is one the order accepts
        let fog = world.fog_of(first).unwrap();
        for &tile in &reach.tiles {
            assert_ne!(fog.visibility(tile as usize), TileVisibility::Unexplored);
            let path: Vec<CubeCoords> = reach
                .path_to(tile)
                .unwrap()
                .into_iter()
                .map(|step| world.tile_coords(step).unwrap())
                .collect();
            if !path.is_empty() {
                let mut trial = session_on(ground, &[(1, 2)]).0;
                trial
                    .submit(GameCommand::MoveUnit { unit: own, path })
                    .expect("the preview only offers what the rules accept");
            }
        }
        assert!(
            movement::reachable(world, second, own).is_none(),
            "not the viewer's unit"
        );
        assert!(movement::reachable(world, first, rival).is_none());
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

    /// A controller that plans nothing and says so.
    #[derive(Debug)]
    struct TalkativeController;

    impl NationController for TalkativeController {
        fn name(&self) -> &str {
            "talkative"
        }

        fn plan(&mut self, _view: &NationView<'_>, _budget: StepBudget) -> Vec<GameCommand> {
            Vec::new()
        }

        fn take_log(&mut self) -> Vec<String> {
            vec!["nothing to do".to_string()]
        }
    }

    #[test]
    fn test_what_a_controller_says_lands_in_the_turn_log() {
        let (mut session, _, second, _, _) = two_nation_session();
        session.set_controller(second, ControllerSlot::Ai(Box::new(TalkativeController)));

        resolve_turn(&mut session);

        let logged = session.drain_events().into_iter().any(|event| {
            matches!(event, GameEvent::ControllerLog { nation, ref controller, ref message }
                if nation == second && controller == "talkative" && message == "nothing to do")
        });
        assert!(
            logged,
            "a controller's log must reach the events, under its nation"
        );
    }

    #[test]
    fn test_a_nation_view_knows_no_ground_it_never_explored() {
        let ground = |col: i16, _: i16| {
            if col == 8 {
                TerrainKind::Mountains
            } else {
                TerrainKind::Hills
            }
        };
        let (session, first, _, own, rival, _) = session_on(ground, &[]);
        let view = NationView::new(session.world(), first);

        assert_eq!(
            view.terrain(at(1, 1)),
            Some(TerrainKind::Hills),
            "under its own scout"
        );
        // the rival's corner of the map, which the first nation has never seen
        assert_eq!(view.terrain(at(8, 6)), None);
        assert_eq!(view.terrain(at(1, -1)), None, "beyond the pole");

        let reachable = view.reachable(own).expect("its own unit");
        assert!(
            reachable.tiles.len() > 1,
            "a scout on hills still gets somewhere"
        );
        assert!(view.reachable(rival).is_none(), "not its unit");
    }

    #[test]
    fn test_moving_reveals_the_ground_the_unit_now_watches() {
        let (mut session, first, _, own, _) = two_nation_session();
        let explored = |session: &GameSession| {
            session
                .snapshot(first)
                .visibility
                .iter()
                .filter(|&&seen| seen != TileVisibility::Unexplored as u8)
                .count()
        };
        let before = explored(&session);
        let version = session.version();

        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            })
            .expect("a legal move");

        // the player is watching: a scout that steps forward reveals what it sees, without waiting
        // for the turn to be resolved
        assert!(explored(&session) > before);
        let Projection::Delta(delta) = session.project(first, version) else {
            panic!("a step must not cost a full snapshot");
        };
        // the tile left, the tile reached, and everything whose visibility moved with them
        assert!(delta.tile_indices.len() > 2);
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

    /// `two_nation_world` with the west half of the map in the first nation's sphere of influence
    /// and the east half in the second's.
    fn two_sphere_session() -> (GameSession, NationId, NationId, UnitId) {
        let (mut world, first, second, own, _) = two_nation_world();
        let tile_sphere = world
            .shape()
            .coords()
            .map(|coords| {
                Some(if OffsetCoords::from(coords).col() < 5 {
                    first
                } else {
                    second
                })
            })
            .collect();
        world.set_genesis(WorldGenesis::new(
            tile_sphere,
            vec![
                Homeland::new(first, at(1, 1), 1.0),
                Homeland::new(second, at(8, 6), 1.0),
            ],
        ));
        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);
        (session, first, second, own)
    }

    #[test]
    fn test_a_snapshot_shows_the_viewers_sphere_everywhere_and_a_rivals_once_explored() {
        let (session, first, second, _) = two_sphere_session();
        let snapshot = session.snapshot(first);
        let index = |col, row| (row * 10 + col) as usize;

        // the viewer's own sphere, right up to its edge, explored or not
        assert_eq!(snapshot.tile_sphere[index(1, 1)], first.raw());
        assert_eq!(
            snapshot.visibility[index(4, 7)],
            TileVisibility::Unexplored as u8
        );
        assert_eq!(snapshot.tile_sphere[index(4, 7)], first.raw());
        // the rival's sphere stays unknown where the viewer has never been
        assert_eq!(
            snapshot.visibility[index(8, 6)],
            TileVisibility::Unexplored as u8
        );
        assert_eq!(snapshot.tile_sphere[index(8, 6)], NO_NATION);

        // the rival sees the mirror image
        let snapshot = session.snapshot(second);
        assert_eq!(snapshot.tile_sphere[index(8, 6)], second.raw());
        assert_eq!(snapshot.tile_sphere[index(1, 1)], NO_NATION);
    }

    #[test]
    fn test_a_delta_carries_the_sphere_of_the_tiles_it_touches() {
        let (mut session, first, _, own) = two_sphere_session();
        let since = session.version();
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            })
            .expect("a legal move");
        resolve_turn(&mut session);

        let Projection::Delta(delta) = session.project(first, since) else {
            panic!("a move is a delta");
        };
        assert_eq!(delta.tile_sphere.len(), delta.tile_indices.len());
        assert!(!delta.tile_indices.is_empty());
        for (index, sphere) in delta.tile_indices.iter().zip(&delta.tile_sphere) {
            // the scout stays in the west: everything it touched is in its own sphere
            assert!(index % 10 < 5, "tile {index}");
            assert_eq!(*sphere, first.raw(), "tile {index}");
        }
    }

    #[test]
    fn test_a_world_without_a_genesis_has_no_sphere_to_show() {
        let (session, first, _, _, _) = two_nation_session();
        let snapshot = session.snapshot(first);
        assert!(
            snapshot
                .tile_sphere
                .iter()
                .all(|&sphere| sphere == NO_NATION)
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
    fn test_a_nations_first_settlement_is_projected_as_its_capital() {
        use crate::simulation::settlements::SettlementStage;

        let (mut session, first, _, own, _) = two_nation_session();
        session
            .submit(GameCommand::FoundSettlement {
                founder: own,
                name: "Landfall".to_string(),
            })
            .expect("a legal founding");
        resolve_turn(&mut session);

        let snapshot = session.snapshot(first);
        assert_eq!(snapshot.settlement_ids.len(), 1);
        // its population is nil, but a nation's seat is a role rather than a size
        assert_eq!(
            snapshot.settlement_stages,
            vec![SettlementStage::Capital as u8]
        );
    }

    #[test]
    fn test_a_deposit_is_projected_once_its_tile_is_explored() {
        use crate::{
            game::projection::NO_TURNS_LEFT,
            simulation::resources::{
                Resource, ResourceDataStorage, ResourceDataStore, ResourceDeposit,
            },
        };

        let (mut world, first, second, own, _) = two_nation_world();
        let mut held = ResourceDataStorage::new();
        held.replenish(Resource::Oil, 340);
        let deposit = world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_oil"),
            at(3, 1),
            held,
            600,
        ));
        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);

        // two tiles off: the scout sees one, so the deposit is not known yet
        assert!(session.snapshot(first).deposits.ids.is_empty());
        let version = session.version();
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            })
            .expect("a legal move");

        let Projection::Delta(delta) = session.project(first, version) else {
            panic!("a step must not cost a full snapshot");
        };
        assert_eq!(delta.deposits.ids, vec![deposit.raw()]);
        assert_eq!(delta.deposits.resources, vec![Resource::Oil as u8]);
        // 340 of a kind that comes 600 at most: the middle step
        assert_eq!(delta.deposits.levels, vec![2]);
        assert_eq!(delta.deposits.quantities, vec![340]);
        assert_eq!(delta.deposits.recharges, vec![0]);
        assert_eq!(delta.deposits.turns_left, vec![NO_TURNS_LEFT]);

        // from then on the snapshot carries it, while the other nation still knows nothing of it
        assert_eq!(session.snapshot(first).deposits.ids, vec![deposit.raw()]);
        assert!(session.snapshot(second).deposits.ids.is_empty());
    }

    #[test]
    fn test_units_are_projected_with_their_kind_and_points_left() {
        let (mut session, first, _, own, _) = two_nation_session();
        let snapshot = session.snapshot(first);
        // the kind names a template of the meta, which carries the figures the client shows
        assert_eq!(snapshot.meta.unit_templates.len(), 1);
        assert_eq!(snapshot.meta.unit_templates[0].r#type, "scout");
        assert_eq!(snapshot.meta.unit_templates[0].movement, 2);
        assert_eq!(snapshot.unit_kinds, vec![0]);
        assert_eq!(
            snapshot.unit_movement_left,
            vec![2],
            "a fresh turn's points"
        );

        let version = session.version();
        session
            .submit(GameCommand::MoveUnit {
                unit: own,
                path: vec![at(2, 1)],
            })
            .expect("a legal move");
        // spending points reaches the delta: the unit is journaled by the move
        let Projection::Delta(delta) = session.project(first, version) else {
            panic!("a step must not cost a full snapshot");
        };
        assert_eq!(delta.unit_ids, vec![own.raw()]);
        assert_eq!(delta.unit_movement_left, vec![1]);
    }

    #[test]
    fn test_an_ordered_building_is_projected_with_its_kind_and_level() {
        use crate::simulation::ids::{EntityIndex, SettlementId};

        let (mut session, first, second, own, _) = two_nation_session();
        session
            .submit(GameCommand::FoundSettlement {
                founder: own,
                name: "Landfall".to_string(),
            })
            .expect("a legal founding");
        let settlement =
            SettlementId::from_index(session.snapshot(first).settlement_ids[0] as usize);
        let version = session.version();

        session
            .submit(GameCommand::ConstructBuilding {
                template: SimulationID::new_abstract_id(MINE),
                tile: at(1, 1),
            })
            .expect("a legal construction");
        let building = session
            .world()
            .tile(at(1, 1))
            .unwrap()
            .artificial()
            .buildings()[0];
        assert_eq!(
            session
                .world()
                .buildings()
                .get(building)
                .unwrap()
                .settlement(),
            settlement
        );

        let snapshot = session.snapshot(first);
        let tile = session.world().tile_index(at(1, 1)).unwrap();
        assert_eq!(snapshot.building_ids.len(), 1);
        assert_eq!(snapshot.building_tiles, vec![tile]);
        assert_eq!(snapshot.building_owners, vec![first.raw()]);
        // its kind names a template of the meta, which is how the client tells a mine from a plant
        let templates: Vec<&str> = snapshot
            .meta
            .building_templates
            .iter()
            .map(|template| template.r#type.as_str())
            .collect();
        assert_eq!(templates, ["mine"]);
        assert_eq!(snapshot.building_kinds, vec![0]);
        assert_eq!(snapshot.building_levels, vec![0], "under construction");

        // the order applied at once, so a client one version behind gets it as a delta
        let Projection::Delta(delta) = session.project(first, version) else {
            panic!("a construction must not cost a full snapshot");
        };
        assert_eq!(delta.building_ids, snapshot.building_ids);
        assert_eq!(delta.building_kinds, vec![0]);
        assert!(delta.buildings_removed.is_empty());

        // and a nation that never saw the tile knows nothing of it
        assert!(session.snapshot(second).building_ids.is_empty());
    }

    const PLANT: &str = "plant";
    const DRILL: &str = "drill";

    fn level(credits: u16, turns: u16, metals: u16) -> crate::simulation::buildings::BuildingLevel {
        crate::simulation::buildings::BuildingLevel {
            cost: [(Resource::Credits, credits)]
                .into_iter()
                .filter(|&(_, n)| n > 0)
                .collect(),
            turns,
            maintenance: Default::default(),
            production: [(Resource::Metals, metals)]
                .into_iter()
                .filter(|&(_, n)| n > 0)
                .collect(),
            sprite: String::new(),
        }
    }

    /// Two levels, paid in credits: 40 and 2 turns for the first, 60 and 3 for the second.
    fn plant_template() -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id(PLANT),
            PLANT.to_string(),
            None,
            vec![level(40, 2, 0), level(60, 3, 0)],
        )
    }

    /// An extractor of metals: free, a turn per level.
    fn drill_template() -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id(DRILL),
            DRILL.to_string(),
            Some(Resource::Metals),
            vec![level(0, 1, 4)],
        )
    }

    /// The first nation settled at (1, 1) - which claims the ring around it - holding `credits`. In
    /// that ring: water at (0, 1), metals at (2, 2), metals run dry at (1, 2), oil at (2, 0); (2, 1)
    /// and (1, 0) are plain land.
    fn builders_session(credits: u16) -> (GameSession, NationId, NationId, SettlementId) {
        let (mut world, first, second, own, _) = two_nation_world();
        world.register_building_template(plant_template());
        world.register_building_template(drill_template());
        let kinds: Vec<TerrainKind> = world
            .shape()
            .coords()
            .map(|coords| {
                if coords == at(0, 1) {
                    TerrainKind::ShallowWater
                } else {
                    TerrainKind::Flat
                }
            })
            .collect();
        world.paint_terrain(&kinds);
        for (resource, stock, col, row) in [
            (Resource::Metals, 300, 2, 2),
            (Resource::Metals, 0, 1, 2),
            (Resource::Oil, 300, 2, 0),
        ] {
            let mut held = ResourceDataStorage::new();
            held.replenish(resource, stock);
            world.place_deposit(ResourceDeposit::new(
                SimulationID::new_abstract_id("deposit"),
                at(col, row),
                held,
                600,
            ));
        }
        let mut changes = TurnChanges::new();
        WorldMut::new(&mut world, &mut changes)
            .nation_mut(first)
            .unwrap()
            .resources_mut()
            .replenish(Resource::Credits, credits);

        let mut session = GameSession::new(world, SEED, first);
        fill_with_stub_ai(&mut session);
        session
            .submit(GameCommand::FoundSettlement {
                founder: own,
                name: "Landfall".to_string(),
            })
            .expect("a legal founding");
        let settlement = session.world().settlements().ids().next().unwrap();
        session.drain_events();
        (session, first, second, settlement)
    }

    fn construct(
        session: &mut GameSession,
        template: &str,
        col: i16,
        row: i16,
    ) -> Result<(), CommandRejection> {
        session.submit(GameCommand::ConstructBuilding {
            template: SimulationID::new_abstract_id(template),
            tile: at(col, row),
        })
    }

    fn upgrade(session: &mut GameSession, building: BuildingId) -> Result<(), CommandRejection> {
        session.submit(GameCommand::UpgradeBuilding { building })
    }

    fn building_at(session: &GameSession, col: i16, row: i16) -> BuildingId {
        session
            .world()
            .tile(at(col, row))
            .unwrap()
            .artificial()
            .buildings()[0]
    }

    fn credits(session: &GameSession, nation: NationId) -> u16 {
        session
            .world()
            .nations()
            .get(nation)
            .unwrap()
            .resources()
            .quantity_of(Resource::Credits)
    }

    /// Run a building's works to completion, as the Economy phase does once it advances them.
    fn finish_works(session: &mut GameSession, building: BuildingId) {
        let mut changes = TurnChanges::new();
        let mut world = WorldMut::new(&mut session.world, &mut changes);
        let target = world.building_mut(building).unwrap();
        while target.works().is_some() {
            target.advance_works();
        }
    }

    #[test]
    fn test_a_building_is_ordered_on_the_nations_own_land() {
        let (mut session, first, _, settlement) = builders_session(100);

        construct(&mut session, PLANT, 2, 1).expect("own land, free, affordable");

        let building = building_at(&session, 2, 1);
        let ordered = session.world().buildings().get(building).unwrap();
        assert_eq!(ordered.level(), 0, "under construction");
        assert_eq!(
            ordered
                .works()
                .map(|works| (works.turns_left(), works.turns_total())),
            Some((2, 2)),
            "level 1's turns"
        );
        assert_eq!((ordered.owner(), ordered.settlement()), (first, settlement));
        assert_eq!(credits(&session, first), 60, "level 1's 40, paid at once");
        let started = session.drain_events().into_iter().any(|event| {
            event
                == GameEvent::ConstructionStarted {
                    nation: first,
                    building,
                    settlement,
                }
        });
        assert!(started);
    }

    #[test]
    fn test_a_building_needs_free_land_the_nation_owns() {
        let (mut session, ..) = builders_session(1000);
        let refused = |reason| Err(CommandRejection::IllegalPlacement { reason });

        assert_eq!(
            construct(&mut session, PLANT, 5, 5),
            refused("the tile is not the nation's land")
        );
        assert_eq!(
            construct(&mut session, PLANT, 1, -1),
            refused("the tile is off the map")
        );
        assert_eq!(
            construct(&mut session, PLANT, 0, 1),
            refused("a building cannot stand on water")
        );
        construct(&mut session, PLANT, 2, 1).expect("plain land");
        assert_eq!(
            construct(&mut session, PLANT, 2, 1),
            refused("the tile is already built on"),
            "one building per tile"
        );
        construct(&mut session, PLANT, 1, 1).expect("the settlement's own tile is land like any");
        assert_eq!(
            construct(&mut session, "nothing", 1, 0),
            Err(CommandRejection::UnknownDefinition(
                SimulationID::new_abstract_id("nothing")
            ))
        );
    }

    #[test]
    fn test_an_extractor_stands_on_a_deposit_of_its_resource_with_something_left() {
        let (mut session, ..) = builders_session(0);
        let missing = Err(CommandRejection::MissingDeposit {
            resource: Resource::Metals,
        });

        assert_eq!(
            construct(&mut session, DRILL, 2, 1),
            missing,
            "no deposit at all"
        );
        assert_eq!(
            construct(&mut session, DRILL, 2, 0),
            missing,
            "oil is no metals"
        );
        assert_eq!(
            construct(&mut session, DRILL, 1, 2),
            missing,
            "metals run dry"
        );
        construct(&mut session, DRILL, 2, 2).expect("on its deposit");

        // it draws once it works: ordering it took nothing out
        let deposit = session.world().tile(at(2, 2)).unwrap().natural().deposits()[0];
        let held = session.world().deposits().get(deposit).unwrap().amount();
        assert_eq!(held, 300);
    }

    #[test]
    fn test_a_building_belongs_to_the_nearest_settlement() {
        let (mut session, first, _, landfall) = builders_session(1000);
        let mut changes = TurnChanges::new();
        let scout = WorldMut::new(&mut session.world, &mut changes).insert_unit(Unit::recruit(
            SimulationID::new_abstract_id(SCOUT),
            first,
            at(5, 1),
        ));
        session
            .submit(GameCommand::FoundSettlement {
                founder: scout,
                name: "Outpost".to_string(),
            })
            .expect("a legal founding");
        let outpost = session
            .world()
            .settlements()
            .ids()
            .find(|&id| id != landfall)
            .unwrap();

        construct(&mut session, PLANT, 2, 1).unwrap();
        construct(&mut session, PLANT, 4, 1).unwrap();

        let settlement_of = |col, row| {
            let building = building_at(&session, col, row);
            session
                .world()
                .buildings()
                .get(building)
                .unwrap()
                .settlement()
        };
        assert_eq!(
            settlement_of(2, 1),
            landfall,
            "one tile from Landfall, three from Outpost"
        );
        assert_eq!(settlement_of(4, 1), outpost, "and the other way round");
    }

    #[test]
    fn test_a_nation_without_a_settlement_cannot_build() {
        let (mut world, first, ..) = two_nation_world();
        world.register_building_template(plant_template());
        let mut changes = TurnChanges::new();
        let mut gate = WorldMut::new(&mut world, &mut changes);
        // land of its own, handed over rather than claimed by a founding
        gate.tile_mut(at(2, 1))
            .unwrap()
            .artificial_mut()
            .set_owner(Some(first));
        gate.nation_mut(first)
            .unwrap()
            .resources_mut()
            .replenish(Resource::Credits, 100);
        let mut session = GameSession::new(world, SEED, first);

        assert_eq!(
            construct(&mut session, PLANT, 2, 1),
            Err(CommandRejection::NoSettlement)
        );
    }

    #[test]
    fn test_a_building_is_paid_in_full_or_not_at_all() {
        let (mut session, first, ..) = builders_session(39);

        assert_eq!(
            construct(&mut session, PLANT, 2, 1),
            Err(CommandRejection::InsufficientResources {
                resource: Resource::Credits,
                missing: 1,
            })
        );
        assert_eq!(credits(&session, first), 39, "nothing taken");
        assert!(
            session
                .world()
                .tile(at(2, 1))
                .unwrap()
                .artificial()
                .buildings()
                .is_empty()
        );

        // exactly what it costs is enough, and all of it goes
        let mut changes = TurnChanges::new();
        WorldMut::new(&mut session.world, &mut changes)
            .nation_mut(first)
            .unwrap()
            .resources_mut()
            .replenish(Resource::Credits, 1);
        construct(&mut session, PLANT, 2, 1).expect("40 for 40");
        assert_eq!(credits(&session, first), 0);
    }

    #[test]
    fn test_a_building_is_raised_one_level_at_a_time() {
        let (mut session, first, second, _) = builders_session(1000);
        construct(&mut session, PLANT, 2, 1).unwrap();
        let building = building_at(&session, 2, 1);

        assert_eq!(
            upgrade(&mut session, building),
            Err(CommandRejection::WorksUnderWay),
            "it is still being built"
        );

        finish_works(&mut session, building);
        session.drain_events();
        upgrade(&mut session, building).expect("level 1 stands, level 2 to go");
        let raised = session.world().buildings().get(building).unwrap();
        assert_eq!(raised.level(), 1, "it works at level 1 while it is raised");
        assert_eq!(
            raised
                .works()
                .map(|works| (works.turns_left(), works.turns_total())),
            Some((3, 3)),
            "level 2's turns"
        );
        assert_eq!(credits(&session, first), 1000 - 40 - 60);
        assert!(session.drain_events().contains(&GameEvent::UpgradeStarted {
            nation: first,
            building,
            level: 2,
        }));

        assert_eq!(
            upgrade(&mut session, building),
            Err(CommandRejection::WorksUnderWay),
            "one level at a time"
        );
        finish_works(&mut session, building);
        assert_eq!(
            session.world().buildings().get(building).unwrap().level(),
            2
        );
        assert_eq!(
            upgrade(&mut session, building),
            Err(CommandRejection::AtMaxLevel)
        );
        assert_eq!(credits(&session, first), 900, "and refusals cost nothing");

        let mut changes = TurnChanges::new();
        let rivals =
            WorldMut::new(&mut session.world, &mut changes).insert_building(Building::order(
                SimulationID::new_abstract_id(PLANT),
                second,
                SettlementId::from_index(0),
                at(8, 6),
                1,
            ));
        assert_eq!(
            upgrade(&mut session, rivals),
            Err(CommandRejection::NotYours(EntityRef::Building(rivals)))
        );
        let nowhere = BuildingId::from_index(99);
        assert_eq!(
            upgrade(&mut session, nowhere),
            Err(CommandRejection::UnknownEntity(EntityRef::Building(
                nowhere
            )))
        );
    }

    const WORKSHOP: &str = "workshop";
    const PUMP: &str = "pump";

    /// Free to reach: level 1 in 2 turns, upkeep 1 credit, 3 food; level 2 in 1 turn, upkeep 2
    /// credits, 5 food.
    fn workshop_template() -> BuildingTemplate {
        let level = |turns, upkeep, food| crate::simulation::buildings::BuildingLevel {
            cost: Default::default(),
            turns,
            maintenance: [(Resource::Credits, upkeep)].into(),
            production: [(Resource::Food, food)].into(),
            sprite: String::new(),
        };
        BuildingTemplate::new(
            SimulationID::new_abstract_id(WORKSHOP),
            WORKSHOP.to_string(),
            None,
            vec![level(2, 1, 3), level(1, 2, 5)],
        )
    }

    /// An extractor of water, free, built in a turn, after 5 water a turn.
    fn pump_template() -> BuildingTemplate {
        BuildingTemplate::new(
            SimulationID::new_abstract_id(PUMP),
            PUMP.to_string(),
            Some(Resource::Water),
            vec![crate::simulation::buildings::BuildingLevel {
                cost: Default::default(),
                turns: 1,
                maintenance: Default::default(),
                production: [(Resource::Water, 5)].into(),
                sprite: String::new(),
            }],
        )
    }

    fn held(session: &GameSession, nation: NationId, resource: Resource) -> u16 {
        session
            .world()
            .nations()
            .get(nation)
            .unwrap()
            .resources()
            .quantity_of(resource)
    }

    fn give(session: &mut GameSession, nation: NationId, resource: Resource, amount: u16) {
        let mut changes = TurnChanges::new();
        WorldMut::new(&mut session.world, &mut changes)
            .nation_mut(nation)
            .unwrap()
            .resources_mut()
            .replenish(resource, amount);
    }

    fn building(session: &GameSession, id: BuildingId) -> &Building {
        session.world().buildings().get(id).unwrap()
    }

    /// `builders_session`, with a workshop ordered on (2, 1).
    fn workshop_session(credits: u16) -> (GameSession, NationId, BuildingId) {
        let (mut session, first, ..) = builders_session(credits);
        session
            .world
            .register_building_template(workshop_template());
        construct(&mut session, WORKSHOP, 2, 1).expect("free, on own land");
        let workshop = building_at(&session, 2, 1);
        session.drain_events();
        (session, first, workshop)
    }

    #[test]
    fn test_a_building_works_from_the_turn_after_its_construction_completes() {
        let (mut session, first, workshop) = workshop_session(10);

        resolve_turn(&mut session);
        assert_eq!(building(&session, workshop).level(), 0);
        assert_eq!(
            building(&session, workshop)
                .works()
                .map(|works| works.turns_left()),
            Some(1)
        );

        resolve_turn(&mut session);
        assert_eq!(building(&session, workshop).level(), 1);
        assert!(
            session
                .drain_events()
                .contains(&GameEvent::BuildingCompleted {
                    nation: first,
                    building: workshop,
                    level: 1,
                })
        );
        // it was still being built when that turn settled it
        assert_eq!(held(&session, first, Resource::Food), 0);
        assert_eq!(held(&session, first, Resource::Credits), 10);

        resolve_turn(&mut session);
        assert_eq!(held(&session, first, Resource::Food), 3, "its production");
        assert_eq!(held(&session, first, Resource::Credits), 9, "its upkeep");
    }

    #[test]
    fn test_an_upgrading_building_works_at_its_level_until_the_upgrade_is_done() {
        let (mut session, first, workshop) = workshop_session(100);
        resolve_turn(&mut session);
        resolve_turn(&mut session);
        upgrade(&mut session, workshop).expect("level 1 stands");

        resolve_turn(&mut session);
        // settled at level 1, then raised: level 2's single turn is over
        assert_eq!(held(&session, first, Resource::Food), 3);
        assert_eq!(held(&session, first, Resource::Credits), 99);
        assert_eq!(building(&session, workshop).level(), 2);

        resolve_turn(&mut session);
        assert_eq!(held(&session, first, Resource::Food), 3 + 5);
        assert_eq!(held(&session, first, Resource::Credits), 99 - 2);
    }

    #[test]
    fn test_a_building_halts_while_its_upkeep_goes_unpaid() {
        let (mut session, first, workshop) = workshop_session(0);
        resolve_turn(&mut session);
        resolve_turn(&mut session);
        session.drain_events();

        resolve_turn(&mut session);
        assert!(building(&session, workshop).halted());
        assert_eq!(held(&session, first, Resource::Food), 0, "nothing made");
        let halted = |events: Vec<GameEvent>| {
            events
                .iter()
                .filter(|event| matches!(event, GameEvent::BuildingHalted { .. }))
                .count()
        };
        assert_eq!(halted(session.drain_events()), 1);

        resolve_turn(&mut session);
        assert_eq!(
            halted(session.drain_events()),
            0,
            "told the turn it stops, once"
        );

        give(&mut session, first, Resource::Credits, 5);
        resolve_turn(&mut session);
        assert!(
            !building(&session, workshop).halted(),
            "paid again, it works again"
        );
        assert_eq!(held(&session, first, Resource::Food), 3);
        assert_eq!(held(&session, first, Resource::Credits), 4);
    }

    #[test]
    fn test_when_upkeep_runs_short_the_earlier_building_is_the_one_paid() {
        let (mut session, first, earlier) = workshop_session(1);
        construct(&mut session, WORKSHOP, 1, 0).unwrap();
        let later = building_at(&session, 1, 0);
        resolve_turn(&mut session);
        resolve_turn(&mut session);

        resolve_turn(&mut session);
        assert!(!building(&session, earlier).halted());
        assert!(building(&session, later).halted());
        assert_eq!(
            held(&session, first, Resource::Food),
            3,
            "the one paid made its food"
        );
        assert_eq!(held(&session, first, Resource::Credits), 0);
    }

    #[test]
    fn test_an_extractor_draws_its_deposit_down_to_nothing() {
        let (mut session, first, ..) = builders_session(0);
        let mut stock = ResourceDataStorage::new();
        stock.replenish(Resource::Metals, 10);
        let deposit = session.world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_metals"),
            at(1, 0),
            stock,
            600,
        ));
        construct(&mut session, DRILL, 1, 0).expect("on its deposit");
        resolve_turn(&mut session);
        session.drain_events();

        let left = |session: &GameSession| {
            let deposit = session.world().deposits().get(deposit).unwrap();
            (deposit.amount(), deposit.turns_left())
        };
        let mut exhausted = 0;
        for (metals, stock, turns_left) in [(4, 6, Some(1)), (8, 2, Some(0)), (10, 0, Some(0))] {
            resolve_turn(&mut session);
            assert_eq!(held(&session, first, Resource::Metals), metals);
            assert_eq!(left(&session), (stock, turns_left), "at {metals} metals");
            exhausted += session
                .drain_events()
                .iter()
                .filter(|event| {
                    **event
                        == GameEvent::DepositExhausted {
                            nation: first,
                            deposit,
                        }
                })
                .count();
        }
        assert_eq!(exhausted, 1, "the turn it ran dry");
        assert_eq!(session.world().deposits().get(deposit).unwrap().level(), 0);

        resolve_turn(&mut session);
        assert_eq!(
            held(&session, first, Resource::Metals),
            10,
            "nothing left to draw"
        );
        assert!(
            session
                .drain_events()
                .iter()
                .all(|event| !matches!(event, GameEvent::DepositExhausted { .. }))
        );
    }

    #[test]
    fn test_a_recharging_deposit_yields_up_to_its_recharge_and_never_runs_out() {
        let (mut session, first, ..) = builders_session(0);
        session.world.register_building_template(pump_template());
        let mut spring = ResourceDataStorage::new();
        spring.replenish(Resource::Water, 0);
        spring.set_income(Resource::Water, 3);
        let deposit = session.world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_water"),
            at(1, 0),
            spring,
            10,
        ));
        construct(&mut session, PUMP, 1, 0).expect("on its spring");
        resolve_turn(&mut session);

        for water in [3, 6, 9] {
            resolve_turn(&mut session);
            assert_eq!(
                held(&session, first, Resource::Water),
                water,
                "3 of the 5 it is after"
            );
        }
        let spring = session.world().deposits().get(deposit).unwrap();
        assert_eq!(
            (spring.amount(), spring.turns_left()),
            (3, None),
            "as it was"
        );
    }

    /// What `viewer` is shown of a building: its state bits, turns left and total, and level.
    fn shown(session: &GameSession, viewer: NationId, id: BuildingId) -> (u8, u16, u16, u8) {
        let snapshot = session.snapshot(viewer);
        let at = snapshot
            .building_ids
            .iter()
            .position(|&shown| shown == id.raw())
            .expect("the viewer sees it");
        (
            snapshot.building_states[at],
            snapshot.building_turns_left[at],
            snapshot.building_turns_total[at],
            snapshot.building_levels[at],
        )
    }

    #[test]
    fn test_an_owner_is_shown_its_buildings_works_and_its_templates() {
        use crate::game::projection::building_states::{UNDER_CONSTRUCTION, UPGRADING};

        let (mut session, first, workshop) = workshop_session(100);
        let meta = session.snapshot(first).meta;
        let template = meta
            .building_templates
            .iter()
            .find(|template| template.r#type == WORKSHOP)
            .expect("in the meta");
        assert_eq!(template.id, WORKSHOP, "what a construct order names it by");
        assert_eq!(template.levels.len(), 2, "every level, for the panels");
        assert_eq!(template.levels[1].production, [(Resource::Food, 5)].into());
        assert_eq!(template.deposit, None);

        assert_eq!(
            shown(&session, first, workshop),
            (UNDER_CONSTRUCTION, 2, 2, 0)
        );
        resolve_turn(&mut session);
        assert_eq!(
            shown(&session, first, workshop),
            (UNDER_CONSTRUCTION, 1, 2, 0)
        );
        resolve_turn(&mut session);
        assert_eq!(shown(&session, first, workshop), (0, 0, 0, 1), "built");

        upgrade(&mut session, workshop).unwrap();
        assert_eq!(shown(&session, first, workshop), (UPGRADING, 1, 1, 1));
    }

    #[test]
    fn test_a_rival_sees_works_under_way_but_not_how_they_go_nor_a_halt() {
        use crate::game::projection::building_states::{HALTED, UNDER_CONSTRUCTION};

        let (mut session, first, workshop) = workshop_session(0);
        let second = NationId::from_index(1);
        // a scout of the other nation's, next to the works
        let mut changes = TurnChanges::new();
        WorldMut::new(&mut session.world, &mut changes).insert_unit(Unit::recruit(
            SimulationID::new_abstract_id(SCOUT),
            second,
            at(3, 1),
        ));

        resolve_turn(&mut session);
        assert_eq!(
            shown(&session, first, workshop),
            (UNDER_CONSTRUCTION, 1, 2, 0)
        );
        assert_eq!(
            shown(&session, second, workshop),
            (UNDER_CONSTRUCTION, 0, 0, 0),
            "the scaffolding, not the turns"
        );

        // built, then unpaid: its owner has no credits
        resolve_turn(&mut session);
        resolve_turn(&mut session);
        assert_eq!(shown(&session, first, workshop), (HALTED, 0, 0, 1));
        assert_eq!(
            shown(&session, second, workshop),
            (0, 0, 0, 1),
            "a rival's upkeep is its own"
        );
    }

    #[test]
    fn test_a_deposit_drawn_on_is_caught_up_with_in_the_delta() {
        let (mut session, first, ..) = builders_session(0);
        let mut stock = ResourceDataStorage::new();
        stock.replenish(Resource::Metals, 10);
        let deposit = session.world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_metals"),
            at(1, 0),
            stock,
            600,
        ));
        construct(&mut session, DRILL, 1, 0).unwrap();
        resolve_turn(&mut session);
        let version = session.version();

        resolve_turn(&mut session);

        let Projection::Delta(delta) = session.project(first, version) else {
            panic!("a turn must not cost a full snapshot");
        };
        let at = delta
            .deposits
            .ids
            .iter()
            .position(|&id| id == deposit.raw())
            .expect("the deposit it drew on");
        assert_eq!(delta.deposits.quantities[at], 6);
        assert_eq!(delta.deposits.turns_left[at], 1, "6 left, 4 a turn");
    }

    #[test]
    fn test_a_nation_view_knows_its_own_land_buildings_and_stockpile_and_deposits_it_saw() {
        let (mut session, first, second, ..) = builders_session(70);
        construct(&mut session, PLANT, 2, 1).unwrap();
        // a deposit far off in the other nation's corner, which the first has never explored
        let mut stock = ResourceDataStorage::new();
        stock.replenish(Resource::Metals, 50);
        let unseen = session.world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_metals"),
            at(8, 6),
            stock,
            600,
        ));

        let view = NationView::new(session.world(), first);
        let mut territory: Vec<CubeCoords> = view.territory().collect();
        territory.sort_by_key(|&coords| session.world().tile_index(coords));
        let mut ring: Vec<CubeCoords> = session
            .world()
            .tiles_within(at(1, 1), 1)
            .into_iter()
            .filter_map(|index| session.world().tile_coords(index))
            .collect();
        ring.sort_by_key(|&coords| session.world().tile_index(coords));
        assert_eq!(territory, ring, "the ring its settlement claimed");

        assert_eq!(view.own_buildings().count(), 1);
        assert_eq!(
            view.stockpile()
                .map(|held| held.quantity_of(Resource::Credits)),
            Some(30),
            "70, less the plant's 40"
        );
        let known: Vec<DepositId> = view.known_deposits().map(|(id, _)| id).collect();
        assert_eq!(known.len(), 3, "the three around its settlement");
        assert!(!known.contains(&unseen), "never explored");
        assert_eq!(
            view.building_templates().count(),
            3,
            "the rules, every nation's"
        );

        // and the other nation sees none of it
        let theirs = NationView::new(session.world(), second);
        assert_eq!(theirs.territory().count(), 0);
        assert_eq!(theirs.own_buildings().count(), 0);
        assert!(theirs.known_deposits().all(|(id, _)| id == unseen));
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
