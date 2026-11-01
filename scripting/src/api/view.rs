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
        fog::TileVisibility,
        ids::{EntityIndex, NationId, SettlementId, SimulationID, UnitId},
        movement::Reachable,
    },
};

use super::{ScriptResult, coords, entity_id};

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
        }))
    }
}

impl Knowledge {
    /// Row-major index of a tile, like `World::tile_index`: wrapped east-west, `None` past a pole.
    fn index(&self, coords: CubeCoords) -> Option<usize> {
        let offset = OffsetCoords::from(self.shape.wrap(coords)?);
        Some(offset.row() as usize * self.shape.columns() as usize + offset.col() as usize)
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
        );
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
