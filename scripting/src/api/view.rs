//! What a nation knows, as its script sees it.

use std::{collections::BTreeMap, rc::Rc};

use rhai::{Array, Dynamic, Engine, INT, ImmutableString, Map};

use project_x_core::{
    game::{ai::NationView, state::TileIndex},
    hex_map::{
        coordinates::{CubeCoords, OffsetCoords},
        shape::RectangularHexMapShape,
        terrain::TerrainKind,
        vegetation::Vegetation,
    },
    simulation::{
        buildings::BuildingLevel,
        fog::TileVisibility,
        ids::{BuildingId, DepositId, EntityIndex, NationId, SettlementId, SimulationID, UnitId},
        movement::Reachable,
        resources::{Resource, ResourceQuantity},
    },
};

use super::{ScriptResult, common::resource_name, coords, entity_id};

/// What one nation knows, taken at the start of its `plan`.
///
/// Owned rather than a borrow of `NationView`, because a value a script holds has to be `'static`;
/// and only ever built from a `NationView`, so it cannot know more than the nation does. A shared
/// handle, so that a script passing it to its own functions copies nothing.
#[derive(Clone, Debug)]
pub struct ScriptView(Rc<Knowledge>);

#[derive(Debug)]
struct Knowledge {
    turn: INT,
    nation: NationId,
    shape: RectangularHexMapShape,
    /// Per tile, row-major.
    visibility: Vec<TileVisibility>,
    /// Per tile, row-major; `None` where the nation has never looked.
    terrain: Vec<Option<TerrainKind>>,
    /// Per tile, row-major; `None` where the nation has never looked.
    vegetation: Vec<Option<Vegetation>>,
    /// Every unit of the nation: a controller plays a whole nation, for now. One playing only part
    /// of a human nation will be shown the units in its charge, and its orders for any other
    /// refused by the host.
    units: Vec<OwnUnit>,
    settlements: Vec<OwnSettlement>,
    foreign_units: Vec<ForeignUnit>,
    /// Where each unit that can still move can get to, by the rules its order will be judged by.
    ///
    /// Computed up front, since nothing is left to compute it from once the view is handed over;
    /// and only for the units that have somewhere to go.
    reachable: BTreeMap<UnitId, Reachable>,
    /// The tiles the nation owns.
    territory: Vec<CubeCoords>,
    /// The deposits it knows of, by tile index: the richer, on a tile that holds two.
    deposits: BTreeMap<usize, KnownDeposit>,
    /// Its own buildings, its works under way included.
    buildings: Vec<OwnBuilding>,
    /// The game's building templates: its rules, every nation's to know.
    templates: Vec<TemplateData>,
    /// What it holds, by resource.
    stockpile: BTreeMap<Resource, ResourceQuantity>,
    /// What each stockpile changes by next turn, buildings' production and upkeep included: the
    /// figure its player is shown. Only those that change.
    income: BTreeMap<Resource, i32>,
}

#[derive(Debug)]
struct KnownDeposit {
    id: DepositId,
    resource: Resource,
    level: u8,
    stock: ResourceQuantity,
    recharge: ResourceQuantity,
}

#[derive(Debug)]
struct OwnBuilding {
    id: BuildingId,
    tile: CubeCoords,
    template: ImmutableString,
    level: u8,
    halted: bool,
    /// Turns left, and in all.
    works: Option<(u16, u16)>,
}

#[derive(Debug)]
struct TemplateData {
    id: ImmutableString,
    kind: ImmutableString,
    deposit: Option<Resource>,
    levels: Vec<BuildingLevel>,
}

#[derive(Debug)]
struct OwnUnit {
    id: UnitId,
    tile: CubeCoords,
    template: ImmutableString,
    movement_left: u8,
    health: u16,
}

#[derive(Debug)]
struct OwnSettlement {
    id: SettlementId,
    tile: CubeCoords,
    name: ImmutableString,
}

#[derive(Debug)]
struct ForeignUnit {
    id: UnitId,
    tile: CubeCoords,
    owner: NationId,
}

impl ScriptView {
    pub fn new(view: &NationView<'_>) -> Self {
        let shape = view.shape();
        let (visibility, terrain) = shape
            .coords()
            .map(|coords| (view.visibility(coords), view.terrain(coords)))
            .unzip();
        let vegetation = shape
            .coords()
            .map(|coords| view.vegetation(coords))
            .collect();
        let units: Vec<OwnUnit> = view
            .own_units()
            .map(|(id, unit)| OwnUnit {
                id,
                tile: unit.position(),
                template: template_name(unit.template()),
                movement_left: unit.movement_left(),
                health: unit.health_points(),
            })
            .collect();
        let reachable = units
            .iter()
            .filter(|unit| unit.movement_left > 0)
            .filter_map(|unit| Some((unit.id, view.reachable(unit.id)?)))
            .collect();
        let settlements = view
            .own_settlements()
            .map(|(id, settlement)| OwnSettlement {
                id,
                tile: settlement.position(),
                name: settlement.name().into(),
            })
            .collect();
        let foreign_units = view
            .visible_foreign_units()
            .map(|(id, unit)| ForeignUnit {
                id,
                tile: unit.position(),
                owner: unit.owner(),
            })
            .collect();

        let mut deposits = BTreeMap::new();
        for (id, deposit) in view.known_deposits() {
            let (Some(index), Some(resource)) =
                (index_in(shape, deposit.position()), deposit.resource())
            else {
                continue;
            };
            let data = deposit.resources().get(&resource);
            let known = KnownDeposit {
                id,
                resource,
                level: deposit.level(),
                stock: data.map_or(0, |data| data.quantity()),
                recharge: data.and_then(|data| data.income()).unwrap_or(0),
            };
            let richer = deposits
                .get(&index)
                .is_none_or(|kept: &KnownDeposit| known.level > kept.level);
            if richer {
                deposits.insert(index, known);
            }
        }
        let buildings = view
            .own_buildings()
            .map(|(id, building)| OwnBuilding {
                id,
                tile: building.position(),
                template: template_name(building.template()),
                level: building.level(),
                halted: building.halted(),
                works: building
                    .works()
                    .map(|works| (works.turns_left(), works.turns_total())),
            })
            .collect();
        let templates = view
            .building_templates()
            .map(|(id, template)| TemplateData {
                id: template_name(id),
                kind: template.r#type().into(),
                deposit: template.deposit(),
                levels: template.levels().to_vec(),
            })
            .collect();
        let stockpile = view
            .stockpile()
            .map(|held| {
                held.iter()
                    .map(|(&resource, data)| (resource, data.quantity()))
                    .collect()
            })
            .unwrap_or_default();
        let income = view
            .forecast()
            .into_iter()
            .map(|(resource, forecast)| {
                let change = i32::from(forecast.gained) - i32::from(forecast.spent);
                (resource, change)
            })
            .filter(|&(_, change)| change != 0)
            .collect();

        Self(Rc::new(Knowledge {
            turn: INT::from(view.turn()),
            nation: view.nation(),
            shape,
            visibility,
            terrain,
            vegetation,
            units,
            settlements,
            foreign_units,
            reachable,
            territory: view.territory().collect(),
            deposits,
            buildings,
            templates,
            stockpile,
            income,
        }))
    }
}

/// Row-major index of a tile, like `World::tile_index`: wrapped east-west, `None` past a pole.
fn index_in(shape: RectangularHexMapShape, coords: CubeCoords) -> Option<usize> {
    let offset = OffsetCoords::from(shape.wrap(coords)?);
    Some(offset.row() as usize * shape.columns() as usize + offset.col() as usize)
}

impl Knowledge {
    fn index(&self, coords: CubeCoords) -> Option<usize> {
        index_in(self.shape, coords)
    }

    /// Coordinates of a tile from its row-major index, like `World::tile_coords`.
    fn coords(&self, index: TileIndex) -> CubeCoords {
        let columns = self.shape.columns() as u32;
        OffsetCoords::new((index % columns) as i16, (index / columns) as i16).into()
    }

    fn visibility(&self, coords: CubeCoords) -> TileVisibility {
        self.index(coords)
            .map_or(TileVisibility::Unexplored, |index| self.visibility[index])
    }

    fn terrain(&self, coords: CubeCoords) -> Option<TerrainKind> {
        self.index(coords).and_then(|index| self.terrain[index])
    }

    fn vegetation(&self, coords: CubeCoords) -> Option<Vegetation> {
        self.index(coords).and_then(|index| self.vegetation[index])
    }
}

pub fn register(engine: &mut Engine) {
    engine
        .register_type_with_name::<ScriptView>("View")
        .register_get("turn", |view: &mut ScriptView| view.0.turn)
        .register_get("nation", |view: &mut ScriptView| id(view.0.nation))
        .register_get("columns", |view: &mut ScriptView| {
            INT::from(view.0.shape.columns())
        })
        .register_get("rows", |view: &mut ScriptView| {
            INT::from(view.0.shape.rows())
        })
        .register_get("units", |view: &mut ScriptView| -> Array {
            view.0.units.iter().map(OwnUnit::to_dynamic).collect()
        })
        .register_get("settlements", |view: &mut ScriptView| -> Array {
            view.0
                .settlements
                .iter()
                .map(OwnSettlement::to_dynamic)
                .collect()
        })
        .register_get("foreign_units", |view: &mut ScriptView| -> Array {
            view.0
                .foreign_units
                .iter()
                .map(ForeignUnit::to_dynamic)
                .collect()
        })
        .register_fn(
            "visibility",
            |view: &mut ScriptView, at: Map| -> ScriptResult<ImmutableString> {
                let name = match view.0.visibility(coords::from_map(&at)?) {
                    TileVisibility::Unexplored => "unexplored",
                    TileVisibility::Fogged => "fogged",
                    TileVisibility::Visible => "visible",
                };
                Ok(name.into())
            },
        )
        .register_fn(
            "terrain",
            |view: &mut ScriptView, at: Map| -> ScriptResult<ImmutableString> {
                Ok(terrain_name(view.0.terrain(coords::from_map(&at)?)).into())
            },
        )
        .register_fn(
            "vegetation",
            |view: &mut ScriptView, at: Map| -> ScriptResult<ImmutableString> {
                Ok(vegetation_name(view.0.vegetation(coords::from_map(&at)?)).into())
            },
        )
        .register_fn(
            "neighbors",
            |view: &mut ScriptView, at: Map| -> ScriptResult<Array> {
                let at = coords::from_map(&at)?;
                Ok(view.0.shape.neighbors(at).map(tile).collect())
            },
        )
        .register_fn(
            "distance",
            |view: &mut ScriptView, from: Map, to: Map| -> ScriptResult<INT> {
                let (from, to) = (coords::from_map(&from)?, coords::from_map(&to)?);
                Ok(INT::from(view.0.shape.distance(from, to)))
            },
        )
        .register_fn(
            "reachable",
            |view: &mut ScriptView, unit: INT| -> ScriptResult<Array> {
                let unit = entity_id::<UnitId>(unit, "unit")?;
                let Some(reachable) = view.0.reachable.get(&unit) else {
                    // not this nation's unit, or one that cannot move this turn
                    return Ok(Array::new());
                };
                Ok(reachable
                    .tiles
                    .iter()
                    .zip(&reachable.costs)
                    .map(|(&index, &cost)| {
                        let mut spot = Map::new();
                        spot.insert("tile".into(), tile(view.0.coords(index)));
                        spot.insert("cost".into(), INT::from(cost).into());
                        Dynamic::from_map(spot)
                    })
                    .collect())
            },
        )
        .register_fn(
            "path_to",
            // the path a move order takes, the unit's own tile left out; `()` when out of reach
            |view: &mut ScriptView, unit: INT, to: Map| -> ScriptResult<Dynamic> {
                let unit = entity_id::<UnitId>(unit, "unit")?;
                let to = coords::from_map(&to)?;
                let path = view
                    .0
                    .reachable
                    .get(&unit)
                    .and_then(|reachable| reachable.path_to(view.0.index(to)? as TileIndex));
                Ok(path.map_or(Dynamic::UNIT, |path| {
                    Dynamic::from_array(
                        path.into_iter()
                            .map(|index| tile(view.0.coords(index)))
                            .collect(),
                    )
                }))
            },
        )
        // the economy: what the nation owns, holds and may build
        .register_get("territory", |view: &mut ScriptView| -> Array {
            view.0.territory.iter().copied().map(tile).collect()
        })
        .register_fn(
            "deposit",
            // the deposit on a tile it has explored, `()` on any other
            |view: &mut ScriptView, at: Map| -> ScriptResult<Dynamic> {
                let at = coords::from_map(&at)?;
                Ok(view
                    .0
                    .index(at)
                    .and_then(|index| view.0.deposits.get(&index))
                    .map_or(Dynamic::UNIT, KnownDeposit::to_dynamic))
            },
        )
        .register_get("buildings", |view: &mut ScriptView| -> Array {
            view.0
                .buildings
                .iter()
                .map(OwnBuilding::to_dynamic)
                .collect()
        })
        .register_get("templates", |view: &mut ScriptView| -> Array {
            view.0
                .templates
                .iter()
                .map(TemplateData::to_dynamic)
                .collect()
        })
        .register_get("stockpile", |view: &mut ScriptView| -> Map {
            view.0
                .stockpile
                .iter()
                .map(|(&resource, &quantity)| {
                    (resource_name(resource).into(), INT::from(quantity).into())
                })
                .collect()
        })
        .register_get("income", |view: &mut ScriptView| -> Map {
            view.0
                .income
                .iter()
                .map(|(&resource, &change)| {
                    (resource_name(resource).into(), INT::from(change).into())
                })
                .collect()
        });
}

impl KnownDeposit {
    fn to_dynamic(&self) -> Dynamic {
        let mut deposit = Map::new();
        deposit.insert("id".into(), id(self.id).into());
        deposit.insert("resource".into(), resource_name(self.resource).into());
        deposit.insert("level".into(), INT::from(self.level).into());
        deposit.insert("stock".into(), INT::from(self.stock).into());
        deposit.insert("recharge".into(), INT::from(self.recharge).into());
        Dynamic::from_map(deposit)
    }
}

impl OwnBuilding {
    fn to_dynamic(&self) -> Dynamic {
        let mut building = Map::new();
        building.insert("id".into(), id(self.id).into());
        building.insert("tile".into(), tile(self.tile));
        building.insert("template".into(), self.template.clone().into());
        building.insert("level".into(), INT::from(self.level).into());
        building.insert("halted".into(), self.halted.into());
        let works = self
            .works
            .map_or(Dynamic::UNIT, |(turns_left, turns_total)| {
                let mut works = Map::new();
                works.insert("turns_left".into(), INT::from(turns_left).into());
                works.insert("turns_total".into(), INT::from(turns_total).into());
                Dynamic::from_map(works)
            });
        building.insert("works".into(), works);
        Dynamic::from_map(building)
    }
}

impl TemplateData {
    fn to_dynamic(&self) -> Dynamic {
        let mut template = Map::new();
        template.insert("id".into(), self.id.clone().into());
        template.insert("type".into(), self.kind.clone().into());
        template.insert(
            "deposit".into(),
            self.deposit
                .map_or(Dynamic::UNIT, |resource| resource_name(resource).into()),
        );
        let levels: Array = self
            .levels
            .iter()
            .map(|level| {
                let mut figures = Map::new();
                figures.insert("cost".into(), amounts(&level.cost));
                figures.insert("turns".into(), INT::from(level.turns).into());
                figures.insert("maintenance".into(), amounts(&level.maintenance));
                figures.insert("production".into(), amounts(&level.production));
                Dynamic::from_map(figures)
            })
            .collect();
        template.insert("levels".into(), Dynamic::from_array(levels));
        Dynamic::from_map(template)
    }
}

/// Quantities by resource, as scripts read them: `#{credits: 200, metals: 100}`.
fn amounts(figures: &BTreeMap<Resource, ResourceQuantity>) -> Dynamic {
    Dynamic::from_map(
        figures
            .iter()
            .map(|(&resource, &amount)| (resource_name(resource).into(), INT::from(amount).into()))
            .collect(),
    )
}

impl OwnUnit {
    fn to_dynamic(&self) -> Dynamic {
        let mut unit = Map::new();
        unit.insert("id".into(), id(self.id).into());
        unit.insert("tile".into(), tile(self.tile));
        unit.insert("template".into(), self.template.clone().into());
        unit.insert("movement_left".into(), INT::from(self.movement_left).into());
        unit.insert("health".into(), INT::from(self.health).into());
        Dynamic::from_map(unit)
    }
}

impl OwnSettlement {
    fn to_dynamic(&self) -> Dynamic {
        let mut settlement = Map::new();
        settlement.insert("id".into(), id(self.id).into());
        settlement.insert("tile".into(), tile(self.tile));
        settlement.insert("name".into(), self.name.clone().into());
        Dynamic::from_map(settlement)
    }
}

impl ForeignUnit {
    fn to_dynamic(&self) -> Dynamic {
        let mut unit = Map::new();
        unit.insert("id".into(), id(self.id).into());
        unit.insert("tile".into(), tile(self.tile));
        unit.insert("owner".into(), id(self.owner).into());
        Dynamic::from_map(unit)
    }
}

fn id(id: impl EntityIndex) -> INT {
    id.index() as INT
}

fn tile(coords: CubeCoords) -> Dynamic {
    Dynamic::from_map(coords::to_map(coords))
}

fn template_name(template: &SimulationID) -> ImmutableString {
    match template {
        SimulationID::Abstract(name) | SimulationID::Property(name) => name.as_str().into(),
    }
}

fn vegetation_name(vegetation: Option<Vegetation>) -> &'static str {
    match vegetation {
        None => "unknown",
        Some(Vegetation::None) => "none",
        Some(Vegetation::Forest) => "forest",
    }
}

fn terrain_name(terrain: Option<TerrainKind>) -> &'static str {
    match terrain {
        None => "unknown",
        Some(TerrainKind::DeepWater) => "deep_water",
        Some(TerrainKind::ShallowWater) => "shallow_water",
        Some(TerrainKind::Flat) => "flat",
        Some(TerrainKind::Hills) => "hills",
        Some(TerrainKind::Mountains) => "mountains",
        Some(TerrainKind::Ice) => "ice",
        Some(TerrainKind::RoughIce) => "rough_ice",
    }
}
