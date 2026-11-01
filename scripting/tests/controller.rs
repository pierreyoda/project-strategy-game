//! An AI script playing a nation, through the very turn loop a game runs.

use project_x_core::{
    game::{
        ai::ControllerSlot,
        commands::GameCommand,
        events::GameEvent,
        rng::GameSeed,
        session::GameSession,
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
        ids::{NationId, SimulationID, UnitId},
        military::{Unit, UnitTemplate},
        nations::Nation,
    },
};
use project_x_scripting::{ScriptEngine, ScriptError, ScriptKind, ScriptLimits, ScriptSource};

const SEED: GameSeed = 20_261_031;
const SCOUT_SCRIPT: &str = include_str!("../../scripts/ai/scout.rhai");
const SCRIPT_NAME: &str = "under_test";

fn at(col: i16, row: i16) -> CubeCoords {
    OffsetCoords::new(col, row).into()
}

/// A flat world: the player's scout in the north-west, the scripted nation's two in the south-east,
/// out of sight of one another.
struct Game {
    session: GameSession,
    ai: NationId,
    player_scout: UnitId,
    scouts: [UnitId; 2],
}

fn game(script: &str, limits: ScriptLimits) -> Game {
    let shape = RectangularHexMapShape::new(12, 10);
    let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
    for coords in shape.coords() {
        tiles.insert(coords, HexMapTile::from_properties(120));
    }
    let mut world = World::new(shape, HexMap::from_tiles(tiles));
    let scout = SimulationID::new_abstract_id("scout");
    world.register_unit_template(UnitTemplate::new(scout.clone(), "scout".to_string(), 2, 1));

    let player = world.register_nation(Nation::new(
        SimulationID::new_abstract_id("player"),
        "Player".to_string(),
    ));
    let ai = world.register_nation(Nation::new(
        SimulationID::new_abstract_id("scripted"),
        "Scripted".to_string(),
    ));
    let player_scout = world.place_unit(Unit::recruit(scout.clone(), player, at(1, 1)));
    let scouts = [(8, 6), (9, 7)]
        .map(|(col, row)| world.place_unit(Unit::recruit(scout.clone(), ai, at(col, row))));

    let mut session = GameSession::new(world, SEED, player);
    let engine = ScriptEngine::load_with(
        &[ScriptSource {
            name: SCRIPT_NAME.to_string(),
            kind: ScriptKind::Ai,
            source: script.to_string(),
        }],
        limits,
    )
    .expect("the script loads");
    let controller = engine
        .controller_for(ai, SEED)
        .expect("an AI script was given");
    session.set_controller(ai, ControllerSlot::Ai(Box::new(controller)));
    session.drain_events();

    Game {
        session,
        ai,
        player_scout,
        scouts,
    }
}

/// Resolve a whole turn, and hand back what happened during it.
fn end_turn(session: &mut GameSession) -> Vec<GameEvent> {
    session.begin_turn();
    for _ in 0..256 {
        if let TurnProgress::Done { .. } = session.step(StepBudget::default()) {
            return session.drain_events();
        }
    }
    panic!("the turn never finished");
}

fn position(session: &GameSession, unit: UnitId) -> CubeCoords {
    session.world().units().get(unit).unwrap().position()
}

/// What the scripted nation's controller logged, in order.
fn logged_by(nation: NationId, events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ControllerLog {
                nation: by,
                controller,
                message,
            } if *by == nation => {
                assert_eq!(controller, SCRIPT_NAME, "named after its script");
                Some(message.clone())
            }
            _ => None,
        })
        .collect()
}

fn rejections(events: &[GameEvent]) -> Vec<&GameEvent> {
    events
        .iter()
        .filter(|event| matches!(event, GameEvent::CommandRejected { .. }))
        .collect()
}

#[test]
fn test_the_scout_script_sends_every_scout_somewhere_new() {
    let Game {
        mut session,
        ai,
        player_scout,
        scouts,
    } = game(SCOUT_SCRIPT, ScriptLimits::default());
    let before = scouts.map(|scout| position(&session, scout));
    let player_before = position(&session, player_scout);

    for turn in 0..3 {
        let events = end_turn(&mut session);
        assert!(rejections(&events).is_empty(), "turn {turn}: {events:?}");
        assert!(logged_by(ai, &events).is_empty(), "turn {turn}: {events:?}");
    }

    for (scout, from) in scouts.iter().zip(before) {
        assert_ne!(position(&session, *scout), from, "{scout:?} never left");
    }
    assert_eq!(
        position(&session, player_scout),
        player_before,
        "not the script's unit"
    );
    // the orders went through the player's path, log included
    let ordered = session
        .command_log()
        .iter()
        .filter(|logged| logged.nation == ai)
        .filter(|logged| matches!(logged.command, GameCommand::MoveUnit { .. }))
        .count();
    assert_eq!(ordered, 6, "two scouts, three turns");
}

#[test]
fn test_a_script_that_never_ends_is_stopped_and_its_earlier_orders_stand() {
    let script = r#"
        fn plan(view, orders, rng) {
            let scout = view.units[0];
            orders.move_unit(scout.id, view.path_to(scout.id, view.neighbors(scout.tile)[0]));
            loop {}
        }
    "#;
    let limits = ScriptLimits {
        runaway_operations: 100_000,
    };
    let Game {
        mut session,
        ai,
        scouts,
        ..
    } = game(script, limits);
    let from = position(&session, scouts[0]);

    let events = end_turn(&mut session);

    assert_eq!(
        logged_by(ai, &events),
        [ScriptError::Runaway {
            script: SCRIPT_NAME.to_string(),
            operations: 100_000,
        }
        .to_string()]
    );
    assert_ne!(
        position(&session, scouts[0]),
        from,
        "the order before the loop went through"
    );
}

#[test]
fn test_a_failing_script_is_logged_after_what_it_printed() {
    let script = r#"
        fn plan(view, orders, rng) {
            print("planning");
            let scout = view.units[1];
            orders.move_unit(scout.id, view.path_to(scout.id, view.neighbors(scout.tile)[0]));
            throw "out of ideas";
        }
    "#;
    let Game {
        mut session,
        ai,
        scouts,
        ..
    } = game(script, ScriptLimits::default());
    let from = position(&session, scouts[1]);

    let events = end_turn(&mut session);

    let log = logged_by(ai, &events);
    assert_eq!(log.len(), 2, "{log:?}");
    assert_eq!(log[0], "planning");
    assert!(
        log[1].starts_with(&format!("script `{SCRIPT_NAME}` failed: "))
            && log[1].contains("out of ideas"),
        "{}",
        log[1]
    );
    assert_ne!(
        position(&session, scouts[1]),
        from,
        "the order before the error went through"
    );
    // and the next turn runs the script again, from scratch
    assert_eq!(logged_by(ai, &end_turn(&mut session)).len(), 2);
}

#[test]
fn test_a_script_cannot_see_through_the_fog() {
    let script = r#"
        fn plan(view, orders, rng) {
            let own = view.units[0].tile;
            print(`${view.terrain(own)} ${view.visibility(own)} ${view.vegetation(own)}`);
            // where the player's scout stands, far out of sight
            let far = offset(1, 1);
            print(`${view.terrain(far)} ${view.visibility(far)} ${view.vegetation(far)}`);
            print(`${view.foreign_units.len()} foreign units`);
            // and beyond the pole, which is no error, only nothing to know
            print(`${view.terrain(offset(3, -1))} ${view.visibility(offset(3, -1))}`);
        }
    "#;
    let Game {
        mut session, ai, ..
    } = game(script, ScriptLimits::default());

    let events = end_turn(&mut session);

    assert_eq!(
        logged_by(ai, &events),
        [
            "flat visible none",
            "unknown unexplored unknown",
            "0 foreign units",
            "unknown unexplored"
        ]
    );
}

#[test]
fn test_numbers_a_script_makes_up_are_errors_not_panics() {
    // each of these reaches a `core` constructor that asserts on what it is given, unless checked
    for (call, expected) in [
        ("orders.move_unit(-1, [])", "-1 is not a unit ID"),
        (
            "orders.move_unit(99999999999, [])",
            "99999999999 is not a unit ID",
        ),
        (
            "orders.construct_building(-4, \"farm\")",
            "-4 is not a settlement ID",
        ),
        (
            "orders.move_unit(0, [#{ q: \"east\", r: 0 }])",
            "coordinates need an integer `q`",
        ),
        ("orders.move_unit(0, [42])", "a path is made of coordinates"),
        (
            "view.terrain(axial(20000, 20000))",
            "20000 is out of bounds",
        ),
        (
            "view.path_to(0, #{ r: 3 })",
            "coordinates need an integer `q`",
        ),
        ("rng.range(5, 5)", "cannot draw from the empty range 5..5"),
    ] {
        let script = format!("fn plan(view, orders, rng) {{ {call}; }}");
        let Game {
            mut session, ai, ..
        } = game(&script, ScriptLimits::default());

        let log = logged_by(ai, &end_turn(&mut session));

        assert_eq!(log.len(), 1, "{call}: {log:?}");
        assert!(log[0].contains(expected), "{call}: {}", log[0]);
    }
}

#[test]
fn test_a_script_draws_from_its_own_nation_stream() {
    let script = r#"
        fn plan(view, orders, rng) {
            print(`${rng.range(0, 1000000)} ${rng.range(0, 1000000)}`);
        }
    "#;
    let draws = || {
        let Game {
            mut session, ai, ..
        } = game(script, ScriptLimits::default());
        let first = logged_by(ai, &end_turn(&mut session));
        let second = logged_by(ai, &end_turn(&mut session));
        (first, second)
    };

    let (first, second) = draws();
    assert_eq!(
        draws(),
        (first.clone(), second.clone()),
        "same seed, same numbers"
    );
    assert_ne!(
        first, second,
        "the stream carries on from one turn to the next"
    );
}
