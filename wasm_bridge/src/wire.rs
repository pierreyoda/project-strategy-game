//! The wire format between the WASM bridge and the client.
//!
//! It lives here, not in `core`: the schema the TypeScript side reads is a boundary concern, and
//! keeping it out of the simulation lets `core` stay free of dependencies and of any notion that a
//! client exists at all. Everything in this module is a DTO, never a simulation type.
//!
//! Commands and events travel as JSON because they are low-frequency and shape-heavy, so typed
//! bindings would cost more boilerplate than they save. The map does **not**: a million tiles
//! through `JSON.parse` is a non-starter, which is why the projections travel as typed arrays.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use project_x_core::{
    game::{
        commands::{CommandRejection, GameCommand},
        events::GameEvent,
        projection::{BuildingTemplateMeta, NationMeta, ResourceMeta, SnapshotMeta},
    },
    hex_map::coordinates::{CubeCoords, OffsetCoords},
    simulation::{
        ids::{BuildingId, EntityIndex, SimulationID, UnitId},
        resources::Resource,
    },
};
use project_x_procgen::nations::GenerationReport;
use project_x_scripting::{ScriptKind, ScriptSource};

/// Offset coordinates, the way the client thinks of the map.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct OffsetCoordsWire {
    pub col: i16,
    pub row: i16,
}

impl From<OffsetCoordsWire> for CubeCoords {
    fn from(value: OffsetCoordsWire) -> Self {
        OffsetCoords::new(value.col, value.row).into()
    }
}

/// How a new game starts.
///
/// TODO: scenarios and nation rosters belong to scripts; this is the scaffold's stand-in.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGameSettings {
    pub seed: u32,
    pub columns: i16,
    pub rows: i16,
    /// How many nations play, the local player included. At least one.
    #[serde(default = "default_nations")]
    pub nations: u16,
    /// Share of the map covered by the seas, from 0.0 to 1.0.
    #[serde(default = "default_ocean_coverage")]
    pub ocean_coverage: f64,
    /// The scripts the game runs: one scenario, which a game cannot start without, and at most one
    /// AI script. For now the AI script plays every nation but the local player's - the stub does
    /// without one - and nothing yet hands part of the player's nation to a script.
    #[serde(default)]
    pub scripts: Vec<ScriptSourceWire>,
}

/// A script, sent by the client: the bridge compiles what it is handed and embeds nothing.
#[derive(Clone, Debug, Deserialize)]
pub struct ScriptSourceWire {
    pub name: String,
    pub kind: ScriptKindWire,
    pub source: String,
}

/// Like `ScriptKind` in the `scripting` crate.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScriptKindWire {
    Ai,
    Scenario,
}

impl From<ScriptSourceWire> for ScriptSource {
    fn from(value: ScriptSourceWire) -> Self {
        Self {
            name: value.name,
            kind: match value.kind {
                ScriptKindWire::Ai => ScriptKind::Ai,
                ScriptKindWire::Scenario => ScriptKind::Scenario,
            },
            source: value.source,
        }
    }
}

fn default_nations() -> u16 {
    2
}

/// Like `ocean_coverage` in `MapGeneratorSettings::default`.
fn default_ocean_coverage() -> f64 {
    0.575
}

/// How the siting of the nations went, like `GenerationReport` in the `procgen` crate.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NationsReportWire {
    pub spacing_requested: u32,
    pub spacing_final: u32,
    pub relaxations: u32,
    pub unplaced: u16,
    pub unclaimed_land: u32,
}

impl From<&GenerationReport> for NationsReportWire {
    fn from(report: &GenerationReport) -> Self {
        Self {
            spacing_requested: report.siting.spacing_requested,
            spacing_final: report.siting.spacing_final,
            relaxations: report.siting.relaxations,
            unplaced: report.siting.unplaced,
            // a map has at most 1024 * 1024 tiles
            unclaimed_land: report.unclaimed_land as u32,
        }
    }
}

/// A command, as the client sends it.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CommandWire {
    #[serde(rename_all = "camelCase")]
    MoveUnit {
        unit: u32,
        path: Vec<OffsetCoordsWire>,
    },
    #[serde(rename_all = "camelCase")]
    FoundSettlement { founder: u32, name: String },
    #[serde(rename_all = "camelCase")]
    ConstructBuilding {
        template: String,
        tile: OffsetCoordsWire,
    },
    #[serde(rename_all = "camelCase")]
    UpgradeBuilding { building: u32 },
}

impl From<CommandWire> for GameCommand {
    fn from(value: CommandWire) -> Self {
        match value {
            CommandWire::MoveUnit { unit, path } => Self::MoveUnit {
                unit: UnitId::from_index(unit as usize),
                path: path.into_iter().map(CubeCoords::from).collect(),
            },
            CommandWire::FoundSettlement { founder, name } => Self::FoundSettlement {
                founder: UnitId::from_index(founder as usize),
                name,
            },
            CommandWire::ConstructBuilding { template, tile } => Self::ConstructBuilding {
                template: SimulationID::new_abstract_id(&template),
                tile: tile.into(),
            },
            CommandWire::UpgradeBuilding { building } => Self::UpgradeBuilding {
                building: BuildingId::from_index(building as usize),
            },
        }
    }
}

/// Why a command was refused.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectionWire {
    /// Machine-readable discriminator, for a client that wants to react to one in particular.
    pub kind: &'static str,
    /// Human-readable, straight from `CommandRejection`'s `Display`.
    pub message: String,
}

impl From<&CommandRejection> for RejectionWire {
    fn from(value: &CommandRejection) -> Self {
        let kind = match value {
            CommandRejection::NotYourTurn { .. } => "notYourTurn",
            CommandRejection::UnknownEntity(_) => "unknownEntity",
            CommandRejection::UnknownDefinition(_) => "unknownDefinition",
            CommandRejection::NotYours(_) => "notYours",
            CommandRejection::IllegalMove { .. } => "illegalMove",
            CommandRejection::TileOccupied => "tileOccupied",
            CommandRejection::InsufficientResources { .. } => "insufficientResources",
            CommandRejection::IllegalPlacement { .. } => "illegalPlacement",
            CommandRejection::MissingDeposit { .. } => "missingDeposit",
            CommandRejection::NoSettlement => "noSettlement",
            CommandRejection::AtMaxLevel => "atMaxLevel",
            CommandRejection::WorksUnderWay => "worksUnderWay",
            CommandRejection::NotImplemented(_) => "notImplemented",
        };
        Self {
            kind,
            message: value.to_string(),
        }
    }
}

/// An event, as the client reads it.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum EventWire {
    #[serde(rename_all = "camelCase")]
    TurnStarted { turn: u32 },
    #[serde(rename_all = "camelCase")]
    Log {
        nation: Option<u16>,
        message: String,
    },
    #[serde(rename_all = "camelCase")]
    UnitMoved { unit: u32, from: u32, to: u32 },
    #[serde(rename_all = "camelCase")]
    SettlementFounded {
        settlement: u32,
        nation: u16,
        tile: u32,
    },
    #[serde(rename_all = "camelCase")]
    ConstructionStarted {
        nation: u16,
        building: u32,
        settlement: u32,
    },
    #[serde(rename_all = "camelCase")]
    UpgradeStarted {
        nation: u16,
        building: u32,
        level: u8,
    },
    #[serde(rename_all = "camelCase")]
    BuildingCompleted {
        nation: u16,
        building: u32,
        level: u8,
    },
    #[serde(rename_all = "camelCase")]
    BuildingHalted { nation: u16, building: u32 },
    #[serde(rename_all = "camelCase")]
    DepositExhausted { nation: u16, deposit: u32 },
    #[serde(rename_all = "camelCase")]
    ResourceShortage { nation: u16, resource: &'static str },
    #[serde(rename_all = "camelCase")]
    CommandRejected { nation: u16, reason: String },
    #[serde(rename_all = "camelCase")]
    ControllerLog {
        nation: u16,
        controller: String,
        message: String,
    },
}

impl From<&GameEvent> for EventWire {
    fn from(value: &GameEvent) -> Self {
        match value {
            GameEvent::TurnStarted { turn } => Self::TurnStarted { turn: *turn },
            GameEvent::Log { nation, message } => Self::Log {
                nation: nation.map(|nation| nation.raw()),
                message: message.clone(),
            },
            GameEvent::UnitMoved { unit, from, to } => Self::UnitMoved {
                unit: unit.raw(),
                from: *from,
                to: *to,
            },
            GameEvent::SettlementFounded {
                settlement,
                nation,
                tile,
            } => Self::SettlementFounded {
                settlement: settlement.raw(),
                nation: nation.raw(),
                tile: *tile,
            },
            GameEvent::ConstructionStarted {
                nation,
                building,
                settlement,
            } => Self::ConstructionStarted {
                nation: nation.raw(),
                building: building.raw(),
                settlement: settlement.raw(),
            },
            GameEvent::UpgradeStarted {
                nation,
                building,
                level,
            } => Self::UpgradeStarted {
                nation: nation.raw(),
                building: building.raw(),
                level: *level,
            },
            GameEvent::BuildingCompleted {
                nation,
                building,
                level,
            } => Self::BuildingCompleted {
                nation: nation.raw(),
                building: building.raw(),
                level: *level,
            },
            GameEvent::BuildingHalted { nation, building } => Self::BuildingHalted {
                nation: nation.raw(),
                building: building.raw(),
            },
            GameEvent::DepositExhausted { nation, deposit } => Self::DepositExhausted {
                nation: nation.raw(),
                deposit: deposit.raw(),
            },
            GameEvent::ResourceShortage { nation, resource } => Self::ResourceShortage {
                nation: nation.raw(),
                resource: resource_name(*resource),
            },
            GameEvent::CommandRejected { nation, reason } => Self::CommandRejected {
                nation: nation.raw(),
                reason: reason.clone(),
            },
            GameEvent::ControllerLog {
                nation,
                controller,
                message,
            } => Self::ControllerLog {
                nation: nation.raw(),
                controller: controller.clone(),
                message: message.clone(),
            },
        }
    }
}

/// The small, irregular half of a projection: everything that is not a buffer.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaWire {
    /// Exact below 2^53, which no playthrough will reach: a `BigInt` here would buy nothing and
    /// cost every comparison on the client side.
    pub version: f64,
    pub turn: u32,
    pub viewer: u16,
    pub active_nation: u16,
    pub nations: Vec<NationWire>,
    /// What the `buildingKinds` buffers index into.
    pub building_templates: Vec<BuildingTemplateWire>,
    /// What the `unitKinds` buffers index into.
    pub unit_templates: Vec<UnitTemplateWire>,
}

/// A building template, like `projection::BuildingTemplateMeta` in the `core` crate.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingTemplateWire {
    /// What `constructBuilding` names it by.
    pub id: String,
    pub r#type: String,
    /// The resource it extracts from the deposit it stands on, if any.
    pub deposit: Option<&'static str>,
    pub levels: Vec<BuildingLevelWire>,
}

/// A level of a building template, like `buildings::BuildingLevel` in the `core` crate.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevelWire {
    /// What the client draws the level with: a key into its art.
    pub sprite: String,
    /// What reaching the level costs: the construction for the first, an upgrade above.
    pub cost: BTreeMap<&'static str, u16>,
    pub turns: u16,
    /// Per turn, while it works at this level.
    pub maintenance: BTreeMap<&'static str, u16>,
    pub production: BTreeMap<&'static str, u16>,
}

impl From<&BuildingTemplateMeta> for BuildingTemplateWire {
    fn from(value: &BuildingTemplateMeta) -> Self {
        // resources by the names the client knows them by, as on the rest of the wire
        let named = |figures: &BTreeMap<Resource, u16>| {
            figures
                .iter()
                .map(|(&resource, &amount)| (resource_name(resource), amount))
                .collect()
        };
        Self {
            id: value.id.clone(),
            r#type: value.r#type.clone(),
            deposit: value.deposit.map(resource_name),
            levels: value
                .levels
                .iter()
                .map(|level| BuildingLevelWire {
                    sprite: level.sprite.clone(),
                    cost: named(&level.cost),
                    turns: level.turns,
                    maintenance: named(&level.maintenance),
                    production: named(&level.production),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitTemplateWire {
    pub r#type: String,
    pub movement: u8,
    pub sight: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NationWire {
    pub id: u16,
    pub name: String,
    pub resources: Vec<ResourceWire>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceWire {
    pub resource: &'static str,
    pub quantity: u16,
    /// Gained next turn, buildings' production included.
    pub income: u16,
    /// Lost next turn, buildings' maintenance included.
    pub depletion: u16,
}

impl From<&SnapshotMeta> for MetaWire {
    fn from(value: &SnapshotMeta) -> Self {
        Self {
            version: value.version as f64,
            turn: value.turn,
            viewer: value.viewer,
            active_nation: value.active_nation,
            nations: value.nations.iter().map(NationWire::from).collect(),
            building_templates: value
                .building_templates
                .iter()
                .map(BuildingTemplateWire::from)
                .collect(),
            unit_templates: value
                .unit_templates
                .iter()
                .map(|template| UnitTemplateWire {
                    r#type: template.r#type.clone(),
                    movement: template.movement,
                    sight: template.sight,
                })
                .collect(),
        }
    }
}

impl From<&NationMeta> for NationWire {
    fn from(value: &NationMeta) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            resources: value.resources.iter().map(ResourceWire::from).collect(),
        }
    }
}

impl From<&ResourceMeta> for ResourceWire {
    fn from(value: &ResourceMeta) -> Self {
        Self {
            resource: resource_name(value.resource),
            quantity: value.quantity,
            income: value.income,
            depletion: value.depletion,
        }
    }
}

pub fn resource_name(resource: Resource) -> &'static str {
    match resource {
        Resource::Credits => "credits",
        Resource::Water => "water",
        Resource::Food => "food",
        Resource::Electricity => "electricity",
        Resource::Metals => "metals",
        Resource::Oil => "oil",
        Resource::Uranium => "uranium",
    }
}
