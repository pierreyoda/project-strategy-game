//! The default AI's economy: what it builds and raises, on the land it has, with what it holds.

use project_x_core::{
    game::{
        ai::ControllerSlot,
        events::GameEvent,
        journal::TurnChanges,
        rng::GameSeed,
        session::GameSession,
        state::{World, WorldMut},
        turn::{StepBudget, TurnProgress},
    },
    hex_map::{
        HexMap, HexMapStorage,
        coordinates::{CubeCoords, OffsetCoords},
        shape::RectangularHexMapShape,
        tile::HexMapTile,
    },
    simulation::{
        buildings::{BuildingLevel, BuildingTemplate},
        districts::{District, DistrictRules, DistrictSite, DistrictTerms},
        ids::{BuildingId, NationId, SimulationID, UnitId},
        military::{Unit, UnitTemplate},
        nations::Nation,
        resources::{Resource, ResourceDataStorage, ResourceDataStore, ResourceDeposit},
        settlements::Settlement,
        strength::Recovery,
    },
};
use project_x_scripting::{ScriptEngine, ScriptKind, ScriptLimits, ScriptSource};

const SEED: GameSeed = 20_261_103;
const DEFAULT_AI: &str = include_str!("../../scripts/ai/default.rhai");

fn at(col: i16, row: i16) -> CubeCoords {
    OffsetCoords::new(col, row).into()
}

/// Its settlement stands here; the deposit, when there is one, in the ring it owns around it.
const SETTLEMENT: (i16, i16) = (8, 6);
const DEPOSIT: (i16, i16) = (9, 6);

fn level(credits: u16, turns: u16, upkeep: u16, made: (Resource, u16)) -> BuildingLevel {
    BuildingLevel {
        cost: [(Resource::Credits, credits)].into(),
        turns,
        maintenance: [(Resource::Credits, upkeep)].into(),
        production: [made].into(),
        sprite: String::new(),
    }
}

struct Game {
    session: GameSession,
    ai: NationId,
    scout: UnitId,
}

/// Districts at 100 credits each, laid out in a turn.
#[derive(Debug)]
struct FlatDistricts;

impl DistrictRules for FlatDistricts {
    fn terms(&self, _: &DistrictSite) -> Result<DistrictTerms, String> {
        Ok(DistrictTerms {
            cost: [(Resource::Credits, 100)].into(),
            turns: 1,
        })
    }
}

/// `game_with`, the ring laid out as districts.
fn game(credits: u16, deposit: bool) -> Game {
    game_with(credits, deposit, true)
}

/// The player in the north-west; the scripted nation in the south-east, settled at `SETTLEMENT` with
/// the ring around it its own - and laid out as districts if asked - a scout, and `credits`; and a
/// metals deposit at `DEPOSIT` if asked.
///
/// A mine is built in 2 turns for 100 credits and raised to level 2 in 1 for 150, upkeep 1 then 2;
/// a plant is built in 1 for 300, upkeep 1; a district is laid out in 1 for 100.
fn game_with(credits: u16, deposit: bool, laid_out: bool) -> Game {
    let shape = RectangularHexMapShape::new(12, 10);
    let mut tiles = HexMapStorage::with_capacity(shape.tiles_count());
    for coords in shape.coords() {
        tiles.insert(coords, HexMapTile::from_properties(120));
    }
    let mut world = World::new(shape, HexMap::from_tiles(tiles));
    let scout = UnitTemplate::new(
        SimulationID::new_abstract_id("scout"),
        "scout".to_string(),
        2,
        1,
        4,
        Recovery::All,
    );
    world.register_building_template(BuildingTemplate::new(
        SimulationID::new_abstract_id("mine"),
        "mine".to_string(),
        Some(Resource::Metals),
        vec![
            level(100, 2, 1, (Resource::Metals, 4)),
            level(150, 1, 2, (Resource::Metals, 6)),
        ],
    ));
    world.register_building_template(BuildingTemplate::new(
        SimulationID::new_abstract_id("plant"),
        "plant".to_string(),
        None,
        vec![level(300, 1, 1, (Resource::Electricity, 2))],
    ));

    let player = world.register_nation(Nation::new(
        SimulationID::new_abstract_id("player"),
        "Player".to_string(),
    ));
    let ai = world.register_nation(Nation::new(
        SimulationID::new_abstract_id("scripted"),
        "Scripted".to_string(),
    ));
    world.place_unit(Unit::recruit(&scout, player, at(1, 1)));
    let scripted_scout = world.place_unit(Unit::recruit(&scout, ai, at(7, 7)));
    if deposit {
        let mut stock = ResourceDataStorage::new();
        stock.replenish(Resource::Metals, 500);
        world.place_deposit(ResourceDeposit::new(
            SimulationID::new_abstract_id("deposit_metals"),
            at(DEPOSIT.0, DEPOSIT.1),
            stock,
            600,
        ));
    }

    let mut changes = TurnChanges::new();
    let mut gate = WorldMut::new(&mut world, &mut changes);
    let settled = at(SETTLEMENT.0, SETTLEMENT.1);
    let outpost = gate.insert_settlement(Settlement::found("Outpost".to_string(), ai, settled));
    let ring: Vec<CubeCoords> = gate
        .world()
        .tiles_within(settled, 1)
        .into_iter()
        .filter_map(|index| gate.world().tile_coords(index))
        .collect();
    for coords in ring {
        let ground = gate.tile_mut(coords).unwrap().artificial_mut();
        ground.set_owner(Some(ai));
        if laid_out && coords != settled {
            ground.set_district(Some(District::laid_out(outpost)));
        }
    }
    gate.nation_mut(ai)
        .unwrap()
        .resources_mut()
        .replenish(Resource::Credits, credits);

    world.register_unit_template(scout);
    world.set_district_rules(Box::new(FlatDistricts));
    let mut session = GameSession::new(world, SEED, player);
    let engine = ScriptEngine::load_with(
        &[ScriptSource {
            name: "default_ai".to_string(),
            kind: ScriptKind::Ai,
            source: DEFAULT_AI.to_string(),
        }],
        ScriptLimits::default(),
    )
    .expect("the default AI loads");
    session.set_controller(
        ai,
        ControllerSlot::Ai(Box::new(engine.controller_for(ai, SEED).unwrap())),
    );
    session.drain_events();
    Game {
        session,
        ai,
        scout: scripted_scout,
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

/// What the scripted nation ordered in a turn: `("construct", building)` or `("upgrade", building)`.
fn ordered(ai: NationId, events: &[GameEvent]) -> Vec<(&'static str, BuildingId)> {
    events
        .iter()
        .filter_map(|event| match *event {
            GameEvent::ConstructionStarted {
                nation, building, ..
            } if nation == ai => Some(("construct", building)),
            GameEvent::UpgradeStarted {
                nation, building, ..
            } if nation == ai => Some(("upgrade", building)),
            _ => None,
        })
        .collect()
}

fn template_of(session: &GameSession, building: BuildingId) -> String {
    format!(
        "{}",
        session
            .world()
            .buildings()
            .get(building)
            .unwrap()
            .template()
    )
}

fn tile_of(session: &GameSession, building: BuildingId) -> CubeCoords {
    session
        .world()
        .buildings()
        .get(building)
        .unwrap()
        .position()
}

#[test]
fn test_it_builds_a_mine_on_its_deposit_once_it_can_keep_it_up() {
    // 100 for the mine, and 10 turns of its upkeep of 1 to keep in hand: 110
    let Game {
        mut session, ai, ..
    } = game(109, true);
    assert!(
        ordered(ai, &end_turn(&mut session)).is_empty(),
        "one credit short"
    );

    let Game {
        mut session, ai, ..
    } = game(110, true);
    let orders = ordered(ai, &end_turn(&mut session));
    let [("construct", mine)] = orders.as_slice() else {
        panic!("a construction, got {orders:?}");
    };
    assert_eq!(template_of(&session, *mine), "abstract:mine");
    assert_eq!(
        tile_of(&session, *mine),
        at(DEPOSIT.0, DEPOSIT.1),
        "on its deposit"
    );
}

#[test]
fn test_it_runs_one_works_at_a_time_raising_what_it_has_first() {
    let Game {
        mut session,
        ai,
        scout,
    } = game(10_000, true);
    let start = session.world().units().get(scout).unwrap().position();

    // turn 1: the mine is ordered, and its first turn of works passes
    let orders = ordered(ai, &end_turn(&mut session));
    let [("construct", mine)] = orders.as_slice() else {
        panic!("the mine first, got {orders:?}");
    };
    let mine = *mine;
    assert_eq!(template_of(&session, mine), "abstract:mine");
    assert_ne!(
        session.world().units().get(scout).unwrap().position(),
        start,
        "and its scout still scouts"
    );

    // turn 2: its works are under way, so nothing else starts; they complete
    assert!(ordered(ai, &end_turn(&mut session)).is_empty());
    assert_eq!(session.world().buildings().get(mine).unwrap().level(), 1);

    // turn 3: raising the mine comes before building anything new
    assert_eq!(ordered(ai, &end_turn(&mut session)), [("upgrade", mine)]);
    assert_eq!(session.world().buildings().get(mine).unwrap().level(), 2);

    // turn 4: the mine at its last level, a plant on free land - not the settlement's tile
    let orders = ordered(ai, &end_turn(&mut session));
    let [("construct", plant)] = orders.as_slice() else {
        panic!("a plant, got {orders:?}");
    };
    assert_eq!(template_of(&session, *plant), "abstract:plant");
    let site = tile_of(&session, *plant);
    assert!(site != at(SETTLEMENT.0, SETTLEMENT.1) && site != at(DEPOSIT.0, DEPOSIT.1));
}

#[test]
fn test_without_a_deposit_of_its_own_it_builds_what_needs_none() {
    let Game {
        mut session, ai, ..
    } = game(10_000, false);

    let orders = ordered(ai, &end_turn(&mut session));
    let [("construct", plant)] = orders.as_slice() else {
        panic!("a plant, got {orders:?}");
    };
    assert_eq!(template_of(&session, *plant), "abstract:plant");
    let site = tile_of(&session, *plant);
    let owner = session.world().tile(site).unwrap().artificial().owner();
    assert_eq!(owner, Some(ai), "on its own land");
    assert_ne!(site, at(SETTLEMENT.0, SETTLEMENT.1));
}

#[test]
fn test_it_sees_its_economy_as_its_nation_knows_it() {
    let script = r#"
        fn plan(view, orders, rng) {
            print(`territory ${view.territory.len()}`);
            print(`deposit ${view.deposit(axial(0, 0))}`);
            let deposit = view.deposit(offset(9, 6));
            print(`metals ${deposit.resource} ${deposit.level} ${deposit.stock}`);
            print(`credits ${view.stockpile.credits}`);
            let ids = [];
            for template in view.templates { ids.push(template.id); }
            print(`templates ${ids}`);
            let mine = view.templates[0];
            print(`mine ${mine.deposit} ${mine.levels[1].cost.credits} ${mine.levels[1].turns}`);
            print(`buildings ${view.buildings.len()}`);
            let outpost = view.settlements[0];
            let terms = outpost.district_terms;
            print(`districts ${outpost.districts.len()} ${outpost.districts[0].works} ${terms.cost.credits} ${terms.turns}`);
        }
    "#;
    let Game {
        mut session, ai, ..
    } = game(500, true);
    let engine = ScriptEngine::load(&[ScriptSource {
        name: "probe".to_string(),
        kind: ScriptKind::Ai,
        source: script.to_string(),
    }])
    .unwrap();
    session.set_controller(
        ai,
        ControllerSlot::Ai(Box::new(engine.controller_for(ai, SEED).unwrap())),
    );

    let said: Vec<String> = end_turn(&mut session)
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::ControllerLog { message, .. } => Some(message),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            "territory 7",
            // the player's corner: never explored, so no deposit it knows of
            "deposit ",
            "metals metals 3 500",
            "credits 500",
            "templates [\"mine\", \"plant\"]",
            "mine metals 150 1",
            "buildings 0",
            // its whole ring, all laid out; the next would cost 100 and take a turn
            "districts 6  100 1",
        ]
    );
}

#[test]
fn test_without_a_free_district_it_lays_one_out_then_builds_on_it() {
    let Game {
        mut session, ai, ..
    } = game_with(10_000, false, false);
    let outpost = session.world().settlements().ids().next().unwrap();

    // turn 1: nowhere to build its plant, so it lays a district out, which takes the turn
    let events = end_turn(&mut session);
    assert!(ordered(ai, &events).is_empty(), "nothing built yet");
    assert!(events.contains(&GameEvent::Log {
        nation: Some(ai),
        message: "Outpost lays out a district.".to_string(),
    }));
    let districts = session.world().districts_of(outpost);
    let [(site, district)] = districts.as_slice() else {
        panic!("one district, got {districts:?}");
    };
    assert!(district.is_laid_out());
    let site = *site;

    // turn 2: the plant goes on it
    let orders = ordered(ai, &end_turn(&mut session));
    let [("construct", plant)] = orders.as_slice() else {
        panic!("a plant, got {orders:?}");
    };
    assert_eq!(template_of(&session, *plant), "abstract:plant");
    assert_eq!(tile_of(&session, *plant), site);
}

#[test]
fn test_its_income_is_what_the_turn_does_to_its_stockpile_buildings_and_all() {
    // the default AI builds its mine on turn 1 out of 110 credits; it works from turn 3
    let Game {
        mut session, ai, ..
    } = game(110, true);
    end_turn(&mut session);
    end_turn(&mut session);
    let engine = ScriptEngine::load(&[ScriptSource {
        name: "probe".to_string(),
        kind: ScriptKind::Ai,
        source: r#"
            fn plan(view, orders, rng) {
                print(`income ${view.income.metals} ${view.income.credits} ${view.income.food}`);
                print(`stockpile ${view.stockpile.metals} ${view.stockpile.credits}`);
            }
        "#
        .to_string(),
    }])
    .unwrap();
    session.set_controller(
        ai,
        ControllerSlot::Ai(Box::new(engine.controller_for(ai, SEED).unwrap())),
    );

    let said: Vec<String> = end_turn(&mut session)
        .into_iter()
        .filter_map(|event| match event {
            GameEvent::ControllerLog { message, .. } => Some(message),
            _ => None,
        })
        .collect();
    assert_eq!(
        said,
        [
            // the mine's 4 metals and its credit of upkeep; nothing changes food, so it is not in
            "income 4 -1 ",
            // no metals held yet: the mine has not made any
            "stockpile  10",
        ]
    );
    let stockpile = session.world().nations().get(ai).unwrap().resources();
    assert_eq!(
        (
            stockpile.quantity_of(Resource::Metals),
            stockpile.quantity_of(Resource::Credits)
        ),
        (4, 9),
        "as foretold"
    );
}
