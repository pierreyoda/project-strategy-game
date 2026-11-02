//! What buildings cost and make, and the turn that settles it.

use std::collections::BTreeMap;

use crate::game::{
    events::{EventBuffer, GameEvent},
    state::WorldMut,
};

use super::{
    buildings::BuildingProduction,
    ids::{BuildingId, DepositId, NationId},
    resources::{Resource, ResourceDataStorage, ResourceDataStore, ResourceQuantity},
};

// `BTreeMap` rather than `HashMap`: costs are iterated during turn resolution, and a resolution
// whose outcome depends on iteration order is not reproducible from a seed.

/// Construction cost.
pub type ConstructionCosts = BTreeMap<Resource, ResourceQuantity>;

/// Maintenance cost, per turn.
pub type MaintenanceCosts = BTreeMap<Resource, ResourceQuantity>;

/// The first resource a stockpile holds too little of to pay `bill` in full, and how much is
/// missing; `None` when it can pay all of it.
pub fn shortfall(
    stockpile: &ResourceDataStorage,
    bill: &BTreeMap<Resource, ResourceQuantity>,
) -> Option<(Resource, ResourceQuantity)> {
    bill.iter().find_map(|(&resource, &amount)| {
        let held = stockpile.quantity_of(resource);
        (held < amount).then(|| (resource, amount - held))
    })
}

/// Take `bill` out of a stockpile `shortfall` found enough in.
pub fn take(stockpile: &mut ResourceDataStorage, bill: &BTreeMap<Resource, ResourceQuantity>) {
    for (&resource, &amount) in bill {
        stockpile.consume(resource, amount);
    }
}

/// One turn of a nation's buildings, in ID order: each one that works pays its maintenance and
/// makes its production, then its works advance a turn.
///
/// Working first is what makes an upgrading building keep its level until the upgrade is done: the
/// turn its works complete, it was already settled at the level it had. The order is fixed so that
/// a playthrough replays: a building's production can pay for a later one's maintenance in the same
/// turn, and when the stockpile runs short, the earlier one is the one paid.
pub fn run_buildings(world: &mut WorldMut<'_>, nation: NationId, events: &mut EventBuffer) {
    let buildings: Vec<BuildingId> = world
        .world()
        .buildings()
        .iter()
        .filter(|(_, building)| building.owner() == nation)
        .map(|(id, _)| id)
        .collect();
    for building in buildings {
        operate(world, nation, building, events);
        advance_works(world, nation, building, events);
    }
}

/// What a working building's level asks and gives, read before anything is written.
struct Shift {
    maintenance: MaintenanceCosts,
    production: BuildingProduction,
    /// The resource it extracts, and the deposit of it on its tile.
    extracted: Option<(Resource, Option<DepositId>)>,
    halted: bool,
}

fn shift_of(world: &WorldMut<'_>, id: BuildingId) -> Option<Shift> {
    let state = world.world();
    let building = state.buildings().get(id)?;
    // still being built: nothing to pay, nothing to make
    if !building.is_operating() {
        return None;
    }
    let template = state.building_template(building.template())?;
    let level = template.level(building.level())?;
    let extracted = template.deposit().map(|resource| {
        let deposit = state.tile(building.position()).and_then(|tile| {
            tile.natural().deposits().iter().copied().find(|&deposit| {
                state
                    .deposits()
                    .get(deposit)
                    .is_some_and(|deposit| deposit.resource() == Some(resource))
            })
        });
        (resource, deposit)
    });
    Some(Shift {
        maintenance: level.maintenance.clone(),
        production: level.production.clone(),
        extracted,
        halted: building.halted(),
    })
}

fn operate(world: &mut WorldMut<'_>, nation: NationId, id: BuildingId, events: &mut EventBuffer) {
    let Some(shift) = shift_of(world, id) else {
        return;
    };

    // maintenance is all or nothing: a building whose whole bill cannot be paid takes nothing,
    // makes nothing, and halts - told the turn it stops, not every turn it stays stopped
    let short = world
        .world()
        .nations()
        .get(nation)
        .is_none_or(|payer| shortfall(payer.resources(), &shift.maintenance).is_some());
    if short {
        if !shift.halted {
            if let Some(building) = world.building_mut(id) {
                building.set_halted(true);
            }
            events.push(GameEvent::BuildingHalted {
                nation,
                building: id,
            });
        }
        return;
    }
    if shift.halted
        && let Some(building) = world.building_mut(id)
    {
        building.set_halted(false);
    }

    let mut made = shift.production;
    if let Some((resource, deposit)) = shift.extracted {
        let wanted = made.get(&resource).copied().unwrap_or(0);
        let drawn = deposit.map_or(0, |deposit| draw(world, nation, deposit, wanted, events));
        made.insert(resource, drawn);
    }
    if let Some(payer) = world.nation_mut(nation) {
        let stockpile = payer.resources_mut();
        take(stockpile, &shift.maintenance);
        for (resource, amount) in made {
            if amount > 0 {
                stockpile.replenish(resource, amount);
            }
        }
    }
}

/// Take up to `wanted` out of a deposit, and say how much came.
///
/// A deposit that recharges - water - yields up to its recharge every turn and never runs out. One
/// with a stock runs it down, at a rate its `turns_left` is read from, and the turn it runs dry is
/// told once.
fn draw(
    world: &mut WorldMut<'_>,
    nation: NationId,
    id: DepositId,
    wanted: ResourceQuantity,
    events: &mut EventBuffer,
) -> ResourceQuantity {
    let Some((resource, stock, recharge)) = world.world().deposits().get(id).and_then(|deposit| {
        let resource = deposit.resource()?;
        let data = deposit.resources().get(&resource)?;
        Some((resource, data.quantity(), data.income().unwrap_or(0)))
    }) else {
        return 0;
    };
    if stock == 0 {
        // nothing changes on a deposit that springs back, or on one long run dry
        return if recharge > 0 {
            wanted.min(recharge)
        } else {
            0
        };
    }

    let drawn = wanted.min(stock);
    if let Some(deposit) = world.deposit_mut(id) {
        let held = deposit.resources_mut();
        held.consume(resource, drawn);
        held.set_depletion(resource, wanted);
    }
    if drawn == stock {
        events.push(GameEvent::DepositExhausted {
            nation,
            deposit: id,
        });
    }
    drawn
}

fn advance_works(
    world: &mut WorldMut<'_>,
    nation: NationId,
    id: BuildingId,
    events: &mut EventBuffer,
) {
    let under_way = world
        .world()
        .buildings()
        .get(id)
        .is_some_and(|building| building.works().is_some());
    if !under_way {
        return;
    }
    if let Some(building) = world.building_mut(id)
        && building.advance_works()
    {
        events.push(GameEvent::BuildingCompleted {
            nation,
            building: id,
            level: building.level(),
        });
    }
}
