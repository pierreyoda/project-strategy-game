//! What buildings cost and make, and the turn that settles it.

use std::collections::BTreeMap;

use crate::game::{
    events::{EventBuffer, GameEvent},
    state::{World, WorldMut},
};

use super::{
    buildings::BuildingProduction,
    ids::{BuildingId, DepositId, NationId},
    resources::{
        Resource, ResourceData, ResourceDataStorage, ResourceDataStore, ResourceDeposit,
        ResourceQuantity,
    },
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
    for building in buildings_of(world.world(), nation) {
        operate(world, nation, building, events);
        advance_works(world, nation, building, events);
    }
}

/// A nation's buildings, in the ID order their turn is settled in.
fn buildings_of(world: &World, nation: NationId) -> Vec<BuildingId> {
    world
        .buildings()
        .iter()
        .filter(|(_, building)| building.owner() == nation)
        .map(|(id, _)| id)
        .collect()
}

/// What a stockpile gains and loses over a turn's economy phase.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Forecast {
    /// Its income, and what buildings make of it.
    pub gained: ResourceQuantity,
    /// Its depletion, and what buildings take of it for their maintenance.
    pub spent: ResourceQuantity,
}

/// What each of a nation's stockpiles gains and loses in its next economy phase, as things stand.
///
/// The phase itself, played on a copy of the stockpile: its income and depletion, then its
/// buildings in the order they are settled in - each one paid for works and makes what its deposit
/// yields; one that cannot be paid, halted or not, makes nothing. Played rather than summed, so
/// that the figure the player is shown is the change the turn makes, shortfalls and all. Every
/// resource the turn leaves a stockpile of is in, at zero if nothing changes it.
pub fn forecast(world: &World, nation: NationId) -> BTreeMap<Resource, Forecast> {
    let Some(payer) = world.nations().get(nation) else {
        return BTreeMap::new();
    };
    let mut stockpile = payer.resources().clone();
    let mut forecast: BTreeMap<Resource, Forecast> = BTreeMap::new();
    stockpile.update_with(&mut |resource, income, depletion| {
        let entry = forecast.entry(resource).or_default();
        entry.gained = entry.gained.saturating_add(income);
        entry.spent = entry.spent.saturating_add(depletion);
    });

    for id in buildings_of(world, nation) {
        let Some(shift) = shift_of(world, id) else {
            continue;
        };
        if shortfall(&stockpile, &shift.maintenance).is_some() {
            continue;
        }
        take(&mut stockpile, &shift.maintenance);
        for (&resource, &amount) in &shift.maintenance {
            let entry = forecast.entry(resource).or_default();
            entry.spent = entry.spent.saturating_add(amount);
        }
        let made = output(&shift, |deposit, wanted| {
            world
                .deposits()
                .get(deposit)
                .and_then(held)
                .map_or(0, |(_, data)| yield_of(data, wanted))
        });
        for (resource, amount) in made {
            if amount > 0 {
                stockpile.replenish(resource, amount);
                let entry = forecast.entry(resource).or_default();
                entry.gained = entry.gained.saturating_add(amount);
            }
        }
    }
    for &resource in stockpile.keys() {
        forecast.entry(resource).or_default();
    }
    forecast
}

/// What a working building's level asks and gives, read before anything is written.
struct Shift {
    maintenance: MaintenanceCosts,
    production: BuildingProduction,
    /// The resource it extracts, and the deposit of it on its tile.
    extracted: Option<(Resource, Option<DepositId>)>,
    halted: bool,
}

fn shift_of(state: &World, id: BuildingId) -> Option<Shift> {
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

/// What a paid building makes: its production, the resource it extracts being what `draw` gets out
/// of the deposit under it - nothing, without one.
fn output(
    shift: &Shift,
    draw: impl FnOnce(DepositId, ResourceQuantity) -> ResourceQuantity,
) -> BuildingProduction {
    let mut made = shift.production.clone();
    if let Some((resource, deposit)) = shift.extracted {
        let wanted = made.get(&resource).copied().unwrap_or(0);
        made.insert(resource, deposit.map_or(0, |deposit| draw(deposit, wanted)));
    }
    made
}

fn operate(world: &mut WorldMut<'_>, nation: NationId, id: BuildingId, events: &mut EventBuffer) {
    let Some(shift) = shift_of(world.world(), id) else {
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

    let made = output(&shift, |deposit, wanted| {
        draw(world, nation, deposit, wanted, events)
    });
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
    let Some((resource, stock, drawn)) = world.world().deposits().get(id).and_then(|deposit| {
        let (resource, data) = held(deposit)?;
        Some((resource, data.quantity(), yield_of(data, wanted)))
    }) else {
        return 0;
    };
    if stock == 0 {
        // nothing changes on a deposit that springs back, or on one long run dry
        return drawn;
    }

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

/// The resource a deposit holds, and how much of it.
fn held(deposit: &ResourceDeposit) -> Option<(Resource, &ResourceData)> {
    let resource = deposit.resource()?;
    Some((resource, deposit.resources().get(&resource)?))
}

/// What a deposit gives of `wanted`: out of its stock while it has one, and of a deposit that
/// springs back - no stock, a recharge - up to its recharge.
fn yield_of(data: &ResourceData, wanted: ResourceQuantity) -> ResourceQuantity {
    match data.quantity() {
        0 => wanted.min(data.income().unwrap_or(0)),
        stock => wanted.min(stock),
    }
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
